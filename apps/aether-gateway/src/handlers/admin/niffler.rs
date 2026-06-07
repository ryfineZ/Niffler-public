use std::collections::{BTreeMap, BTreeSet};

use aether_data::repository::users::StoredUserGroup;
use aether_data_contracts::repository::candidates::StoredRequestCandidate;
use aether_data_contracts::repository::global_models::{
    AdminGlobalModelListQuery, AdminProviderModelListQuery, StoredAdminGlobalModel,
    StoredAdminProviderModel,
};
use aether_data_contracts::repository::niffler_core::{
    NifflerCoreMappingSummary, NifflerCoreReadinessReport, NifflerCoreReadinessSummary,
    NifflerDisabledProviderReference, NifflerGroupPolicyGap, NifflerKeyScopeResidue,
    NifflerPriceGap, NifflerReadinessIssue, NifflerReadinessSeverity,
    NifflerRouteSkipReasonSummary, NifflerRouteSkipSample, NifflerShadowTableItem,
    NifflerShadowTableStatus, NifflerUsageAnomaly,
};
use aether_data_contracts::repository::provider_catalog::{
    StoredProviderCatalogKey, StoredProviderCatalogProvider,
};
use aether_data_contracts::repository::usage::{StoredRequestUsageAudit, UsageAuditListQuery};
use axum::{
    body::Body,
    http,
    response::{IntoResponse, Response},
    Json,
};

use crate::clock::current_unix_secs;
use crate::handlers::admin::request::{AdminAppState, AdminRequestContext, AdminRouteRequest};
use crate::handlers::admin::shared::query_param_value;
use crate::handlers::shared::{
    parse_catalog_auth_config_json, provider_key_account_label_from_auth_config,
};
use crate::GatewayError;

const READINESS_PATH: &str = "/api/admin/niffler-core/readiness";
const MAX_ISSUE_ITEMS: usize = 50;
const MAX_USAGE_SCAN: usize = 200;
const MAX_USAGE_ITEMS: usize = 50;
const MAX_PROVIDER_MODELS_PER_PROVIDER: usize = 2_000;
const MAX_GLOBAL_MODELS: usize = 10_000;
const MAX_ROUTE_SKIP_SAMPLE: usize = 500;
const SHADOW_TABLES: &[&str] = &[
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
];

pub(crate) async fn maybe_build_local_admin_niffler_response(
    request: AdminRouteRequest<'_>,
) -> Result<Option<Response<Body>>, GatewayError> {
    let state = request.state();
    let request_context = request.request_context();

    if request_context.route_family() != Some("niffler_core_manage")
        || request_context.path() != READINESS_PATH
    {
        return Ok(None);
    }
    if request_context.method() != http::Method::GET {
        return Ok(Some(
            (
                http::StatusCode::METHOD_NOT_ALLOWED,
                Json(serde_json::json!({ "detail": "只支持只读检查" })),
            )
                .into_response(),
        ));
    }

    let recent_days = parse_recent_days(request_context.query_string());
    let report = build_readiness_report(&state, recent_days).await?;
    Ok(Some(Json(report).into_response()))
}

fn parse_recent_days(query_string: Option<&str>) -> u32 {
    query_param_value(query_string, "recent_days")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| (1..=90).contains(value))
        .unwrap_or(7)
}

