#!/usr/bin/env python3
"""Read-only readiness checks for the Niffler core refactor.

The script only runs SELECT statements. It expects a Postgres DATABASE_URL
and prints a compact report that highlights old data patterns that must be
handled before switching runtime traffic to the new Niffler model.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from typing import Any


SHADOW_TABLES = [
    "niffler_upstream_services",
    "niffler_upstream_accounts",
    "niffler_product_plans",
    "niffler_product_plan_models",
    "niffler_model_base_prices",
    "niffler_upstream_model_prices",
    "niffler_account_model_capabilities",
    "niffler_upstream_service_capabilities",
    "niffler_settlement_snapshots",
    "niffler_billing_reservations",
    "niffler_billing_reservation_events",
    "niffler_route_attempts",
    "niffler_error_return_settings",
    "niffler_account_risk_events",
    "niffler_api_key_pauses",
    "niffler_referral_reward_rules",
    "niffler_referral_reward_ledger",
    "niffler_referral_reward_events",
]


@dataclass(frozen=True)
class Check:
    name: str
    title: str
    severity: str
    sql: str
    note: str


class ReadinessError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run read-only Postgres checks before the Niffler core refactor."
    )
    parser.add_argument(
        "--database-url",
        default=os.environ.get("DATABASE_URL"),
        help="Postgres DATABASE_URL. Defaults to the DATABASE_URL environment variable.",
    )
    parser.add_argument(
        "--recent-days",
        type=int,
        default=7,
        help="Recent usage/request window in days. Defaults to 7.",
    )
    parser.add_argument(
        "--statement-timeout-ms",
        type=int,
        default=5000,
        help="Per-check statement timeout. Defaults to 5000 ms.",
    )
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="Output format. Defaults to text.",
    )
    parser.add_argument(
        "--psql-bin",
        default="psql",
        help="psql binary path. Defaults to psql.",
    )
    return parser.parse_args()


def require_safe_args(args: argparse.Namespace) -> None:
    if not args.database_url:
        raise ReadinessError("DATABASE_URL 未设置，请通过环境变量或 --database-url 传入。")
    if args.recent_days < 1 or args.recent_days > 90:
        raise ReadinessError("--recent-days 必须在 1 到 90 之间。")
    if args.statement_timeout_ms < 500 or args.statement_timeout_ms > 60000:
        raise ReadinessError("--statement-timeout-ms 必须在 500 到 60000 之间。")
    if not shutil.which(args.psql_bin):
        raise ReadinessError(f"找不到 psql：{args.psql_bin}")


def sql_json_query(sql: str, statement_timeout_ms: int) -> str:
    return f"""
SET statement_timeout = {statement_timeout_ms};
SELECT COALESCE(json_agg(row_to_json(readiness_row)), '[]'::json)
FROM (
{sql}
) AS readiness_row;
"""


def run_sql(args: argparse.Namespace, sql: str) -> list[dict[str, Any]]:
    command = [
        args.psql_bin,
        "-X",
        "-q",
        "-A",
        "-t",
        "-v",
        "ON_ERROR_STOP=1",
        "-c",
        sql_json_query(sql, args.statement_timeout_ms),
    ]
    env = os.environ.copy()
    env["PGDATABASE"] = args.database_url
    proc = subprocess.run(command, text=True, capture_output=True, check=False, env=env)
    if proc.returncode != 0:
        stderr = proc.stderr.strip() or proc.stdout.strip()
        raise ReadinessError(stderr)
    text = proc.stdout.strip()
    if not text:
        return []
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ReadinessError(f"psql 返回内容不是 JSON：{text[:200]}") from exc
    if not isinstance(value, list):
        raise ReadinessError("psql 返回 JSON 不是数组。")
    return value


def checks(recent_days: int) -> list[Check]:
    shadow_values = ", ".join(f"('{table}')" for table in SHADOW_TABLES)
    return [
        Check(
            name="shadow_tables",
            title="Niffler 影子表是否存在",
            severity="info",
            note="第 1 批迁移完成后，这里应该全部为 true。",
            sql=f"""
SELECT table_name,
       to_regclass('public.' || table_name) IS NOT NULL AS exists
