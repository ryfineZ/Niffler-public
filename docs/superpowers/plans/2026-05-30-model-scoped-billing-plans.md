# Model-Scoped Billing Plans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add model-scoped周期额度套餐 so a plan can apply only to selected global models while staying independent from API Key groups.

**Architecture:** Store `allowed_global_model_ids` inside each usage-quota entitlement snapshot. Before a request reaches an upstream provider, auth checks must resolve the requested model to a global model and only count quota from plans that include that global model. Settlement repeats the same model-scoped check when consuming quota. If no matching plan exists, the request falls through to wallet billing. API Key groups continue to control wallet pay-as-you-go rules, concurrency, and RPM, but do not block plan quota usage.

**Tech Stack:** Rust, sqlx, serde_json, Vue 3, TypeScript, existing `billing_plans.entitlements_json` / `user_plan_entitlements.entitlements_snapshot`.

---

### Task 1: Pass Global Model Into Settlement

**Files:**
- Modify: `crates/aether-data-contracts/src/repository/settlement/types.rs`
- Modify: `crates/aether-usage-runtime/src/settlement.rs`
- Modify: `crates/aether-data/src/repository/settlement/postgres.rs`
- Modify: `crates/aether-data/src/repository/settlement/sqlite.rs`

- [x] **Step 1: Add optional model identity to settlement input**

Add these fields to `UsageSettlementInput`:

```rust
pub global_model_id: Option<String>,
pub global_model_name: Option<String>,
pub model: Option<String>,
```

Keep them optional so old settlement callers and old usage rows keep working.

- [x] **Step 2: Populate settlement model identity from usage metadata**

When building `UsageSettlementInput` from usage runtime data, fill:

```rust
global_model_id: usage.request_metadata
    .as_ref()
    .and_then(|value| value.get("global_model_id"))
    .and_then(|value| value.as_str())
    .map(str::to_string),
global_model_name: usage.request_metadata
    .as_ref()
    .and_then(|value| value.get("global_model_name"))
    .and_then(|value| value.as_str())
    .map(str::to_string),
model: Some(usage.model.clone()).filter(|value| !value.trim().is_empty()),
```

If the local variable names differ, use the equivalent usage record fields already used in that file.

- [x] **Step 3: Run compile check for changed Rust crates**

Run:

```bash
cargo check -p aether-data-contracts -p aether-usage-runtime -p aether-data
```

Expected: no compile errors from the new optional fields.

---

### Task 2: Filter Plan Quota By Global Model

**Files:**
- Modify: `crates/aether-data/src/repository/billing/quota.rs`
- Modify: `crates/aether-data/src/repository/billing/postgres.rs`
- Modify: `crates/aether-data/src/repository/billing/sqlite.rs`
- Modify: `crates/aether-data/src/repository/settlement/postgres.rs`
- Modify: `crates/aether-data/src/repository/settlement/sqlite.rs`

- [x] **Step 1: Add model scope to `UsageQuotaGrant`**

Extend the grant struct:

```rust
pub allowed_global_model_ids: Option<Vec<String>>,
```

Add a helper:

```rust
pub(crate) fn entitlement_allows_global_model(
    allowed_global_model_ids: Option<&[String]>,
    request_global_model_id: Option<&str>,
) -> bool {
    let Some(allowed) = allowed_global_model_ids else {
        return true;
    };
    if allowed.is_empty() {
        return true;
    }
    let Some(request_global_model_id) = request_global_model_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    allowed.iter().any(|id| id == request_global_model_id)
}
```

- [x] **Step 2: Parse `allowed_global_model_ids` from entitlement JSON**

Inside `usage_quota_grants_from_entitlement`, parse the optional array:

```rust
let allowed_global_model_ids = item
    .get("allowed_global_model_ids")
    .and_then(|value| value.as_array())
    .map(|items| {
        items
            .iter()
            .filter_map(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    })
    .filter(|items| !items.is_empty());
```

Copy this value into every grant created from that entitlement item.

- [x] **Step 3: Filter grants during settlement**

In `consume_daily_quota_postgres` and `consume_daily_quota_sqlite`, add `request_global_model_id: Option<&str>` and filter grants before calculating remaining quota:

```rust
grants.retain(|grant| {
    entitlement_allows_global_model(
        grant.allowed_global_model_ids.as_deref(),
        request_global_model_id,
    )
});
```

Then call it with:

```rust
input.global_model_id.as_deref()
```

- [x] **Step 4: Add model-scoped quota availability for request preflight**

Add a repository method that keeps the existing aggregate method intact for display and legacy checks:

```rust
async fn find_user_daily_quota_availability_for_global_model(
    &self,
    user_id: &str,
    global_model_id: Option<&str>,
) -> Result<Option<UserDailyQuotaAvailabilityRecord>, crate::DataLayerError>
```

Postgres, SQLite, MySQL, and memory repositories must filter grants with `entitlement_allows_global_model` when `global_model_id` is present. The old `find_user_daily_quota_availability` remains aggregate data and must not be used as the final model permission check before upstream dispatch.