async fn build_readiness_report(
    state: &AdminAppState<'_>,
    recent_days: u32,
) -> Result<NifflerCoreReadinessReport, GatewayError> {
    let generated_at_unix_secs = current_unix_secs();
    let shadow_tables = build_shadow_table_status(state).await?;

    let providers = if state.has_provider_catalog_data_reader() {
        state.list_provider_catalog_providers(false).await?
    } else {
        Vec::new()
    };
    let provider_ids = providers
        .iter()
        .map(|provider| provider.id.clone())
        .collect::<Vec<_>>();
    let keys = if state.has_provider_catalog_data_reader() && !provider_ids.is_empty() {
        state
            .list_provider_catalog_key_summaries_by_provider_ids(&provider_ids)
            .await?
    } else {
        Vec::new()
    };
    let user_groups = state.list_user_groups().await?;
    let global_models = if state.has_global_model_data_reader() {
        state
            .list_admin_global_models(&AdminGlobalModelListQuery {
                offset: 0,
                limit: MAX_GLOBAL_MODELS,
                is_active: None,
                search: None,
            })
            .await?
            .items
    } else {
        Vec::new()
    };
    let provider_models = read_provider_models(state, &providers).await?;
    let provider_map = providers
        .iter()
        .map(|provider| (provider.id.as_str(), provider))
        .collect::<BTreeMap<_, _>>();
    let key_map = keys
        .iter()
        .map(|key| (key.id.as_str(), key))
        .collect::<BTreeMap<_, _>>();

    let disabled_provider_references =
        collect_disabled_provider_references(&user_groups, &provider_map);
    let mut key_scope_residue = collect_key_scope_residue(&keys, &provider_map);
    let group_policy_gaps = collect_group_policy_gaps(&user_groups);
    let price_gaps = collect_price_gaps(&global_models, &provider_models, &provider_map);
    let (mut recent_usage_anomalies, recent_problem_usage_sample_count) =
        collect_recent_usage_anomalies(
            state,
            recent_days,
            generated_at_unix_secs,
            &provider_map,
            &key_map,
        )
        .await?;
    let (route_skip_reasons, mut route_skip_samples) =
        collect_route_skip_reports(state, &provider_map, &key_map).await?;
    let account_labels = load_account_labels_for_readiness(
        state,
        &key_scope_residue,
        &recent_usage_anomalies,
        &route_skip_samples,
    )
    .await?;
    apply_account_labels_to_key_residue(&mut key_scope_residue, &account_labels);
    apply_account_labels_to_usage_anomalies(&mut recent_usage_anomalies, &account_labels);
    apply_account_labels_to_route_skip_samples(&mut route_skip_samples, &account_labels);
    let provider_status_counts = provider_status_counts(&providers);
    let account_status_counts = account_status_counts(&keys);
    let issues = collect_issues(
        state,
        &shadow_tables,
        &disabled_provider_references,
        &key_scope_residue,
        &group_policy_gaps,
        &price_gaps,
        &recent_usage_anomalies,
    );

    Ok(NifflerCoreReadinessReport {
        schema_version: 1,
        generated_at_unix_secs,
        recent_days,
        shadow_tables,
        summary: NifflerCoreReadinessSummary {
            providers_total: providers.len() as u64,
            providers_active: providers
                .iter()
                .filter(|provider| provider.is_active)
                .count() as u64,
            provider_keys_total: keys.len() as u64,
            provider_keys_active: keys.iter().filter(|key| key.is_active).count() as u64,
            product_plans_total: user_groups.len() as u64,
            product_plans_public: user_groups
                .iter()
                .filter(|group| group.visibility.trim().eq_ignore_ascii_case("public"))
                .count() as u64,
            global_models_total: global_models.len() as u64,
            global_models_active: global_models.iter().filter(|model| model.is_active).count()
                as u64,
            recent_problem_usage_sample_count,
        },
        provider_mapping: NifflerCoreMappingSummary {
            legacy_count: providers.len() as u64,
            mapped_count: providers
                .iter()
                .filter(|provider| provider.is_active)
                .count() as u64,
            blocked_count: providers
                .iter()
                .filter(|provider| !provider.is_active)
                .count() as u64,
            notes: vec![
                "启用 Provider 可以映射为上游服务；停用 Provider 不能被新产品策略选择。"
                    .to_string(),
            ],
        },
        account_mapping: NifflerCoreMappingSummary {
            legacy_count: keys.len() as u64,
            mapped_count: keys
                .iter()
                .filter(|key| key_status_label(key) == "available")
                .count() as u64,
            blocked_count: keys
                .iter()
                .filter(|key| key_status_label(key) != "available")
                .count() as u64,
            notes: vec!["启用且未标记 OAuth 失效的 Provider Key 可以映射为上游账号。".to_string()],
        },
        product_plan_mapping: NifflerCoreMappingSummary {
            legacy_count: user_groups.len() as u64,
            mapped_count: user_groups.len() as u64,
            blocked_count: 0,
            notes: vec![
                "旧用户分组可以映射为产品策略；公开/内部只影响是否允许用户 Key 公开绑定。"
                    .to_string(),
            ],
        },
        provider_status_counts,
        account_status_counts,
        disabled_provider_references,
        key_scope_residue,
        group_policy_gaps,
        price_gaps,
        recent_usage_anomalies,
        route_skip_reasons,
        route_skip_samples,
        issues,
    })
}

async fn build_shadow_table_status(
    state: &AdminAppState<'_>,
) -> Result<NifflerShadowTableStatus, GatewayError> {
    let rows = state
        .app()
        .data
        .check_table_existence(SHADOW_TABLES)
        .await
        .map_err(|err| GatewayError::Internal(err.to_string()))?;
    let tables = rows
        .into_iter()
        .map(|(table_name, exists)| NifflerShadowTableItem { table_name, exists })
        .collect::<Vec<_>>();
    let existing_tables = tables.iter().filter(|table| table.exists).count() as u64;
    Ok(NifflerShadowTableStatus {
        database_driver: state.app().data.database_driver_name(),
        expected_tables: SHADOW_TABLES.len() as u64,
        existing_tables,
        all_present: existing_tables == SHADOW_TABLES.len() as u64,
        tables,
    })
}

async fn read_provider_models(
    state: &AdminAppState<'_>,
    providers: &[StoredProviderCatalogProvider],
) -> Result<Vec<StoredAdminProviderModel>, GatewayError> {
    if !state.has_global_model_data_reader() {
        return Ok(Vec::new());
    }
    let mut models = Vec::new();
    for provider in providers {
        let mut provider_models = state
            .list_admin_provider_models(&AdminProviderModelListQuery {
                provider_id: provider.id.clone(),
                is_active: None,
                offset: 0,
                limit: MAX_PROVIDER_MODELS_PER_PROVIDER,
            })
            .await?;
        models.append(&mut provider_models);
    }
    Ok(models)
}

fn collect_disabled_provider_references(
    user_groups: &[StoredUserGroup],
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
) -> Vec<NifflerDisabledProviderReference> {
    let mut references = Vec::new();
    for group in user_groups {
        if !group
            .allowed_providers_mode
            .trim()
            .eq_ignore_ascii_case("specific")
        {
            continue;
        }
        if let Some(provider_ids) = &group.allowed_providers {
            for provider_id in provider_ids {
                push_disabled_provider_reference(
                    &mut references,
                    group,
                    provider_id,
                    "allowed_providers",
                    provider_map,
                );
            }
        }
    }
    references.truncate(MAX_ISSUE_ITEMS);
    references
}