FROM (VALUES {shadow_values}) AS expected(table_name)
ORDER BY table_name
""",
        ),
        Check(
            name="api_key_group_integrity",
            title="用户 Key 分组完整性",
            severity="error",
            note="missing_group 或 dangling_group 大于 0 时，切换产品策略前必须修复。",
            sql="""
SELECT COUNT(*) AS total_api_keys,
       COUNT(*) FILTER (WHERE api_keys.group_id IS NULL) AS missing_group,
       COUNT(*) FILTER (WHERE api_keys.group_id IS NOT NULL AND user_groups.id IS NULL) AS dangling_group
FROM public.api_keys
LEFT JOIN public.user_groups
  ON user_groups.id = api_keys.group_id
""",
        ),
        Check(
            name="api_key_legacy_scopes",
            title="用户 Key 自身残留限制",
            severity="warning",
            note="新模型里用户 Key 只绑定产品策略，这些 Key 级限制需要迁移或删除。",
            sql="""
SELECT COUNT(*) FILTER (
         WHERE allowed_providers IS NOT NULL
           AND allowed_providers::text NOT IN ('null', '[]')
       ) AS keys_with_allowed_providers,
       COUNT(*) FILTER (
         WHERE allowed_api_formats IS NOT NULL
           AND allowed_api_formats::text NOT IN ('null', '[]')
       ) AS keys_with_allowed_api_formats,
       COUNT(*) FILTER (
         WHERE allowed_models IS NOT NULL
           AND allowed_models::text NOT IN ('null', '[]')
       ) AS keys_with_allowed_models
FROM public.api_keys
""",
        ),
        Check(
            name="user_group_policy_gaps",
            title="旧分组策略空配置",
            severity="warning",
            note="specific 模式配空数组会导致用户以为开放了模型或服务，实际请求会被限制。",
            sql="""
SELECT COUNT(*) AS total_groups,
       COUNT(*) FILTER (
         WHERE allowed_models_mode = 'specific'
           AND CASE
             WHEN allowed_models IS NULL THEN true
             WHEN json_typeof(allowed_models) <> 'array' THEN true
             ELSE json_array_length(allowed_models) = 0
           END
       ) AS specific_models_empty,
       COUNT(*) FILTER (
         WHERE allowed_providers_mode = 'specific'
           AND CASE
             WHEN allowed_providers IS NULL THEN true
             WHEN json_typeof(allowed_providers) <> 'array' THEN true
             ELSE json_array_length(allowed_providers) = 0
           END
       ) AS specific_providers_empty,
       COUNT(*) FILTER (
         WHERE allowed_api_formats_mode = 'specific'
           AND CASE
             WHEN allowed_api_formats IS NULL THEN true
             WHEN json_typeof(allowed_api_formats) <> 'array' THEN true
             ELSE json_array_length(allowed_api_formats) = 0
           END
       ) AS specific_api_formats_empty
FROM public.user_groups
""",
        ),
        Check(
            name="disabled_provider_references",
            title="旧分组引用停用提供商",
            severity="warning",
            note="停用提供商仍能被旧分组引用时，新产品策略页面必须禁止继续选择。",
            sql="""
WITH group_provider_refs AS (
    SELECT user_groups.id AS group_id,
           user_groups.name AS group_name,
           provider_ref.provider_id
    FROM public.user_groups
    CROSS JOIN LATERAL json_array_elements_text(
      CASE
        WHEN user_groups.allowed_providers IS NOT NULL
          AND json_typeof(user_groups.allowed_providers) = 'array'
        THEN user_groups.allowed_providers
        ELSE '[]'::json
      END
    ) AS provider_ref(provider_id)
    WHERE user_groups.allowed_providers_mode = 'specific'
)
SELECT COUNT(*) AS disabled_reference_count
FROM group_provider_refs
JOIN public.providers
  ON providers.id = group_provider_refs.provider_id
WHERE providers.is_active = false
   OR providers.enabled = false
""",
        ),
        Check(
            name="provider_state_counts",
            title="旧 Provider 状态分布",
            severity="info",
            note="用于评估上游服务迁移规模。",
            sql="""
SELECT COALESCE(provider_type, 'unknown') AS provider_type,
       enabled,
       is_active,
       COUNT(*) AS count