- [x] **Step 4.1: Avoid reading quota windows for unrelated plans**

Before reading a five-hour quota window, inspect the entitlement snapshot first:

```rust
entitlements_snapshot_has_usage_quota_for_global_model(&entitlements, request_global_model_id)
```

If the entitlement has quota only for another global model, skip it immediately. This keeps a Claude request from reading Codex-only quota windows and reduces database work before both auth checks and settlement.

- [x] **Step 5: Check plan model scope before upstream dispatch**

In `apps/aether-gateway/src/control/auth/gate.rs`, resolve the requested model to a global model before returning `ModelNotAllowed`.

When the API Key group does not allow the requested model:

1. If the user has active plan quota for that resolved global model, allow the request to continue.
2. If the user has no matching plan quota, keep the existing `ModelNotAllowed` result.
3. If the model cannot be resolved, keep the existing behavior and do not assume a plan applies.

Also update `balance_capacity_rejection` to use `find_user_daily_quota_availability_for_global_model` when a requested global model is known. This prevents a user with a Codex-only plan and no wallet balance from sending a Claude request to the upstream provider.

- [x] **Step 5.1: Reuse preflight quota lookup and add short runtime cache**

The auth path must not query scoped plan quota twice for the same request. If the model permission branch has already loaded the user quota for the resolved global model, the balance branch reuses that result.

Quota availability is cached for a short period:

```text
key: billing:daily_quota_availability:v1:{user_id}:{global_model_id}
ttl: 3 seconds
```

Redis-backed runtime state shares this across gateway nodes. If Redis is unavailable, the request falls back to the database and does not fail only because cache read/write failed. The cache is only a request-admission optimization; final quota consumption and ledger records remain in the database.

The capacity check only loads plan quota after the request model is known and the estimated request cost is greater than zero. Requests without a model, or requests that resolve to a zero-cost provider/model, do not need plan quota admission and must not query quota availability.

- [x] **Step 6: Add unit tests for model matching**

Add tests in `crates/aether-data/src/repository/billing/quota.rs`:

```rust
#[test]
fn scoped_entitlement_allows_matching_global_model() {
    assert!(entitlement_allows_global_model(
        Some(&["global-codex".to_string()]),
        Some("global-codex"),
    ));
}

#[test]
fn scoped_entitlement_rejects_other_global_model() {
    assert!(!entitlement_allows_global_model(
        Some(&["global-codex".to_string()]),
        Some("global-claude"),
    ));
}

#[test]
fn unscoped_entitlement_keeps_legacy_behavior() {
    assert!(entitlement_allows_global_model(None, Some("global-claude")));
}
```

- [x] **Step 7: Run tests**

Run:

```bash
cargo test -p aether-data repository::billing::quota
```

Expected: all quota parsing and model-scope tests pass.

---

### Task 3: Validate Plan Model Scope In Admin API

**Files:**
- Modify: `apps/aether-gateway/src/handlers/admin/billing/plans.rs`

- [x] **Step 1: Require model scope for usage quota entitlements**

In `validate_entitlements`, when `kind` is `daily_quota` or `usage_quota`, require `allowed_global_model_ids` to be a non-empty string array:

```rust
let allowed_global_model_ids = item
    .get("allowed_global_model_ids")
    .and_then(|value| value.as_array())
    .ok_or_else(|| format!("{kind}.allowed_global_model_ids is required"))?;
if allowed_global_model_ids.is_empty() {
    return Err(format!("{kind}.allowed_global_model_ids must not be empty"));
}
for id in allowed_global_model_ids {
    let id = id
        .as_str()
        .ok_or_else(|| format!("{kind}.allowed_global_model_ids must contain strings"))?;
    if id.trim().is_empty() {
        return Err(format!("{kind}.allowed_global_model_ids must not contain empty strings"));
    }
}
```

- [x] **Step 2: Keep old purchased entitlements usable**

Do not reject existing `user_plan_entitlements.entitlements_snapshot` during settlement if the array is missing. Old snapshots remain unscoped and continue to work.

- [x] **Step 3: Add handler tests**

Extend existing tests in the same file:

```rust
#[test]
fn usage_quota_requires_allowed_global_model_ids() {
    let mut request = sample_plan_request(9.9);
    request.entitlements = json!([{
        "type": "daily_quota",
        "daily_quota_usd": 100.0,
        "allow_wallet_overage": false
    }]);
    let err = normalize_plan_input_for_create(request).expect_err("model scope required");
    assert!(err.contains("allowed_global_model_ids"));
}

#[test]
fn usage_quota_accepts_allowed_global_model_ids() {
    let mut request = sample_plan_request(9.9);
    request.entitlements = json!([{
        "type": "daily_quota",
        "daily_quota_usd": 100.0,
        "allow_wallet_overage": false,
        "allowed_global_model_ids": ["global-codex"]
    }]);
    let input = normalize_plan_input_for_create(request).expect("valid scoped quota");
    assert_eq!(input.entitlements_json[0]["allowed_global_model_ids"][0], "global-codex");
}
```