fn push_disabled_provider_reference(
    references: &mut Vec<NifflerDisabledProviderReference>,
    group: &StoredUserGroup,
    provider_id: &str,
    source_field: &str,
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
) {
    let Some(provider) = provider_map.get(provider_id).copied().or_else(|| {
        provider_map
            .values()
            .copied()
            .find(|provider| provider.name == provider_id)
    }) else {
        return;
    };
    if provider.is_active {
        return;
    }
    let exists = references.iter().any(|item| {
        item.product_plan_id == group.id
            && item.provider_id == provider.id
            && item.source_field == source_field
    });
    if exists {
        return;
    }
    references.push(NifflerDisabledProviderReference {
        product_plan_id: group.id.clone(),
        product_plan_name: group.name.clone(),
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        source_field: source_field.to_string(),
        source_field_label: source_field_label(source_field).to_string(),
        reason: "分组的可用 Provider 列表里仍包含已停用 Provider。".to_string(),
        impact: "迁移到新产品策略后，停用 Provider 不允许被选择；如果不处理，这个分组实际可用服务会比页面配置少。".to_string(),
        recommended_action: "从分组里移除这个 Provider，或先恢复 Provider 再迁移。".to_string(),
    });
}

fn collect_key_scope_residue(
    keys: &[StoredProviderCatalogKey],
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
) -> Vec<NifflerKeyScopeResidue> {
    let mut residue = Vec::new();
    for key in keys {
        let mut fields = Vec::new();
        push_json_field_if_present(&mut fields, "api_formats", &key.api_formats);
        push_json_field_if_present(&mut fields, "auth_type_by_format", &key.auth_type_by_format);
        push_json_field_if_present(
            &mut fields,
            "allow_auth_channel_mismatch_formats",
            &key.allow_auth_channel_mismatch_formats,
        );
        push_json_field_if_present(&mut fields, "rate_multipliers", &key.rate_multipliers);
        push_json_field_if_present(
            &mut fields,
            "global_priority_by_format",
            &key.global_priority_by_format,
        );
        push_json_field_if_present(&mut fields, "allowed_models", &key.allowed_models);
        push_json_field_if_present(&mut fields, "locked_models", &key.locked_models);
        push_json_field_if_present(
            &mut fields,
            "model_include_patterns",
            &key.model_include_patterns,
        );
        push_json_field_if_present(
            &mut fields,
            "model_exclude_patterns",
            &key.model_exclude_patterns,
        );
        if fields.is_empty() {
            continue;
        }
        let provider_name = provider_map
            .get(key.provider_id.as_str())
            .map(|provider| provider.name.clone());
        let display_name = non_empty_string(&key.name).unwrap_or_else(|| key.id.clone());
        let field_labels = fields
            .iter()
            .map(|field| residue_field_label(field).to_string())
            .collect::<Vec<_>>();
        residue.push(NifflerKeyScopeResidue {
            subject_kind: "provider_key".to_string(),
            key_id: key.id.clone(),
            key_name: Some(key.name.clone()),
            owner_label: provider_name.clone().or_else(|| Some(key.provider_id.clone())),
            display_name,
            provider_id: Some(key.provider_id.clone()),
            provider_name,
            account_label: None,
            residue_fields: fields,
            field_labels,
            reason: "这把上游账号仍在 Key 自身保存模型、格式或优先级限制。".to_string(),
            impact: "新模型里这些限制应该归到账号能力或调度策略；如果继续散落在 Key 上，页面和后端调度容易不一致。".to_string(),
            recommended_action: "迁移前确认这些限制是否还需要保留，需要保留的迁到账号能力或调度策略，不需要的清理掉。".to_string(),
        });
    }
    residue.truncate(MAX_ISSUE_ITEMS);
    residue
}

fn push_json_field_if_present(
    fields: &mut Vec<String>,
    field_name: &str,
    value: &Option<serde_json::Value>,
) {
    if value.as_ref().is_some_and(value_has_content) {
        fields.push(field_name.to_string());
    }
}

fn value_has_content(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(object) => !object.is_empty(),
        serde_json::Value::String(value) => !value.trim().is_empty(),
        _ => true,
    }
}

fn collect_group_policy_gaps(user_groups: &[StoredUserGroup]) -> Vec<NifflerGroupPolicyGap> {
    let mut gaps = Vec::new();
    for group in user_groups {
        if !group
            .allowed_models_mode
            .trim()
            .eq_ignore_ascii_case("specific")
        {
            gaps.push(NifflerGroupPolicyGap {
                product_plan_id: group.id.clone(),
                product_plan_name: group.name.clone(),
                gap_kind: "unrestricted_models".to_string(),
                gap_label: "允许全部模型".to_string(),
                message: "这个用户分组当前允许全部模型；迁移为产品策略前需要确认是否继续开放全部模型。"
                    .to_string(),
                impact: "如果直接迁移，会变成一个可售模型范围很大的产品策略，用户可能看到不该开放的模型。".to_string(),
                recommended_action: "确认这个分组是否真的要开放全部模型；如果不是，先收敛为明确的可售模型列表。".to_string(),
            });
            if gaps.len() >= MAX_ISSUE_ITEMS {
                break;
            }
            continue;
        }
        if group
            .allowed_models
            .as_ref()
            .is_none_or(|models| models.is_empty())
        {
            gaps.push(NifflerGroupPolicyGap {
                product_plan_id: group.id.clone(),
                product_plan_name: group.name.clone(),
                gap_kind: "empty_specific_models".to_string(),
                gap_label: "指定模型为空".to_string(),
                message: "这个用户分组设置为只允许指定模型，但模型列表为空；迁移前需要明确可售模型。"
                    .to_string(),
                impact: "迁移后这个产品策略会没有可售模型，绑定到这个策略的用户 Key 将无法正常使用模型。".to_string(),
                recommended_action: "补齐可售模型列表，或把这个分组停用后再迁移。".to_string(),
            });
        }
        if gaps.len() >= MAX_ISSUE_ITEMS {
            break;
        }
    }
    gaps
}

