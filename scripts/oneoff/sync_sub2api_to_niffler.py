#!/usr/bin/env python3
"""One-time sub2api to Niffler data sync.

Default mode is a dry run. Pass --apply to write to the Niffler database.
Sensitive values are never printed.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import hmac
import json
import os
import re
import secrets
import struct
import subprocess
import sys
import time
from collections import Counter, defaultdict
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from typing import Any

from cryptography.hazmat.primitives import padding
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes


SOURCE_HOST = "tc-jp"
TARGET_DB_HOST = "rn01"
TARGET_APP_HOST = "hd0526"

SOURCE_PSQL = "docker exec sub2api-postgres-dev psql -U sub2api -d sub2api -At"
TARGET_PSQL_ARGS = [
    "docker",
    "exec",
    "-i",
    "niffler-postgres",
    "psql",
    "-v",
    "ON_ERROR_STOP=1",
    "-U",
    "postgres",
    "-d",
    "aether",
]

CODEX_PROVIDER_ID = "sub2api-provider-codex"
CODEX_ENDPOINT_ID = "sub2api-codex-responses"
CODEX_FORMAT = "openai:responses"
CODEX_BASE_URL = "https://chatgpt.com/backend-api/codex"
CODEX_CUSTOM_PATH = "/responses"
APP_SALT = hashlib.sha256(b"aether-v1").digest()[:16]
PLACEHOLDER_API_KEY = "__placeholder__"


class SyncError(RuntimeError):
    pass


def run(cmd: list[str], *, input_text: str | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        cmd,
        input=input_text,
        text=True,
        capture_output=True,
        check=False,
    )
    if check and proc.returncode != 0:
        stderr = proc.stderr.strip() or proc.stdout.strip()
        raise SyncError(f"命令执行失败：{' '.join(cmd[:3])}\n{stderr}")
    return proc


def ssh(host: str, remote_cmd: str, *, input_text: str | None = None) -> str:
    proc = run(["ssh", host, remote_cmd], input_text=input_text)
    return proc.stdout


def psql_literal(sql: str) -> str:
    return "'" + sql.replace("'", "''") + "'"


def source_json(query: str) -> list[dict[str, Any]]:
    wrapped = f"select coalesce(jsonb_agg(to_jsonb(t)), '[]'::jsonb) from ({query}) t"
    out = ssh(SOURCE_HOST, f"{SOURCE_PSQL} -c {shell_quote(wrapped)}")
    text = out.strip()
    if not text:
        return []
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        raise SyncError(f"源库返回的 JSON 无法解析：{exc}") from exc
    if not isinstance(value, list):
        raise SyncError("源库返回的数据格式不是列表")
    return value


def target_query(query: str) -> str:
    return ssh(
        TARGET_DB_HOST,
        " ".join(TARGET_PSQL_ARGS + ["-At", "-c", shell_quote(query)]),
    )


def target_exec(sql: str) -> str:
    return ssh(TARGET_DB_HOST, " ".join(TARGET_PSQL_ARGS), input_text=sql)


def shell_quote(value: str) -> str:
    return "'" + value.replace("'", "'\"'\"'") + "'"


def load_target_encryption_key() -> str:
    cmd = (
        "python3 - <<'PY'\n"
        "from pathlib import Path\n"
        "env = Path('/opt/niffler-app/.env')\n"
        "values = {}\n"
        "if env.exists():\n"
        "    for line in env.read_text().splitlines():\n"
        "        line = line.strip()\n"
        "        if not line or line.startswith('#') or '=' not in line:\n"
        "            continue\n"
        "        key, value = line.split('=', 1)\n"
        "        values[key.strip()] = value.strip().strip('\"').strip(\"'\")\n"
        "for key in ('AETHER_GATEWAY_DATA_ENCRYPTION_KEY', 'ENCRYPTION_KEY'):\n"
        "    if values.get(key):\n"
        "        print(values[key])\n"
        "        raise SystemExit(0)\n"
        "raise SystemExit(2)\n"
        "PY"
    )
    proc = run(["ssh", TARGET_APP_HOST, cmd], check=False)
    if proc.returncode != 0:
        raise SyncError("没有在 hd0526 的 /opt/niffler-app/.env 找到加密密钥")
    key = proc.stdout.strip()
    if not key:
        raise SyncError("目标服务加密密钥为空")
    return key


def b64_urlsafe_decode(value: str) -> bytes:
    padding_len = (-len(value)) % 4
    return base64.urlsafe_b64decode(value + ("=" * padding_len))


def raw_fernet_key(secret: str) -> bytes:
    try:
        decoded = b64_urlsafe_decode(secret)
        if len(decoded) == 32:
            return decoded
    except Exception:
        pass
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=APP_SALT,
        iterations=100_000,
    )
    return kdf.derive(secret.encode())


def encrypt_fernet(secret: str, plaintext: str) -> str:
    raw_key = raw_fernet_key(secret)
    signing_key = raw_key[:16]
    encryption_key = raw_key[16:]
    iv = secrets.token_bytes(16)
    padder = padding.PKCS7(128).padder()
    padded = padder.update(plaintext.encode()) + padder.finalize()
    encryptor = Cipher(algorithms.AES(encryption_key), modes.CBC(iv)).encryptor()
    ciphertext = encryptor.update(padded) + encryptor.finalize()
    signed = b"\x80" + struct.pack(">Q", int(time.time())) + iv + ciphertext
    signature = hmac.new(signing_key, signed, hashlib.sha256).digest()
    inner = base64.urlsafe_b64encode(signed + signature)
    return base64.urlsafe_b64encode(inner).decode()


def sql_value(value: Any) -> str:
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float, Decimal)):
        return str(value)
    return "'" + str(value).replace("'", "''") + "'"


def sql_text(value: Any) -> str:
    return sql_value("" if value is None else value)


def sql_json(value: Any, *, jsonb: bool = False) -> str:
    data = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    return sql_value(data) + ("::jsonb" if jsonb else "::json")


def sql_ts(value: Any) -> str:
    if value in (None, ""):
        return "NULL"
    return sql_value(value) + "::timestamptz"


def sql_num(value: Any, default: str = "0") -> str:
    if value in (None, ""):
        return default
    try:
        return str(Decimal(str(value)))
    except InvalidOperation:
        return default


def nullable_int(value: Any) -> int | None:
    if value in (None, ""):
        return None
    try:
        number = int(value)
    except (TypeError, ValueError):
        return None
    return number if number > 0 else None


def truthy_active(value: Any) -> bool:
    return str(value or "").lower() == "active"


def username_from_user(user: dict[str, Any]) -> str:
    user_id = user["id"]
    username = str(user.get("username") or "").strip()
    if username:
        return f"sub2api-{user_id}-{username}"
    email = str(user.get("email") or "").strip()
    if email and "@" in email:
        return f"sub2api-{user_id}-{email.split('@', 1)[0]}"
    return f"sub2api-user-{user_id}"


def normalize_group_name(name: str) -> str:
    normalized = re.sub(r"\s+", "-", name.strip().lower())
    normalized = re.sub(r"[^a-z0-9\u4e00-\u9fff_-]+", "-", normalized)
    normalized = normalized.strip("-_") or "sub2api-group"
    return f"sub2api-{normalized}"


def source_user_id(user_id: Any) -> str:
    return f"sub2api-user-{user_id}"


def source_wallet_id(user_id: Any) -> str:
    return f"sub2api-wallet-user-{user_id}"


def source_api_key_id(key_id: Any) -> str:
    return f"sub2api-api-key-{key_id}"


def source_group_id(group_id: Any) -> str:
    return f"sub2api-user-group-{group_id}"


def source_provider_key_id(account_id: Any) -> str:
    return f"sub2api-provider-key-{account_id}"


def source_plan_id(group_id: Any) -> str:
    return f"sub2api-plan-group-{group_id}"


def source_order_id(subscription_id: Any) -> str:
    return f"sub2api-order-sub-{subscription_id}"


def source_entitlement_id(subscription_id: Any) -> str:
    return f"sub2api-entitlement-{subscription_id}"


def hash_api_key(raw_key: str) -> str:
    return hashlib.sha256(raw_key.encode()).hexdigest()


def api_key_prefix(raw_key: str) -> str:
    if len(raw_key) <= 10:
        return raw_key
    return raw_key[:10]


def parse_epoch(value: Any) -> int | None:
    if value in (None, ""):
        return None
    if isinstance(value, (int, float)):
        return int(value)
    text = str(value).strip()
    if not text:
        return None
    if text.isdigit():
        return int(text)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return int(parsed.timestamp())


def fetch_source_data() -> dict[str, list[dict[str, Any]]]:
    users = source_json(
        """
        select
          id,
          email,
          password_hash,
          role,
          balance::text,
          concurrency,
          status,
          created_at,
          updated_at,
          username,
          notes,
          total_recharged::text,
          last_login_at,
          last_active_at,
          rpm_limit
        from users
        where deleted_at is null
        order by id
        """
    )
    api_keys = source_json(
        """
        select
          ak.id,
          ak.user_id,
          ak.key,
          ak.name,
          ak.group_id,
          g.name as group_name,
          g.platform as group_platform,
          g.subscription_type as group_subscription_type,
          g.rpm_limit as group_rpm_limit,
          ak.status,
          ak.created_at,
          ak.updated_at,
          ak.expires_at,
          ak.last_used_at,
          ak.quota::text,
          ak.quota_used::text,
          ak.rate_limit_5h::text,
          ak.rate_limit_1d::text,
          ak.rate_limit_7d::text,
          ak.usage_5h::text,
          ak.usage_1d::text,
          ak.usage_7d::text,
          ak.window_5h_start,
          ak.window_1d_start,
          ak.window_7d_start,
          ak.ip_whitelist,
          ak.ip_blacklist
        from api_keys ak
        left join groups g on g.id = ak.group_id
        where ak.deleted_at is null
          and ak.status = 'active'
        order by ak.id
        """
    )
    groups = source_json(
        """
        select
          id,
          name,
          description,
          rate_multiplier::text,
          is_exclusive,
          status,
          created_at,
          updated_at,
          platform,
          subscription_type,
          daily_limit_usd::text,
          weekly_limit_usd::text,
          monthly_limit_usd::text,
          five_hour_limit_usd::text,
          default_validity_days,
          rpm_limit,
          sort_order,
          model_routing,
          model_routing_enabled,
          require_oauth_only,
          require_privacy_set
        from groups
        where deleted_at is null
          and status = 'active'
          and platform = 'openai'
        order by sort_order, id
        """
    )
    user_allowed_groups = source_json(
        """
        select user_id, group_id
        from user_allowed_groups
        order by user_id, group_id
        """
    )
    subscriptions = source_json(
        """
        select
          us.id,
          us.user_id,
          u.email as user_email,
          us.group_id,
          g.name as group_name,
          g.subscription_type,
          g.daily_limit_usd::text,
          g.weekly_limit_usd::text,
          g.monthly_limit_usd::text,
          g.five_hour_limit_usd::text,
          g.rpm_limit as group_rpm_limit,
          g.default_validity_days,
          us.status,
          us.starts_at,
          us.expires_at,
          us.daily_window_start,
          us.weekly_window_start,
          us.monthly_window_start,
          us.five_hour_window_start,
          us.daily_usage_usd::text,
          us.weekly_usage_usd::text,
          us.monthly_usage_usd::text,
          us.five_hour_usage_usd::text,
          us.assigned_at,
          us.notes,
          us.created_at,
          us.updated_at
        from user_subscriptions us
        join users u on u.id = us.user_id
        join groups g on g.id = us.group_id
        where us.deleted_at is null
          and us.status = 'active'
          and us.expires_at > now()
          and u.deleted_at is null
        order by us.id
        """
    )
    codex_accounts = source_json(
        """
        select distinct on (a.id)
          a.id,
          a.name,
          a.platform,
          a.type,
          a.credentials,
          a.extra,
          a.concurrency,
          a.priority,
          a.status,
          a.error_message,
          a.last_used_at,
          a.created_at,
          a.updated_at,
          a.schedulable,
          a.rate_limited_at,
          a.rate_limit_reset_at,
          a.overload_until,
          a.temp_unschedulable_until,
          a.temp_unschedulable_reason,
          a.notes,
          a.expires_at,
          a.rate_multiplier::text,
          a.load_factor
        from accounts a
        join account_groups ag on ag.account_id = a.id
        join groups g on g.id = ag.group_id
        where a.deleted_at is null
          and a.platform = 'openai'
          and a.type = 'oauth'
          and g.name = 'Codex'
        order by a.id
        """
    )
    return {
        "users": users,
        "api_keys": api_keys,
        "groups": groups,
        "user_allowed_groups": user_allowed_groups,
        "subscriptions": subscriptions,
        "codex_accounts": codex_accounts,
    }


def target_counts() -> dict[str, int]:
    tables = [
        "users",
        "wallets",
        "api_keys",
        "providers",
        "provider_api_keys",
        "provider_endpoints",
        "billing_plans",
        "payment_orders",
        "user_plan_entitlements",
        "user_groups",
        "api_key_provider_mappings",
        "user_group_members",
    ]
    selects = " union all ".join(
        f"select {sql_value(table)} as table_name, count(*)::bigint from {table}" for table in tables
    )
    out = target_query(selects)
    counts: dict[str, int] = {}
    for line in out.splitlines():
        if not line.strip():
            continue
        table, count = line.split("|", 1)
        counts[table] = int(count)
    return counts


def target_import_counts() -> dict[str, int]:
    checks = {
        "imported_users": "select count(*) from users where id like 'sub2api-user-%'",
        "imported_wallets": "select count(*) from wallets where id like 'sub2api-wallet-%'",
        "imported_api_keys": "select count(*) from api_keys where id like 'sub2api-api-key-%'",
        "imported_providers": "select count(*) from providers where id = 'sub2api-provider-codex'",
        "imported_provider_api_keys": "select count(*) from provider_api_keys where id like 'sub2api-provider-key-%'",
        "imported_plans": "select count(*) from billing_plans where id like 'sub2api-plan-%'",
        "imported_entitlements": "select count(*) from user_plan_entitlements where id like 'sub2api-entitlement-%'",
    }
    query = " union all ".join(
        f"select {sql_value(name)} as name, ({sql})::bigint as count" for name, sql in checks.items()
    )
    out = target_query(query)
    counts: dict[str, int] = {}
    for line in out.splitlines():
        if not line.strip():
            continue
        name, count = line.split("|", 1)
        counts[name] = int(count)
    return counts


def assert_safe_target(counts: dict[str, int], import_counts: dict[str, int], *, force: bool) -> None:
    if any(value > 0 for value in import_counts.values()):
        if not force:
            raise SyncError("目标库里已经有 sub2api 导入数据；如需覆盖更新，请加 --force")
        return

    dirty_tables = {
        "api_keys": counts.get("api_keys", 0),
        "providers": counts.get("providers", 0),
        "provider_api_keys": counts.get("provider_api_keys", 0),
        "provider_endpoints": counts.get("provider_endpoints", 0),
        "billing_plans": counts.get("billing_plans", 0),
        "payment_orders": counts.get("payment_orders", 0),
        "user_plan_entitlements": counts.get("user_plan_entitlements", 0),
        "api_key_provider_mappings": counts.get("api_key_provider_mappings", 0),
    }
    not_empty = {table: count for table, count in dirty_tables.items() if count}
    if not_empty and not force:
        raise SyncError(f"目标库已有业务数据，默认停止：{not_empty}；确认要写请加 --force")


def build_group_entitlements(group: dict[str, Any]) -> dict[str, Any]:
    return {
        "source": "sub2api",
        "source_group_id": group["id"],
        "source_group_name": group.get("name"),
        "subscription_type": group.get("subscription_type"),
        "limits": {
            "daily_limit_usd": group.get("daily_limit_usd"),
            "weekly_limit_usd": group.get("weekly_limit_usd"),
            "monthly_limit_usd": group.get("monthly_limit_usd"),
            "five_hour_limit_usd": group.get("five_hour_limit_usd"),
            "rpm_limit": group.get("rpm_limit"),
        },
        "rate_multiplier": group.get("rate_multiplier"),
    }


def subscription_groups(data: dict[str, list[dict[str, Any]]]) -> list[dict[str, Any]]:
    groups_by_id = {group["id"]: group for group in data["groups"]}
    result: dict[Any, dict[str, Any]] = {}
    for group in data["groups"]:
        if group.get("subscription_type") == "subscription":
            result[group["id"]] = group
    for sub in data["subscriptions"]:
        group = groups_by_id.get(sub["group_id"])
        if group:
            result[group["id"]] = group
    return list(result.values())


def build_auth_config(account: dict[str, Any]) -> dict[str, Any]:
    credentials = account.get("credentials") or {}
    extra = account.get("extra") or {}
    if not isinstance(credentials, dict):
        credentials = {}
    if not isinstance(extra, dict):
        extra = {}

    auth_config: dict[str, Any] = {
        "provider_type": "codex",
        "token_type": credentials.get("token_type") or "Bearer",
        "updated_at": int(time.time()),
    }
    for key in [
        "access_token",
        "refresh_token",
        "id_token",
        "client_id",
        "scope",
        "email",
        "organization_id",
    ]:
        if credentials.get(key):
            auth_config[key] = credentials[key]

    expires_at = parse_epoch(credentials.get("expires_at") or credentials.get("expiresAt"))
    if expires_at:
        auth_config["expires_at"] = expires_at

    account_id = (
        credentials.get("account_id")
        or credentials.get("chatgpt_account_id")
        or extra.get("account_id")
        or extra.get("chatgpt_account_id")
    )
    account_user_id = (
        credentials.get("account_user_id")
        or credentials.get("chatgpt_user_id")
        or extra.get("account_user_id")
        or extra.get("chatgpt_user_id")
    )
    if account_id:
        auth_config["account_id"] = account_id
        auth_config["chatgpt_account_id"] = account_id
    if account_user_id:
        auth_config["account_user_id"] = account_user_id
        auth_config["chatgpt_user_id"] = account_user_id

    plan_type = credentials.get("plan_type") or extra.get("plan_type")
    if plan_type:
        auth_config["plan_type"] = plan_type

    if credentials.get("model_mapping"):
        auth_config["model_mapping"] = credentials["model_mapping"]
    return auth_config


def codex_status_snapshot(account: dict[str, Any]) -> dict[str, Any]:
    credentials = account.get("credentials") or {}
    extra = account.get("extra") or {}
    if not isinstance(credentials, dict):
        credentials = {}
    if not isinstance(extra, dict):
        extra = {}
    return {
        "source": "sub2api",
        "quota": {
            "plan_type": credentials.get("plan_type") or extra.get("plan_type"),
            "primary_used_percent": extra.get("codex_5h_used_percent"),
            "secondary_used_percent": extra.get("codex_7d_used_percent"),
            "primary_reset_at": extra.get("codex_5h_reset_at"),
            "secondary_reset_at": extra.get("codex_7d_reset_at"),
        },
        "source_status": account.get("status"),
        "source_schedulable": account.get("schedulable"),
        "source_rate_limited_at": account.get("rate_limited_at"),
        "source_overload_until": account.get("overload_until"),
    }


def provider_key_metadata(account: dict[str, Any]) -> dict[str, Any]:
    credentials = account.get("credentials") or {}
    extra = account.get("extra") or {}
    if not isinstance(credentials, dict):
        credentials = {}
    if not isinstance(extra, dict):
        extra = {}
    return {
        "source": "sub2api",
        "source_account_id": account.get("id"),
        "source_name": account.get("name"),
        "plan_type": credentials.get("plan_type") or extra.get("plan_type"),
        "email": credentials.get("email") or extra.get("email"),
        "privacy_mode": extra.get("privacy_mode"),
        "rate_multiplier": account.get("rate_multiplier"),
        "load_factor": account.get("load_factor"),
    }


def generate_sql(data: dict[str, list[dict[str, Any]]], encryption_key: str) -> str:
    users_by_id = {user["id"]: user for user in data["users"]}
    groups_by_id = {group["id"]: group for group in data["groups"]}
    api_keys_by_user: defaultdict[Any, list[dict[str, Any]]] = defaultdict(list)
    group_members: set[tuple[str, str]] = set()
    for key in data["api_keys"]:
        api_keys_by_user[key["user_id"]].append(key)
        if key.get("group_id") in groups_by_id:
            group_members.add((source_group_id(key["group_id"]), source_user_id(key["user_id"])))
    for item in data["user_allowed_groups"]:
        if item.get("group_id") in groups_by_id and item.get("user_id") in users_by_id:
            group_members.add((source_group_id(item["group_id"]), source_user_id(item["user_id"])))
    for sub in data["subscriptions"]:
        if sub.get("group_id") in groups_by_id and sub.get("user_id") in users_by_id:
            group_members.add((source_group_id(sub["group_id"]), source_user_id(sub["user_id"])))

    lines: list[str] = [
        "BEGIN;",
        "SET LOCAL lock_timeout = '5s';",
        "SET LOCAL statement_timeout = '120s';",
    ]

    for group in data["groups"]:
        group_rate = nullable_int(group.get("rpm_limit"))
        lines.append(
            """