- [x] **Step 4: Run tests**

Run:

```bash
cargo test -p aether-gateway handlers::admin::billing::plans
```

Expected: plan validation tests pass.

---

### Task 4: Add Model Scope UI To Billing Plan Management

**Files:**
- Modify: `frontend/src/api/billing.ts`
- Modify: `frontend/src/views/admin/BillingPlansManagement.vue`

- [x] **Step 1: Add frontend type**

Add to `DailyQuotaEntitlement`:

```ts
allowed_global_model_ids?: string[]
```

Add to `PlanFormState`:

```ts
allowed_global_model_ids: string[]
```

- [x] **Step 2: Load global models**

Import and call `getGlobalModels`:

```ts
import { getGlobalModels } from '@/api/global-models'
import type { GlobalModelResponse } from '@/api/endpoints/types/model'
```

Add state:

```ts
const globalModels = ref<GlobalModelResponse[]>([])
const loadingGlobalModels = ref(false)
```

Add loader:

```ts
async function loadGlobalModels() {
  loadingGlobalModels.value = true
  try {
    const response = await getGlobalModels({ skip: 0, limit: 500, is_active: true })
    globalModels.value = response.models
  } catch (err) {
    log.error('加载模型失败:', err)
    showError(parseApiError(err, '加载模型失败'))
  } finally {
    loadingGlobalModels.value = false
  }
}
```

Call it in `onMounted` together with plans and user groups.

- [x] **Step 3: Preserve model scope while editing**

In `formFromPlan`, when reading quota entitlement:

```ts
next.allowed_global_model_ids = Array.isArray(quota.allowed_global_model_ids)
  ? [...quota.allowed_global_model_ids]
  : []
```

In `buildEntitlements`, add:

```ts
allowed_global_model_ids: [...form.allowed_global_model_ids],
```

- [x] **Step 4: Validate model scope before saving**

In `validatePlan`:

```ts
if (form.daily_quota_enabled && form.allowed_global_model_ids.length === 0) {
  return '周期额度套餐至少选择一个可用模型'
}
```

- [x] **Step 5: Add model selector UI**

In the daily quota card, add the existing searchable `MultiSelect` component for global models. Preserve unknown legacy IDs as fallback labels so old plans can still be edited safely.

- [x] **Step 6: Show badges in plan list**

In `entitlementBadges`, append:

```ts
const modelCount = Array.isArray(entitlement.allowed_global_model_ids)
  ? entitlement.allowed_global_model_ids.length
  : 0
if (modelCount > 0) {
  parts.push(`${modelCount} 个模型`)
}
```

- [x] **Step 7: Run frontend validation**

Run:

```bash
npm --prefix frontend run type-check
```

Expected: TypeScript passes.

---

### Task 5: User-Facing Display And Admin Grant Display

**Files:**
- Modify: `frontend/src/views/user/BillingPlans.vue`
- Modify: `frontend/src/views/admin/Users.vue`
- Modify: `frontend/src/api/billing.ts`

- [x] **Step 1: Show model count on user plan cards**

When rendering quota entitlement details, show:

```ts
const modelCount = Array.isArray(item.allowed_global_model_ids)
  ? item.allowed_global_model_ids.length
  : 0
```

Display text:

```text
可用模型：{modelCount} 个
```

- [x] **Step 2: Show model count in manual grant dropdown**

When formatting a plan in `Users.vue`, include the selected global model count so administrators do not accidentally grant the wrong package.

- [x] **Step 3: Run frontend typecheck**

Run:

```bash
npm --prefix frontend run type-check
```

Expected: TypeScript passes.

---

### Task 6: Final Verification

**Files:**
- Review: `docs/architecture/group-api-key-pricing-plan-design.md`
- Review: all modified Rust and frontend files

- [x] **Step 1: Run backend tests**

Run:

```bash
cargo test -p aether-data repository::billing::quota
cargo test -p aether-gateway handlers::admin::billing::plans
```

Expected: all tests pass.

- [x] **Step 2: Run frontend checks**

Run:

```bash
npm --prefix frontend run type-check
```

Expected: no TypeScript errors.

- [ ] **Step 3: Manual behavior check**

Create two plans in admin UI:

```text
Plan A: 9.9 CNY, 100 USD, allowed_global_model_ids = [Codex global model]
Plan B: 19.9 CNY, 50 USD, allowed_global_model_ids = [Claude global model]
```

Expected behavior:

```text
Codex request uses Plan A.
Claude request does not use Plan A.
Claude request uses Plan B.
If no matching plan exists, request falls back to wallet pay-as-you-go rules.
API Key group model restrictions do not block matching plan usage.
API Key group concurrency and RPM still apply.
```

- [x] **Step 4: Document any known limitation**

If provider batch selection is not included in the first implementation, note that the first version supports direct global model selection only, and provider-assisted batch selection can be added after the core settlement behavior is safe.