fn collect_price_gaps(
    global_models: &[StoredAdminGlobalModel],
    provider_models: &[StoredAdminProviderModel],
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
) -> Vec<NifflerPriceGap> {
    let mut gaps = Vec::new();
    for model in global_models {
        if has_model_price(
            model.default_price_per_request,
            model.default_tiered_pricing.as_ref(),
        ) {
            continue;
        }
        gaps.push(NifflerPriceGap {
            scope: "global_model".to_string(),
            scope_label: "模型基础价格".to_string(),
            provider_id: None,
            provider_name: None,
            model_id: Some(model.id.clone()),
            model_name: model.name.clone(),
            missing_fields: vec![
                "default_price_per_request".to_string(),
                "default_tiered_pricing".to_string(),
            ],
            reason: "全局模型没有基础价格。".to_string(),
            impact:
                "钱包销售价和套餐消耗都依赖基础价格；缺少基础价格会导致迁移后的计费规则不明确。"
                    .to_string(),
            recommended_action:
                "按官方 API 最新定价补齐模型基础价格，再配置销售倍率或单模型覆盖价格。".to_string(),
        });
        if gaps.len() >= MAX_ISSUE_ITEMS {
            return gaps;
        }
    }
    for model in provider_models {
        let has_own_price = has_model_price(model.price_per_request, model.tiered_pricing.as_ref());
        let has_global_price = has_model_price(
            model.global_model_default_price_per_request,
            model.global_model_default_tiered_pricing.as_ref(),
        );
        if has_own_price || has_global_price {
            continue;
        }
        let provider = provider_map.get(model.provider_id.as_str());
        gaps.push(NifflerPriceGap {
            scope: "provider_model".to_string(),
            scope_label: "上游模型成本价格".to_string(),
            provider_id: Some(model.provider_id.clone()),
            provider_name: provider.map(|item| item.name.clone()),
            model_id: Some(model.id.clone()),
            model_name: model
                .global_model_name
                .clone()
                .unwrap_or_else(|| model.provider_model_name.clone()),
            missing_fields: vec![
                "price_per_request".to_string(),
                "tiered_pricing".to_string(),
            ],
            reason: "Provider 模型没有自身价格，也没有可继承的全局模型价格。".to_string(),
            impact: "迁移后无法计算这个上游模型的成本价，成本对账和账号池成本窗口都会不准确。".to_string(),
            recommended_action: "先补齐全局模型基础价格；如果这个 Provider 成本不同，再配置上游成本倍率或 Provider 模型价格。".to_string(),
        });
        if gaps.len() >= MAX_ISSUE_ITEMS {
            return gaps;
        }
    }
    gaps
}