FROM public.providers
GROUP BY COALESCE(provider_type, 'unknown'), enabled, is_active
ORDER BY count DESC, provider_type ASC
""",
        ),
        Check(
            name="provider_key_state_counts",
            title="旧上游账号状态分布",
            severity="info",
            note="用于评估账号状态迁移到可用、停用、失效、额度耗尽、冷却中的规则。",
            sql="""
SELECT COALESCE(status, 'unknown') AS status,
       is_active,
       COUNT(*) AS count
FROM public.provider_api_keys
GROUP BY COALESCE(status, 'unknown'), is_active
ORDER BY count DESC, status ASC
""",
        ),
        Check(
            name="recent_unknown_usage",
            title="最近使用记录里的 unknown/pending",
            severity="error",
            note="实际请求到上游后仍出现 unknown/pending，必须在新请求记录模型中消除。",
            sql=f"""
SELECT provider_name,
       COUNT(*) AS count
FROM public.usage
WHERE created_at >= now() - interval '{recent_days} days'
  AND (
    provider_name IN ('unknown', 'pending')
    OR provider_id IS NULL
  )
GROUP BY provider_name
ORDER BY count DESC, provider_name ASC
LIMIT 20
""",
        ),
        Check(
            name="recent_request_candidate_skips",
            title="最近路由跳过原因",
            severity="info",
            note="用于设计新 route_attempts 的跳过原因枚举和展示文案。",
            sql=f"""
SELECT status,
       COALESCE(NULLIF(skip_reason, ''), 'none') AS skip_reason,
       COUNT(*) AS count
FROM public.request_candidates
WHERE created_at >= now() - interval '{recent_days} days'
GROUP BY status, COALESCE(NULLIF(skip_reason, ''), 'none')
ORDER BY count DESC, status ASC
LIMIT 30
""",
        ),
        Check(
            name="model_price_gaps",
            title="模型价格缺口",
            severity="warning",
            note="基础价、销售价和成本价统一前，需要确认缺价模型如何处理。",
            sql="""
SELECT
  (SELECT COUNT(*)
   FROM public.global_models
   WHERE enabled = true
     AND is_active = true
     AND default_tiered_pricing IS NULL
     AND default_price_per_request IS NULL) AS active_global_models_without_price,
  (SELECT COUNT(*)
   FROM public.models
   WHERE enabled = true
     AND is_active = true
     AND tiered_pricing IS NULL
     AND price_per_request IS NULL) AS active_provider_models_without_price
""",
        ),
    ]


def run_checks(args: argparse.Namespace) -> dict[str, Any]:
    report: dict[str, Any] = {
        "database": "postgres",
        "read_only": True,
        "recent_days": args.recent_days,
        "statement_timeout_ms": args.statement_timeout_ms,
        "checks": [],
    }
    for check in checks(args.recent_days):
        try:
            rows = run_sql(args, check.sql)
            report["checks"].append(
                {
                    "name": check.name,
                    "title": check.title,
                    "severity": check.severity,
                    "note": check.note,
                    "ok": True,
                    "rows": rows,
                }
            )
        except ReadinessError as exc:
            report["checks"].append(
                {
                    "name": check.name,
                    "title": check.title,
                    "severity": check.severity,
                    "note": check.note,
                    "ok": False,
                    "error": str(exc),
                }
            )
    return report


def print_text(report: dict[str, Any]) -> None:
    print("Niffler 核心重构只读检查")
    print(f"数据库：{report['database']}")
    print(f"最近记录窗口：{report['recent_days']} 天")
    print(f"单条 SQL 超时：{report['statement_timeout_ms']} ms")
    print("")
    for check in report["checks"]:
        status = "OK" if check["ok"] else "FAIL"
        print(f"[{status}] {check['title']} ({check['severity']})")
        print(f"说明：{check['note']}")
        if not check["ok"]:
            print(f"错误：{check['error']}")
        else:
            rows = check["rows"]
            if not rows:
                print("结果：无记录")
            else:
                for row in rows:
                    print("结果：" + json.dumps(row, ensure_ascii=False, sort_keys=True))
        print("")


def main() -> int:
    args = parse_args()
    try:
        require_safe_args(args)
        report = run_checks(args)
    except ReadinessError as exc:
        print(f"检查失败：{exc}", file=sys.stderr)
        return 2

    if args.format == "json":
        print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
    else:
        print_text(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