INSERT INTO user_groups (
  id, name, normalized_name, description, priority,
  allowed_providers, allowed_providers_mode,
  allowed_api_formats, allowed_api_formats_mode,
  allowed_models, allowed_models_mode,
  rate_limit, rate_limit_mode, concurrent_limit, concurrent_limit_mode,
  created_at, updated_at
) VALUES (
  {id}, {name}, {normalized_name}, {description}, {priority},
  {providers}, 'specific',
  {formats}, 'specific',
  NULL, 'inherit',
  {rate_limit}, {rate_limit_mode}, NULL, 'inherit',
  {created_at}, {updated_at}
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  normalized_name = EXCLUDED.normalized_name,
  description = EXCLUDED.description,
  priority = EXCLUDED.priority,
  allowed_providers = EXCLUDED.allowed_providers,
  allowed_providers_mode = EXCLUDED.allowed_providers_mode,
  allowed_api_formats = EXCLUDED.allowed_api_formats,
  allowed_api_formats_mode = EXCLUDED.allowed_api_formats_mode,
  rate_limit = EXCLUDED.rate_limit,
  rate_limit_mode = EXCLUDED.rate_limit_mode,
  updated_at = EXCLUDED.updated_at;
""".format(
                id=sql_value(source_group_id(group["id"])),
                name=sql_value(group.get("name")),
                normalized_name=sql_value(normalize_group_name(str(group.get("name") or group["id"]))),
                description=sql_value(group.get("description") or f"从 sub2api 分组 {group.get('name')} 同步"),
                priority=int(group.get("sort_order") or 0),
                providers=sql_json([CODEX_PROVIDER_ID]),
                formats=sql_json([CODEX_FORMAT]),
                rate_limit=sql_value(group_rate),
                rate_limit_mode=sql_value("custom" if group_rate else "inherit"),
                created_at=sql_ts(group.get("created_at")),
                updated_at=sql_ts(group.get("updated_at")),
            ).strip()
        )

    for user in data["users"]:
        is_active = truthy_active(user.get("status"))
        role = "admin" if user.get("role") == "admin" else "user"
        rate_limit = nullable_int(user.get("rpm_limit"))
        metadata = {
            "source": "sub2api",
            "source_user_id": user.get("id"),
            "notes": user.get("notes"),
            "last_active_at": user.get("last_active_at"),
        }
        lines.append(
            """
INSERT INTO users (
  id, external_id, email, username, password_hash, role,
  allowed_providers, allowed_providers_mode,
  allowed_api_formats, allowed_api_formats_mode,
  allowed_models, allowed_models_mode,
  is_active, is_deleted, created_at, updated_at, last_login_at,
  auth_source, email_verified, rate_limit, rate_limit_mode, metadata
) VALUES (
  {id}, {external_id}, {email}, {username}, {password_hash}, {role}::userrole,
  NULL, 'unrestricted',
  NULL, 'unrestricted',
  NULL, 'unrestricted',
  {is_active}, false, {created_at}, {updated_at}, {last_login_at},
  'local'::authsource, true, {rate_limit}, {rate_limit_mode}, {metadata}
)
ON CONFLICT (id) DO UPDATE SET
  email = EXCLUDED.email,
  username = EXCLUDED.username,
  password_hash = EXCLUDED.password_hash,
  role = EXCLUDED.role,
  allowed_providers = EXCLUDED.allowed_providers,
  allowed_providers_mode = EXCLUDED.allowed_providers_mode,
  allowed_api_formats = EXCLUDED.allowed_api_formats,
  allowed_api_formats_mode = EXCLUDED.allowed_api_formats_mode,
  is_active = EXCLUDED.is_active,
  is_deleted = false,
  updated_at = EXCLUDED.updated_at,
  last_login_at = EXCLUDED.last_login_at,
  rate_limit = EXCLUDED.rate_limit,
  rate_limit_mode = EXCLUDED.rate_limit_mode,
  metadata = EXCLUDED.metadata;
""".format(
                id=sql_value(source_user_id(user["id"])),
                external_id=sql_value(f"sub2api:{user['id']}"),
                email=sql_value(user.get("email")),
                username=sql_value(username_from_user(user)),
                password_hash=sql_value(user.get("password_hash")),
                role=sql_value(role),
                is_active=sql_value(is_active),
                created_at=sql_ts(user.get("created_at")),
                updated_at=sql_ts(user.get("updated_at")),
                last_login_at=sql_ts(user.get("last_login_at")),
                rate_limit=sql_value(rate_limit),
                rate_limit_mode=sql_value("custom" if rate_limit else "system"),
                metadata=sql_json(metadata),
            ).strip()
        )
        lines.append(
            """
INSERT INTO wallets (
  id, user_id, api_key_id, balance, gift_balance, limit_mode, currency,
  status, total_recharged, total_consumed, total_refunded, total_adjusted,
  created_at, updated_at
) VALUES (
  {id}, {user_id}, NULL, {balance}, 0, 'finite', 'USD',
  'active', {total_recharged}, 0, 0, 0,
  {created_at}, {updated_at}
)
ON CONFLICT (id) DO UPDATE SET
  user_id = EXCLUDED.user_id,
  balance = EXCLUDED.balance,
  total_recharged = EXCLUDED.total_recharged,
  updated_at = EXCLUDED.updated_at;
""".format(
                id=sql_value(source_wallet_id(user["id"])),
                user_id=sql_value(source_user_id(user["id"])),
                balance=sql_num(user.get("balance")),
                total_recharged=sql_num(user.get("total_recharged")),
                created_at=sql_ts(user.get("created_at")),
                updated_at=sql_ts(user.get("updated_at")),
            ).strip()
        )

    lines.append(
        """
INSERT INTO providers (
  id, name, description, website, billing_type, monthly_used_usd,
  enabled, priority, provider_priority, is_active, concurrent_limit,
  config, created_at, updated_at, max_retries, request_timeout,
  stream_first_byte_timeout, keep_priority_on_conversion,
  enable_format_conversion, provider_type
) VALUES (
  {id}, 'Codex', '从 sub2api 同步的 Codex OAuth 账号池',
  'https://chatgpt.com', 'pay_as_you_go'::providerbillingtype, 0,
  true, 0, 0, true, NULL,
  {config}, now(), now(), 2, 300, 30, false, true, 'codex'
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  website = EXCLUDED.website,
  billing_type = EXCLUDED.billing_type,
  enabled = true,
  is_active = true,
  config = EXCLUDED.config,
  updated_at = now(),
  max_retries = EXCLUDED.max_retries,
  request_timeout = EXCLUDED.request_timeout,
  stream_first_byte_timeout = EXCLUDED.stream_first_byte_timeout,
  provider_type = EXCLUDED.provider_type;
""".format(
            id=sql_value(CODEX_PROVIDER_ID),
            config=sql_json({"source": "sub2api", "synced_at": datetime.now(timezone.utc).isoformat()}),
        ).strip()
    )

    lines.append(
        """
INSERT INTO provider_endpoints (
  id, provider_id, name, api_format, base_url, max_retries,
  enabled, is_active, weight, custom_path, config, created_at,
  updated_at, api_family, endpoint_kind, health_score, metadata
) VALUES (
  {id}, {provider_id}, 'Codex Responses', {api_format}, {base_url}, 2,
  true, true, 1, {custom_path}, {config}, now(),
  now(), 'openai', 'responses', 1.0, {metadata}
)
ON CONFLICT (id) DO UPDATE SET
  provider_id = EXCLUDED.provider_id,
  name = EXCLUDED.name,
  api_format = EXCLUDED.api_format,
  base_url = EXCLUDED.base_url,
  max_retries = EXCLUDED.max_retries,
  enabled = true,
  is_active = true,
  custom_path = EXCLUDED.custom_path,
  config = EXCLUDED.config,
  updated_at = now(),
  api_family = EXCLUDED.api_family,
  endpoint_kind = EXCLUDED.endpoint_kind,
  metadata = EXCLUDED.metadata;
""".format(
            id=sql_value(CODEX_ENDPOINT_ID),
            provider_id=sql_value(CODEX_PROVIDER_ID),
            api_format=sql_value(CODEX_FORMAT),
            base_url=sql_value(CODEX_BASE_URL),
            custom_path=sql_value(CODEX_CUSTOM_PATH),
            config=sql_json({}),
            metadata=sql_json({"source": "sub2api"}),
        ).strip()
    )

    encrypted_placeholder = encrypt_fernet(encryption_key, PLACEHOLDER_API_KEY)
    for account in data["codex_accounts"]:
        account_active = truthy_active(account.get("status")) and bool(account.get("schedulable"))
        account_status = str(account.get("status") or "inactive")
        auth_config = build_auth_config(account)
        encrypted_auth_config = encrypt_fernet(
            encryption_key,
            json.dumps(auth_config, ensure_ascii=False, separators=(",", ":")),
        )
        metadata = provider_key_metadata(account)
        status_snapshot = codex_status_snapshot(account)
        credentials = account.get("credentials") or {}
        display_email = credentials.get("email") if isinstance(credentials, dict) else None
        name = account.get("name") or display_email or f"sub2api account {account['id']}"
        lines.append(
            """
INSERT INTO provider_api_keys (
  id, api_key, encrypted_key, name, note, internal_priority, rpm_limit,
  concurrent_limit, allowed_models, capabilities, request_count,
  success_count, error_count, total_response_time_ms, is_active,
  expires_at, created_at, updated_at, provider_id, api_formats,
  auth_type_by_format, auth_type, auth_config, upstream_metadata,
  total_tokens, total_cost_usd, status_snapshot, status, weight, metadata
) VALUES (
  {id}, {api_key}, NULL, {name}, {note}, {priority}, NULL,
  {concurrency}, NULL, {capabilities}, 0,
  0, 0, 0, {is_active},
  {expires_at}, {created_at}, {updated_at}, {provider_id}, {api_formats},
  {auth_type_by_format}, 'oauth', {auth_config}, {upstream_metadata},
  0, 0, {status_snapshot}, {status}, 1, {metadata}
)
ON CONFLICT (id) DO UPDATE SET
  api_key = EXCLUDED.api_key,
  name = EXCLUDED.name,
  note = EXCLUDED.note,
  internal_priority = EXCLUDED.internal_priority,
  concurrent_limit = EXCLUDED.concurrent_limit,
  is_active = EXCLUDED.is_active,
  expires_at = EXCLUDED.expires_at,
  updated_at = EXCLUDED.updated_at,
  provider_id = EXCLUDED.provider_id,
  api_formats = EXCLUDED.api_formats,
  auth_type_by_format = EXCLUDED.auth_type_by_format,
  auth_type = EXCLUDED.auth_type,
  auth_config = EXCLUDED.auth_config,
  upstream_metadata = EXCLUDED.upstream_metadata,
  status_snapshot = EXCLUDED.status_snapshot,
  status = EXCLUDED.status,
  metadata = EXCLUDED.metadata;
""".format(
                id=sql_value(source_provider_key_id(account["id"])),
                api_key=sql_value(encrypted_placeholder),
                name=sql_value(str(name)[:255]),
                note=sql_value(account.get("notes") or account.get("error_message")),
                priority=int(account.get("priority") or 50),
                concurrency=sql_value(nullable_int(account.get("concurrency"))),
                capabilities=sql_json({"codex": True, "source": "sub2api"}),
                is_active=sql_value(account_active),
                expires_at=sql_ts(account.get("expires_at")),
                created_at=sql_ts(account.get("created_at")),
                updated_at=sql_ts(account.get("updated_at")),
                provider_id=sql_value(CODEX_PROVIDER_ID),
                api_formats=sql_json([CODEX_FORMAT]),
                auth_type_by_format=sql_json({CODEX_FORMAT: "oauth"}),
                auth_config=sql_value(encrypted_auth_config),
                upstream_metadata=sql_json(metadata, jsonb=True),
                status_snapshot=sql_json(status_snapshot),
                status=sql_value(account_status),
                metadata=sql_json({"source": "sub2api", "source_account_id": account.get("id")}),
            ).strip()
        )

    for group in subscription_groups(data):
        duration_value = int(group.get("default_validity_days") or 0)
        if duration_value <= 0:
            duration_value = 1 if "日" in str(group.get("name") or "") else 30
        entitlements = build_group_entitlements(group)
        lines.append(
            """
INSERT INTO billing_plans (
  id, title, description, price_amount, price_currency,
  duration_unit, duration_value, enabled, sort_order,
  max_active_per_user, purchase_limit_scope, entitlements_json,
  created_at, updated_at
) VALUES (
  {id}, {title}, {description}, 0, 'CNY',
  'day', {duration_value}, true, {sort_order},
  1, 'active_period', {entitlements},
  now(), now()
)
ON CONFLICT (id) DO UPDATE SET
  title = EXCLUDED.title,
  description = EXCLUDED.description,
  duration_value = EXCLUDED.duration_value,
  enabled = true,
  sort_order = EXCLUDED.sort_order,
  entitlements_json = EXCLUDED.entitlements_json,
  updated_at = now();
""".format(
                id=sql_value(source_plan_id(group["id"])),
                title=sql_value(group.get("name")),
                description=sql_value(group.get("description") or f"从 sub2api 分组 {group.get('name')} 同步"),
                duration_value=duration_value,
                sort_order=int(group.get("sort_order") or 0),
                entitlements=sql_json(entitlements, jsonb=True),
            ).strip()
        )

    for key in data["api_keys"]:
        raw_key = key.get("key") or ""
        user = users_by_id.get(key["user_id"])
        group = groups_by_id.get(key.get("group_id"))
        effective_rpm = nullable_int(key.get("group_rpm_limit")) or (nullable_int(user.get("rpm_limit")) if user else None)
        concurrency = nullable_int(user.get("concurrency")) if user else None
        metadata = {
            "source": "sub2api",
            "source_api_key_id": key.get("id"),
            "source_user_id": key.get("user_id"),
            "source_group_id": key.get("group_id"),
            "source_group_name": key.get("group_name"),
            "quota": key.get("quota"),
            "quota_used": key.get("quota_used"),
            "windows": {
                "rate_limit_5h": key.get("rate_limit_5h"),
                "rate_limit_1d": key.get("rate_limit_1d"),
                "rate_limit_7d": key.get("rate_limit_7d"),
                "usage_5h": key.get("usage_5h"),
                "usage_1d": key.get("usage_1d"),
                "usage_7d": key.get("usage_7d"),
                "window_5h_start": key.get("window_5h_start"),
                "window_1d_start": key.get("window_1d_start"),
                "window_7d_start": key.get("window_7d_start"),
            },
            "ip_whitelist": key.get("ip_whitelist"),
            "ip_blacklist": key.get("ip_blacklist"),
        }
        feature_settings = {
            "source_group": {
                "name": group.get("name") if group else key.get("group_name"),
                "subscription_type": group.get("subscription_type") if group else key.get("group_subscription_type"),
                "rate_multiplier": group.get("rate_multiplier") if group else None,
            }
        }
        lines.append(
            """
INSERT INTO api_keys (
  id, user_id, key_hash, key_encrypted, name, key_prefix, status,
  total_requests, total_tokens, total_cost_usd, is_standalone,
  allowed_providers, allowed_api_formats, allowed_models,
  rate_limit, concurrent_limit, force_capabilities, feature_settings,
  is_active, last_used_at, expires_at, auto_delete_on_expiry,
  metadata, created_at, updated_at, is_locked
) VALUES (
  {id}, {user_id}, {key_hash}, {key_encrypted}, {name}, {key_prefix}, 'active',
  0, 0, 0, false,
  NULL, NULL, NULL,
  {rate_limit}, {concurrent_limit}, NULL, {feature_settings},
  true, {last_used_at}, {expires_at}, false,
  {metadata}, {created_at}, {updated_at}, false
)
ON CONFLICT (id) DO UPDATE SET
  user_id = EXCLUDED.user_id,
  key_hash = EXCLUDED.key_hash,
  key_encrypted = EXCLUDED.key_encrypted,
  name = EXCLUDED.name,
  key_prefix = EXCLUDED.key_prefix,
  status = 'active',
  allowed_providers = EXCLUDED.allowed_providers,
  allowed_api_formats = EXCLUDED.allowed_api_formats,
  rate_limit = EXCLUDED.rate_limit,
  concurrent_limit = EXCLUDED.concurrent_limit,
  feature_settings = EXCLUDED.feature_settings,
  is_active = true,
  last_used_at = EXCLUDED.last_used_at,
  expires_at = EXCLUDED.expires_at,
  metadata = EXCLUDED.metadata,
  updated_at = EXCLUDED.updated_at,
  is_locked = false;
""".format(
                id=sql_value(source_api_key_id(key["id"])),
                user_id=sql_value(source_user_id(key["user_id"])),
                key_hash=sql_value(hash_api_key(raw_key)),
                key_encrypted=sql_value(encrypt_fernet(encryption_key, raw_key)),
                name=sql_value(key.get("name") or "sub2api API Key"),
                key_prefix=sql_value(api_key_prefix(raw_key)),
                rate_limit=sql_value(effective_rpm),
                concurrent_limit=sql_value(concurrency),
                feature_settings=sql_json(feature_settings, jsonb=True),
                last_used_at=sql_ts(key.get("last_used_at")),
                expires_at=sql_ts(key.get("expires_at")),
                metadata=sql_json(metadata),
                created_at=sql_ts(key.get("created_at")),
                updated_at=sql_ts(key.get("updated_at")),
            ).strip()
        )
    for sub in data["subscriptions"]:
        user = users_by_id.get(sub["user_id"])
        if not user:
            continue
        group = groups_by_id.get(sub["group_id"])
        if not group:
            continue
        entitlements = build_group_entitlements(group)
        entitlements["source_subscription_id"] = sub.get("id")
        entitlements["usage"] = {
            "daily_usage_usd": sub.get("daily_usage_usd"),
            "weekly_usage_usd": sub.get("weekly_usage_usd"),
            "monthly_usage_usd": sub.get("monthly_usage_usd"),
            "five_hour_usage_usd": sub.get("five_hour_usage_usd"),
            "daily_window_start": sub.get("daily_window_start"),
            "weekly_window_start": sub.get("weekly_window_start"),
            "monthly_window_start": sub.get("monthly_window_start"),
            "five_hour_window_start": sub.get("five_hour_window_start"),
        }
        product_snapshot = {
            "source": "sub2api",
            "source_subscription_id": sub.get("id"),
            "plan_id": source_plan_id(group["id"]),
            "title": group.get("name"),
            "entitlements": entitlements,
        }
        lines.append(
            """
INSERT INTO payment_orders (
  id, order_no, wallet_id, user_id, amount_usd,
  pay_amount, pay_currency, exchange_rate, refunded_amount_usd,
  refundable_amount_usd, payment_method, payment_provider,
  payment_channel, order_kind, product_id, product_snapshot,
  fulfillment_status, gateway_response, status, created_at,
  paid_at, credited_at, expires_at
) VALUES (
  {id}, {order_no}, {wallet_id}, {user_id}, 0,
  0, 'CNY', 1, 0,
  0, 'migration', 'sub2api',
  'migration', 'plan_purchase', {product_id}, {product_snapshot},
  'fulfilled', {gateway_response}, 'paid', {created_at},
  {paid_at}, {credited_at}, {expires_at}
)
ON CONFLICT (id) DO UPDATE SET
  wallet_id = EXCLUDED.wallet_id,
  user_id = EXCLUDED.user_id,
  product_id = EXCLUDED.product_id,
  product_snapshot = EXCLUDED.product_snapshot,
  fulfillment_status = 'fulfilled',
  status = 'paid',
  paid_at = EXCLUDED.paid_at,
  credited_at = EXCLUDED.credited_at,
  expires_at = EXCLUDED.expires_at;
""".format(
                id=sql_value(source_order_id(sub["id"])),
                order_no=sql_value(source_order_id(sub["id"])),
                wallet_id=sql_value(source_wallet_id(sub["user_id"])),
                user_id=sql_value(source_user_id(sub["user_id"])),
                product_id=sql_value(source_plan_id(group["id"])),
                product_snapshot=sql_json(product_snapshot, jsonb=True),
                gateway_response=sql_json({"source": "sub2api", "imported": True}, jsonb=True),
                created_at=sql_ts(sub.get("created_at") or sub.get("assigned_at")),
                paid_at=sql_ts(sub.get("assigned_at") or sub.get("created_at")),
                credited_at=sql_ts(sub.get("assigned_at") or sub.get("created_at")),
                expires_at=sql_ts(sub.get("expires_at")),
            ).strip()
        )
        lines.append(
            """
INSERT INTO user_plan_entitlements (
  id, user_id, plan_id, payment_order_id, status,
  starts_at, expires_at, entitlements_snapshot,
  created_at, updated_at
) VALUES (
  {id}, {user_id}, {plan_id}, {payment_order_id}, {status},
  {starts_at}, {expires_at}, {entitlements_snapshot},
  {created_at}, {updated_at}
)
ON CONFLICT (id) DO UPDATE SET
  user_id = EXCLUDED.user_id,
  plan_id = EXCLUDED.plan_id,
  payment_order_id = EXCLUDED.payment_order_id,
  status = EXCLUDED.status,
  starts_at = EXCLUDED.starts_at,
  expires_at = EXCLUDED.expires_at,
  entitlements_snapshot = EXCLUDED.entitlements_snapshot,
  updated_at = EXCLUDED.updated_at;
""".format(
                id=sql_value(source_entitlement_id(sub["id"])),
                user_id=sql_value(source_user_id(sub["user_id"])),
                plan_id=sql_value(source_plan_id(group["id"])),
                payment_order_id=sql_value(source_order_id(sub["id"])),
                status=sql_value(sub.get("status") or "active"),
                starts_at=sql_ts(sub.get("starts_at")),
                expires_at=sql_ts(sub.get("expires_at")),
                entitlements_snapshot=sql_json(entitlements, jsonb=True),
                created_at=sql_ts(sub.get("created_at") or sub.get("assigned_at")),
                updated_at=sql_ts(sub.get("updated_at") or sub.get("assigned_at")),
            ).strip()
        )

    for group_id, user_id in sorted(group_members):
        lines.append(
            """
INSERT INTO user_group_members (group_id, user_id, created_at)
VALUES ({group_id}, {user_id}, now())
ON CONFLICT (group_id, user_id) DO NOTHING;
""".format(group_id=sql_value(group_id), user_id=sql_value(user_id)).strip()
        )

    lines.append("COMMIT;")
    return "\n".join(lines) + "\n"


def generate_codex_only_sql(data: dict[str, list[dict[str, Any]]], encryption_key: str) -> str:
    lines: list[str] = [
        "BEGIN;",
        "SET LOCAL lock_timeout = '5s';",
        "SET LOCAL statement_timeout = '120s';",
    ]

    lines.append(
        """
INSERT INTO providers (
  id, name, description, website, billing_type, monthly_used_usd,
  enabled, priority, provider_priority, is_active, concurrent_limit,
  config, created_at, updated_at, max_retries, request_timeout,
  stream_first_byte_timeout, keep_priority_on_conversion,
  enable_format_conversion, provider_type
) VALUES (
  {id}, 'Codex', '从 sub2api 同步的 Codex OAuth 账号池',
  'https://chatgpt.com', 'pay_as_you_go'::providerbillingtype, 0,
  true, 0, 0, true, NULL,
  {config}, now(), now(), 2, 300, 30, false, true, 'codex'
)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  description = EXCLUDED.description,
  website = EXCLUDED.website,
  billing_type = EXCLUDED.billing_type,
  enabled = true,
  is_active = true,
  config = EXCLUDED.config,
  updated_at = now(),
  max_retries = EXCLUDED.max_retries,
  request_timeout = EXCLUDED.request_timeout,
  stream_first_byte_timeout = EXCLUDED.stream_first_byte_timeout,
  provider_type = EXCLUDED.provider_type;
""".format(
            id=sql_value(CODEX_PROVIDER_ID),
            config=sql_json({"source": "sub2api", "synced_at": datetime.now(timezone.utc).isoformat()}),
        ).strip()
    )

    lines.append(
        """
INSERT INTO provider_endpoints (
  id, provider_id, name, api_format, base_url, max_retries,
  enabled, is_active, weight, custom_path, config, created_at,
  updated_at, api_family, endpoint_kind, health_score, metadata
) VALUES (
  {id}, {provider_id}, 'Codex Responses', {api_format}, {base_url}, 2,
  true, true, 1, {custom_path}, {config}, now(),
  now(), 'openai', 'responses', 1.0, {metadata}
)
ON CONFLICT (id) DO UPDATE SET
  provider_id = EXCLUDED.provider_id,
  name = EXCLUDED.name,
  api_format = EXCLUDED.api_format,
  base_url = EXCLUDED.base_url,
  max_retries = EXCLUDED.max_retries,
  enabled = true,
  is_active = true,
  custom_path = EXCLUDED.custom_path,
  config = EXCLUDED.config,
  updated_at = now(),
  api_family = EXCLUDED.api_family,
  endpoint_kind = EXCLUDED.endpoint_kind,
  metadata = EXCLUDED.metadata;
""".format(
            id=sql_value(CODEX_ENDPOINT_ID),
            provider_id=sql_value(CODEX_PROVIDER_ID),
            api_format=sql_value(CODEX_FORMAT),
            base_url=sql_value(CODEX_BASE_URL),
            custom_path=sql_value(CODEX_CUSTOM_PATH),
            config=sql_json({}),
            metadata=sql_json({"source": "sub2api"}),
        ).strip()
    )

    encrypted_placeholder = encrypt_fernet(encryption_key, PLACEHOLDER_API_KEY)
    synced_key_ids: list[str] = []
    for account in data["codex_accounts"]:
        key_id = source_provider_key_id(account["id"])
        synced_key_ids.append(key_id)
        account_active = truthy_active(account.get("status")) and bool(account.get("schedulable"))
        account_status = str(account.get("status") or "inactive")
        auth_config = build_auth_config(account)
        encrypted_auth_config = encrypt_fernet(
            encryption_key,
            json.dumps(auth_config, ensure_ascii=False, separators=(",", ":")),
        )
        metadata = provider_key_metadata(account)
        status_snapshot = codex_status_snapshot(account)
        credentials = account.get("credentials") or {}
        display_email = credentials.get("email") if isinstance(credentials, dict) else None
        name = account.get("name") or display_email or f"sub2api account {account['id']}"
        lines.append(
            """
INSERT INTO provider_api_keys (
  id, api_key, encrypted_key, name, note, internal_priority, rpm_limit,
  concurrent_limit, allowed_models, capabilities, request_count,
  success_count, error_count, total_response_time_ms, is_active,
  expires_at, created_at, updated_at, provider_id, api_formats,
  auth_type_by_format, auth_type, auth_config, upstream_metadata,
  total_tokens, total_cost_usd, status_snapshot, status, weight, metadata
) VALUES (
  {id}, {api_key}, NULL, {name}, {note}, {priority}, NULL,
  {concurrency}, NULL, {capabilities}, 0,
  0, 0, 0, {is_active},
  {expires_at}, {created_at}, {updated_at}, {provider_id}, {api_formats},
  {auth_type_by_format}, 'oauth', {auth_config}, {upstream_metadata},
  0, 0, {status_snapshot}, {status}, 1, {metadata}
)
ON CONFLICT (id) DO UPDATE SET
  api_key = EXCLUDED.api_key,
  name = EXCLUDED.name,
  note = EXCLUDED.note,
  internal_priority = EXCLUDED.internal_priority,
  concurrent_limit = EXCLUDED.concurrent_limit,
  is_active = EXCLUDED.is_active,
  expires_at = EXCLUDED.expires_at,
  updated_at = EXCLUDED.updated_at,
  provider_id = EXCLUDED.provider_id,
  api_formats = EXCLUDED.api_formats,
  auth_type_by_format = EXCLUDED.auth_type_by_format,
  auth_type = EXCLUDED.auth_type,
  auth_config = EXCLUDED.auth_config,
  upstream_metadata = EXCLUDED.upstream_metadata,
  status_snapshot = EXCLUDED.status_snapshot,
  status = EXCLUDED.status,
  metadata = EXCLUDED.metadata;
""".format(
                id=sql_value(key_id),
                api_key=sql_value(encrypted_placeholder),
                name=sql_value(str(name)[:255]),
                note=sql_value(account.get("notes") or account.get("error_message")),
                priority=int(account.get("priority") or 50),
                concurrency=sql_value(nullable_int(account.get("concurrency"))),
                capabilities=sql_json({"codex": True, "source": "sub2api"}),
                is_active=sql_value(account_active),
                expires_at=sql_ts(account.get("expires_at")),
                created_at=sql_ts(account.get("created_at")),
                updated_at=sql_ts(account.get("updated_at")),
                provider_id=sql_value(CODEX_PROVIDER_ID),
                api_formats=sql_json([CODEX_FORMAT]),
                auth_type_by_format=sql_json({CODEX_FORMAT: "oauth"}),
                auth_config=sql_value(encrypted_auth_config),
                upstream_metadata=sql_json(metadata, jsonb=True),
                status_snapshot=sql_json(status_snapshot),
                status=sql_value(account_status),
                metadata=sql_json({"source": "sub2api", "source_account_id": account.get("id")}),
            ).strip()
        )

    if synced_key_ids:
        lines.append(
            """
UPDATE provider_api_keys
SET is_active = false,
    status = 'inactive',
    note = coalesce(nullif(note, ''), 'sub2api 当前账号池已不存在，重新同步时自动停用'),
    updated_at = now()
WHERE provider_id = {provider_id}
  AND id LIKE 'sub2api-provider-key-%'
  AND id <> ALL ({synced_ids});
""".format(
                provider_id=sql_value(CODEX_PROVIDER_ID),
                synced_ids="ARRAY[" + ",".join(sql_value(item) for item in synced_key_ids) + "]::varchar[]",
            ).strip()
        )

    lines.append("COMMIT;")
    return "\n".join(lines) + "\n"


def summary(data: dict[str, list[dict[str, Any]]], counts: dict[str, int], import_counts: dict[str, int]) -> str:
    active_users = [user for user in data["users"] if truthy_active(user.get("status"))]
    user_statuses = Counter(str(user.get("status") or "unknown") for user in data["users"])
    key_groups = Counter(str(key.get("group_name") or "未分组") for key in data["api_keys"])
    account_plans: Counter[str] = Counter()
    account_statuses: Counter[str] = Counter()
    for account in data["codex_accounts"]:
        account_statuses[f"{account.get('status')}/{account.get('schedulable')}"] += 1
        credentials = account.get("credentials") or {}
        extra = account.get("extra") or {}
        if not isinstance(credentials, dict):
            credentials = {}
        if not isinstance(extra, dict):
            extra = {}
        account_plans[str(credentials.get("plan_type") or extra.get("plan_type") or "unknown")] += 1
    subscription_users = [
        f"{sub.get('user_email')} / {sub.get('group_name')} / 到期 {sub.get('expires_at')}"
        for sub in data["subscriptions"]
    ]
    lines = [
        "同步前检查结果",
        f"- 源库用户：{len(data['users'])} 个，其中 active {len(active_users)} 个；状态分布 {dict(user_statuses)}",
        f"- 源库 API Key：{len(data['api_keys'])} 个；分组分布 {dict(key_groups)}",
        f"- 源库 OpenAI 分组：{len(data['groups'])} 个",
        f"- 源库有效套餐：{len(data['subscriptions'])} 个" + (f"；{'; '.join(subscription_users)}" if subscription_users else ""),
        f"- 源库 Codex OAuth 账号：{len(data['codex_accounts'])} 个；状态分布 {dict(account_statuses)}；套餐分布 {dict(account_plans)}",
        f"- 目标库现有数量：{counts}",
        f"- 目标库已有 sub2api 导入数据：{import_counts}",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Sync sub2api users and Codex pool into Niffler")
    parser.add_argument("--apply", action="store_true", help="write to the Niffler database")
    parser.add_argument("--force", action="store_true", help="allow writing when imported/business rows already exist")
    parser.add_argument("--codex-only", action="store_true", help="only sync Codex provider, endpoint and OAuth accounts")
    parser.add_argument("--print-sql-size", action="store_true", help="print generated SQL byte size")
    args = parser.parse_args()

    data = fetch_source_data()
    counts = target_counts()
    import_counts = target_import_counts()
    print(summary(data, counts, import_counts))

    encryption_key = load_target_encryption_key()
    if args.codex_only:
        sql = generate_codex_only_sql(data, encryption_key)
    else:
        assert_safe_target(counts, import_counts, force=args.force)
        sql = generate_sql(data, encryption_key)
    if args.print_sql_size:
        print(f"- 生成 SQL 大小：{len(sql.encode())} bytes")

    if not args.apply:
        print("结果：试跑完成，没有写入 Niffler。")
        return 0

    print("开始写入 Niffler 数据库。")
    target_exec(sql)
    after_counts = target_counts()
    after_import_counts = target_import_counts()
    print(f"写入后目标库数量：{after_counts}")
    print(f"写入后 sub2api 导入数据：{after_import_counts}")
    print("结果：正式同步完成。")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SyncError as exc:
        print(f"错误：{exc}", file=sys.stderr)
        raise SystemExit(1)