fn has_model_price(
    price_per_request: Option<f64>,
    tiered_pricing: Option<&serde_json::Value>,
) -> bool {
    price_per_request.is_some_and(|price| price.is_finite() && price >= 0.0)
        || tiered_pricing
            .and_then(|value| value.get("tiers"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|tiers| {
                tiers.iter().any(|tier| {
                    [
                        "input_price_per_1m",
                        "output_price_per_1m",
                        "cache_creation_price_per_1m",
                        "cache_read_price_per_1m",
                    ]
                    .iter()
                    .any(|field| {
                        tier.get(*field)
                            .and_then(serde_json::Value::as_f64)
                            .is_some_and(|price| price.is_finite() && price >= 0.0)
                    })
                })
            })
}

async fn collect_recent_usage_anomalies(
    state: &AdminAppState<'_>,
    recent_days: u32,
    now_unix_secs: u64,
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
    key_map: &BTreeMap<&str, &StoredProviderCatalogKey>,
) -> Result<(Vec<NifflerUsageAnomaly>, u64), GatewayError> {
    if !state.has_usage_data_reader() {
        return Ok((Vec::new(), 0));
    }
    let from = now_unix_secs.saturating_sub(u64::from(recent_days) * 24 * 60 * 60);
    let rows = state
        .list_usage_audits(&UsageAuditListQuery {
            created_from_unix_secs: Some(from),
            created_until_unix_secs: Some(now_unix_secs),
            user_id: None,
            provider_name: None,
            model: None,
            api_format: None,
            statuses: None,
            is_stream: None,
            error_only: false,
            limit: Some(MAX_USAGE_SCAN),
            offset: Some(0),
            newest_first: true,
        })
        .await?;
    let mut anomalies = Vec::new();
    for row in rows {
        let Some(diagnosis) = usage_anomaly_diagnosis(&row) else {
            continue;
        };
        let key = row
            .provider_api_key_id
            .as_deref()
            .and_then(|key_id| key_map.get(key_id).copied());
        let provider_name = row
            .provider_id
            .as_deref()
            .and_then(|provider_id| provider_map.get(provider_id).copied())
            .map(|provider| provider.name.clone());
        let provider_display_name = provider_name
            .clone()
            .or_else(|| non_empty_string(&row.provider_name))
            .unwrap_or_else(|| "未选定上游".to_string());
        let provider_api_key_name = key
            .and_then(|key| non_empty_string(&key.name))
            .or_else(|| row.routing_key_name().map(ToOwned::to_owned));
        let package_debit_usd = row.settlement_package_debit_usd();
        let wallet_debit_usd = row.settlement_wallet_debit_usd();
        anomalies.push(NifflerUsageAnomaly {
            usage_id: row.id,
            request_id: row.request_id,
            created_at_unix_secs: unix_millis_or_secs_to_secs(row.created_at_unix_ms),
            provider_name: row.provider_name,
            provider_id: row.provider_id,
            provider_api_key_id: row.provider_api_key_id,
            provider_display_name,
            provider_api_key_name,
            provider_account_label: None,
            model: row.model,
            status: row.status,
            billing_status: row.billing_status,
            status_code: row.status_code,
            error_category: row.error_category,
            anomaly_kind: diagnosis.kind.to_string(),
            anomaly_label: diagnosis.label.to_string(),
            diagnosis: diagnosis.diagnosis.to_string(),
            impact: diagnosis.impact.to_string(),
            recommended_action: diagnosis.recommended_action.to_string(),
            total_cost_usd: row.total_cost_usd,
            actual_total_cost_usd: row.actual_total_cost_usd,
            package_debit_usd,
            wallet_debit_usd,
        });
        if anomalies.len() >= MAX_USAGE_ITEMS {
            break;
        }
    }
    let count = anomalies.len() as u64;
    Ok((anomalies, count))
}

struct UsageAnomalyDiagnosis {
    kind: &'static str,
    label: &'static str,
    diagnosis: &'static str,
    impact: &'static str,
    recommended_action: &'static str,
}

fn usage_anomaly_diagnosis(row: &StoredRequestUsageAudit) -> Option<UsageAnomalyDiagnosis> {
    let provider_unknown = row.provider_name.trim().eq_ignore_ascii_case("unknown")
        || row.provider_name.trim().is_empty()
        || row.provider_id.is_none();
    if provider_unknown && is_api_key_concurrency_limited(row) {
        return Some(UsageAnomalyDiagnosis {
            kind: "api_key_concurrency_limited",
            label: "平台并发拦截",
            diagnosis: "平台在选择上游前拦截了这个请求：用户 API Key 并发数已达上限，所以没有实际 Provider 或账号。",
            impact: "这类 unknown 不代表 Provider 丢失；请求没有进入上游，也不会消耗上游账号。",
            recommended_action: "检查用户 Key 的并发限制，或等待该用户的并发请求结束。",
        });
    }
    if provider_unknown {
        return Some(UsageAnomalyDiagnosis {
            kind: "provider_unknown",
            label: "未选定上游",
            diagnosis: "这条记录没有实际 Provider ID，失败发生在选定上游前，或旧记录没有保存可展示的上游服务。",
            impact: "管理员无法从使用记录直接定位上游账号，需要结合路由跳过原因判断是策略、额度、冷却还是配置问题。",
            recommended_action: "查看同页的路由跳过原因；如果是新近记录，需要优先修复调度前失败路径的错误记录归因。",
        });
    }
    if row.billing_status.trim().eq_ignore_ascii_case("pending")
        && row.status.trim().eq_ignore_ascii_case("completed")
    {
        return Some(UsageAnomalyDiagnosis {
            kind: "completed_billing_pending",
            label: "完成但未结算",
            diagnosis: "请求已完成，但结算没有最终完成；当前记录没有可展示的钱包扣费快照。",
            impact: "用户可能已经看到成功响应，但后台暂时无法确认套餐或钱包扣费是否完成。",
            recommended_action:
                "检查 usage 结算任务和 pending 清理任务；长期停留 pending 的记录需要进入人工对账。",
        });
    }
    if row.billing_status.trim().eq_ignore_ascii_case("pending") {
        return Some(UsageAnomalyDiagnosis {
            kind: "billing_pending",
            label: "等待结算",
            diagnosis: "请求仍在进行或等待超时清理，暂时没有最终扣费拆分。",
            impact: "这类记录在结算完成前不应用来判断最终扣费。",
            recommended_action: "等待请求结束或清理任务处理；超过预期时间仍 pending 时再人工检查。",
        });
    }
    if row.status.trim().eq_ignore_ascii_case("failed") && row.provider_api_key_id.is_none() {
        return Some(UsageAnomalyDiagnosis {
            kind: "failed_before_account_selected",
            label: "未选定账号失败",
            diagnosis:
                "这条失败记录没有上游账号 ID，说明失败发生在选定账号前或旧记录缺少账号快照。",
            impact: "管理员看不到具体账号，无法判断是哪个上游账号失败。",
            recommended_action:
                "结合路由跳过原因和请求错误信息定位；迁移后需要保证失败记录保存实际尝试链路。",
        });
    }
    let has_charge_snapshot =
        row.settlement_package_debit_usd().is_some() || row.settlement_wallet_debit_usd().is_some();
    if row.status.trim().eq_ignore_ascii_case("completed")
        && row.billing_status.trim().eq_ignore_ascii_case("settled")
        && row.total_cost_usd > 0.0
        && !has_charge_snapshot
    {
        return Some(UsageAnomalyDiagnosis {
            kind: "settled_without_charge_breakdown",
            label: "已结算但缺扣费拆分",
            diagnosis: "这条记录显示已结算且有销售金额，但没有套餐或钱包扣费拆分快照。",
            impact:
                "使用记录页面可能显示不出钱包扣款，管理员需要通过钱包流水或结算快照确认实际扣费。",
            recommended_action:
                "迁移结算快照前先对这类记录做只读对账；后续新结算必须强制写入扣费拆分。",
        });
    }
    None
}

fn is_api_key_concurrency_limited(row: &StoredRequestUsageAudit) -> bool {
    row.error_message
        .as_deref()
        .is_some_and(|message| message.contains("API Key 并发请求数已达上限"))
        || row
            .routing_local_execution_runtime_miss_reason()
            .is_some_and(|reason| reason == "api_key_concurrency_limit_reached")
        || row
            .routing_execution_path()
            .is_some_and(|path| path == "local_api_key_concurrency_limited")
}

async fn collect_route_skip_reports(
    state: &AdminAppState<'_>,
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
    key_map: &BTreeMap<&str, &StoredProviderCatalogKey>,
) -> Result<
    (
        Vec<NifflerRouteSkipReasonSummary>,
        Vec<NifflerRouteSkipSample>,
    ),
    GatewayError,
> {
    if !state.has_request_candidate_data_reader() {
        return Ok((Vec::new(), Vec::new()));
    }
    let rows = state
        .read_recent_request_candidates(MAX_ROUTE_SKIP_SAMPLE)
        .await?;
    let mut counts = BTreeMap::<String, u64>::new();
    let mut samples = Vec::new();
    for row in rows {
        let Some(reason) = row
            .skip_reason
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
        else {
            continue;
        };
        *counts.entry(reason.to_string()).or_default() += 1;
        if samples.len() < MAX_ISSUE_ITEMS {
            samples.push(route_skip_sample(&row, reason, provider_map, key_map));
        }
    }
    let mut summaries = counts
        .into_iter()
        .map(|(reason, count)| {
            let info = route_skip_reason_info(&reason);
            NifflerRouteSkipReasonSummary {
                reason,
                label: info.label.to_string(),
                category: info.category.to_string(),
                count,
                impact: info.impact.to_string(),
                recommended_action: info.recommended_action.to_string(),
            }
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.reason.cmp(&right.reason))
    });
    summaries.truncate(MAX_ISSUE_ITEMS);
    Ok((summaries, samples))
}

fn route_skip_sample(
    row: &StoredRequestCandidate,
    reason: &str,
    provider_map: &BTreeMap<&str, &StoredProviderCatalogProvider>,
    key_map: &BTreeMap<&str, &StoredProviderCatalogKey>,
) -> NifflerRouteSkipSample {
    let provider = row
        .provider_id
        .as_deref()
        .and_then(|provider_id| provider_map.get(provider_id).copied());
    let key = row
        .key_id
        .as_deref()
        .and_then(|key_id| key_map.get(key_id).copied());
    let info = route_skip_reason_info(reason);
    NifflerRouteSkipSample {
        request_id: row.request_id.clone(),
        created_at_unix_secs: unix_millis_or_secs_to_secs(row.created_at_unix_ms),
        provider_id: row.provider_id.clone(),
        provider_name: provider.map(|provider| provider.name.clone()),
        key_id: row.key_id.clone(),
        key_name: key.and_then(|key| non_empty_string(&key.name)),
        account_label: None,
        reason: reason.to_string(),
        label: info.label.to_string(),
        impact: info.impact.to_string(),
        recommended_action: info.recommended_action.to_string(),
    }
}

async fn load_account_labels_for_readiness(
    state: &AdminAppState<'_>,
    key_scope_residue: &[NifflerKeyScopeResidue],
    usage_anomalies: &[NifflerUsageAnomaly],
    route_skip_samples: &[NifflerRouteSkipSample],
) -> Result<BTreeMap<String, String>, GatewayError> {
    if !state.has_provider_catalog_data_reader() {
        return Ok(BTreeMap::new());
    }
    let mut key_ids = BTreeSet::new();
    key_ids.extend(key_scope_residue.iter().map(|item| item.key_id.clone()));
    key_ids.extend(
        usage_anomalies
            .iter()
            .filter_map(|item| item.provider_api_key_id.clone()),
    );
    key_ids.extend(
        route_skip_samples
            .iter()
            .filter_map(|item| item.key_id.clone()),
    );
    if key_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let key_ids = key_ids.into_iter().collect::<Vec<_>>();
    let keys = state.list_provider_catalog_keys_by_ids(&key_ids).await?;
    Ok(keys
        .into_iter()
        .filter_map(|key| {
            let label = provider_key_account_label(state, &key)?;
            Some((key.id, label))
        })
        .collect())
}

fn apply_account_labels_to_key_residue(
    items: &mut [NifflerKeyScopeResidue],
    labels: &BTreeMap<String, String>,
) {
    for item in items {
        let Some(label) = labels.get(&item.key_id).cloned() else {
            continue;
        };
        item.account_label = Some(label.clone());
        item.display_name = label;
    }
}

fn apply_account_labels_to_usage_anomalies(
    items: &mut [NifflerUsageAnomaly],
    labels: &BTreeMap<String, String>,
) {
    for item in items {
        let Some(key_id) = item.provider_api_key_id.as_deref() else {
            continue;
        };
        item.provider_account_label = labels.get(key_id).cloned();
    }
}

fn apply_account_labels_to_route_skip_samples(
    items: &mut [NifflerRouteSkipSample],
    labels: &BTreeMap<String, String>,
) {
    for item in items {
        let Some(key_id) = item.key_id.as_deref() else {
            continue;
        };
        item.account_label = labels.get(key_id).cloned();
    }
}

struct RouteSkipReasonInfo {
    label: &'static str,
    category: &'static str,
    impact: &'static str,
    recommended_action: &'static str,
}

fn route_skip_reason_info(reason: &str) -> RouteSkipReasonInfo {
    match reason {
        "pool_cooldown" => RouteSkipReasonInfo {
            label: "账号冷却中",
            category: "账号状态",
            impact: "调度器不会选择冷却中的上游账号。",
            recommended_action: "等待冷却结束；如果频繁出现，检查上游错误、冷却时间和账号状态。",
        },
        "pool_account_blocked" => RouteSkipReasonInfo {
            label: "账号被阻止调度",
            category: "账号状态",
            impact: "这把上游账号当前不会参与调度。",
            recommended_action: "检查账号是否被手动停用、风控暂停或标记为不可调度。",
        },
        "pool_account_exhausted" => RouteSkipReasonInfo {
            label: "账号额度耗尽",
            category: "账号额度",
            impact: "调度器会跳过额度耗尽的账号。",
            recommended_action: "等待额度周期重置，或补充可用账号后再处理这个服务。",
        },
        "pool_temporary_unavailable" => RouteSkipReasonInfo {
            label: "账号暂不可用",
            category: "账号状态",
            impact: "账号最近健康检查或调度反馈不可用，暂时不会被选择。",
            recommended_action: "查看账号测试结果和最近上游错误；确认恢复后再让账号参与调度。",
        },
        "pool_cost_limit_reached" => RouteSkipReasonInfo {
            label: "成本窗口超限",
            category: "成本控制",
            impact: "账号在当前成本窗口内达到限制，调度器会跳过它。",
            recommended_action: "检查账号成本窗口、上游成本倍率和模型价格配置。",
        },
        "key_inactive" => RouteSkipReasonInfo {
            label: "账号已停用",
            category: "账号状态",
            impact: "停用账号不会参与调度。",
            recommended_action:
                "如果这个账号仍要使用，在账号管理里恢复；否则从策略或账号池里移除。",
        },
        "oauth_invalid" => RouteSkipReasonInfo {
            label: "OAuth 已失效",
            category: "账号状态",
            impact: "OAuth 失效账号不会参与调度。",
            recommended_action: "重新登录这个 OAuth 账号，或移除失效账号。",
        },
        "key_model_disabled" => RouteSkipReasonInfo {
            label: "账号不允许该模型",
            category: "模型能力",
            impact: "这把账号自己的模型限制排除了本次请求模型。",
            recommended_action: "把模型能力迁到统一账号能力里，确认这个账号是否确实支持该模型。",
        },
        "api_key_concurrency_limit_reached" => RouteSkipReasonInfo {
            label: "用户 Key 并发已满",
            category: "平台限制",
            impact: "请求在选择上游前被平台并发限制拦截。",
            recommended_action: "调整用户 Key 并发限制，或等待该用户正在运行的请求结束。",
        },
        "provider_key_concurrency_limit_reached" => RouteSkipReasonInfo {
            label: "上游账号并发已满",
            category: "账号限制",
            impact: "这把上游账号达到并发限制，调度器会尝试其他可用账号。",
            recommended_action: "检查账号并发配置；如果经常满载，需要增加账号或调整调度权重。",
        },
        "routing_profile_disallowed_key" => RouteSkipReasonInfo {
            label: "产品策略不允许这把账号",
            category: "策略限制",
            impact: "当前用户 Key 绑定的策略不允许使用这把上游账号。",
            recommended_action: "检查产品策略、可用服务和账号范围配置是否符合预期。",
        },
        "transport_snapshot_missing" => RouteSkipReasonInfo {
            label: "连接配置缺失",
            category: "配置缺失",
            impact: "缺少执行请求所需的上游连接信息，无法发起上游请求。",
            recommended_action: "检查 Provider、端点、账号密钥和认证配置是否完整。",
        },
        "pool_active_probe_sealed" => RouteSkipReasonInfo {
            label: "未进入探测热池",
            category: "账号池策略",
            impact: "开启主动探测保护后，未进入热池的账号不会被本次请求选择。",
            recommended_action: "等待探测补充热池；如果长期不足，检查主动探测配置和账号健康状态。",
        },
        "transport_unsupported" => RouteSkipReasonInfo {
            label: "协议不支持",
            category: "协议能力",
            impact: "这个上游服务不支持本次请求需要的协议或能力。",
            recommended_action: "检查 Provider 支持的 API 格式、模型能力和请求类型是否匹配。",
        },
        "transport_api_format_mismatch" => RouteSkipReasonInfo {
            label: "API 格式不匹配",
            category: "协议能力",
            impact: "这次请求的 API 格式和上游账号支持的格式不一致。",
            recommended_action: "检查上游账号支持的 API 格式；必要时启用正确的格式或选择其他服务。",
        },
        "format_conversion_disabled" => RouteSkipReasonInfo {
            label: "格式转换未启用",
            category: "协议能力",
            impact: "请求需要格式转换，但当前服务或账号没有启用对应转换。",
            recommended_action:
                "确认是否允许转换；如果不允许，为这个请求类型配置原生支持的上游服务。",
        },
        _ => RouteSkipReasonInfo {
            label: "未归类跳过原因",
            category: "未归类",
            impact: "后台保留了原始跳过代码，但还没有对应的中文说明。",
            recommended_action:
                "保留原始代码并检查路由记录；如果反复出现，把这个原因补入对账说明。",
        },
    }
}

fn source_field_label(source_field: &str) -> &'static str {
    match source_field {
        "allowed_providers" => "可用 Provider",
        _ => "未知字段",
    }
}

fn residue_field_label(field: &str) -> &'static str {
    match field {
        "api_formats" => "API 格式限制",
        "auth_type_by_format" => "按格式认证方式",
        "allow_auth_channel_mismatch_formats" => "允许认证通道不一致",
        "rate_multipliers" => "成本/倍率覆盖",
        "global_priority_by_format" => "按格式优先级",
        "allowed_models" => "允许模型",
        "locked_models" => "锁定模型",
        "model_include_patterns" => "模型包含规则",
        "model_exclude_patterns" => "模型排除规则",
        _ => "未归类字段",
    }
}

fn provider_key_account_label(
    state: &AdminAppState<'_>,
    key: &StoredProviderCatalogKey,
) -> Option<String> {
    let auth_config = parse_catalog_auth_config_json(state.app(), key);
    provider_key_account_label_from_auth_config(auth_config.as_ref())
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn unix_millis_or_secs_to_secs(value: u64) -> u64 {
    if value > 10_000_000_000 {
        value / 1000
    } else {
        value
    }
}

fn provider_status_counts(providers: &[StoredProviderCatalogProvider]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for provider in providers {
        let status = if provider.is_active {
            "active"
        } else {
            "disabled"
        };
        *counts.entry(status.to_string()).or_default() += 1;
    }
    counts
}

fn account_status_counts(keys: &[StoredProviderCatalogKey]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for key in keys {
        *counts.entry(key_status_label(key).to_string()).or_default() += 1;
    }
    counts
}

fn key_status_label(key: &StoredProviderCatalogKey) -> &'static str {
    if !key.is_active {
        "disabled"
    } else if key.oauth_invalid_at_unix_secs.is_some() {
        "invalid"
    } else {
        "available"
    }
}

fn collect_issues(
    state: &AdminAppState<'_>,
    shadow_tables: &NifflerShadowTableStatus,
    disabled_provider_references: &[NifflerDisabledProviderReference],
    key_scope_residue: &[NifflerKeyScopeResidue],
    group_policy_gaps: &[NifflerGroupPolicyGap],
    price_gaps: &[NifflerPriceGap],
    usage_anomalies: &[NifflerUsageAnomaly],
) -> Vec<NifflerReadinessIssue> {
    let mut issues = Vec::new();
    if !shadow_tables.all_present {
        issues.push(issue(
            NifflerReadinessSeverity::Error,
            "shadow_tables_missing",
            "影子表不完整",
            "新模型影子表没有全部创建，不能进入后续迁移。",
        ));
    }
    if !state.has_provider_catalog_data_reader() {
        issues.push(issue(
            NifflerReadinessSeverity::Error,
            "provider_reader_missing",
            "Provider 数据不可读",
            "后台无法读取旧 Provider 和上游账号数据。",
        ));
    }
    if !state.has_global_model_data_reader() {
        issues.push(issue(
            NifflerReadinessSeverity::Error,
            "model_reader_missing",
            "模型数据不可读",
            "后台无法读取模型和价格数据。",
        ));
    }
    if !disabled_provider_references.is_empty() {
        issues.push(issue(
            NifflerReadinessSeverity::Warning,
            "disabled_provider_referenced",
            "停用 Provider 仍被分组引用",
            "用户分组里仍引用了停用 Provider，迁移后这些 Provider 不能被产品策略选择。",
        ));
    }
    if !key_scope_residue.is_empty() {
        issues.push(issue(
            NifflerReadinessSeverity::Warning,
            "key_scope_residue",
            "Key 仍有独立限制",
            "部分上游账号还有模型、格式或优先级限制，需要归入新账号能力或调度策略。",
        ));
    }
    if !group_policy_gaps.is_empty() {
        issues.push(issue(
            NifflerReadinessSeverity::Warning,
            "group_policy_gaps",
            "分组策略需要确认",
            "部分用户分组存在全部模型开放或指定模型列表为空，迁移为产品策略前需要确认。",
        ));
    }
    if !price_gaps.is_empty() {
        issues.push(issue(
            NifflerReadinessSeverity::Warning,
            "price_gaps",
            "价格配置不完整",
            "部分模型没有可用的基础价或 Provider 模型价格，迁移计费前需要补齐。",
        ));
    }
    if !usage_anomalies.is_empty() {
        issues.push(issue(
            NifflerReadinessSeverity::Warning,
            "usage_anomalies",
            "请求记录仍有旧字段问题",
            "最近请求记录里还有 unknown、账号缺失或 pending 结算记录。",
        ));
    }
    issues
}

fn issue(
    severity: NifflerReadinessSeverity,
    code: &str,
    title: &str,
    message: &str,
) -> NifflerReadinessIssue {
    NifflerReadinessIssue {
        severity,
        code: code.to_string(),
        title: title.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_recent_days;

    #[test]
    fn recent_days_is_bounded() {
        assert_eq!(parse_recent_days(Some("recent_days=30")), 30);
        assert_eq!(parse_recent_days(Some("recent_days=0")), 7);
        assert_eq!(parse_recent_days(Some("recent_days=91")), 7);
        assert_eq!(parse_recent_days(None), 7);
    }
}
