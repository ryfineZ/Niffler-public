import apiClient from './client'

export type NifflerReadinessSeverity = 'info' | 'warning' | 'error'

export interface NifflerShadowTableItem {
  table_name: string
  exists: boolean
}

export interface NifflerShadowTableStatus {
  database_driver?: string | null
  expected_tables: number
  existing_tables: number
  all_present: boolean
  tables: NifflerShadowTableItem[]
}

export interface NifflerCoreReadinessSummary {
  providers_total: number
  providers_active: number
  provider_keys_total: number
  provider_keys_active: number
  product_plans_total: number
  product_plans_public: number
  global_models_total: number
  global_models_active: number
  recent_problem_usage_sample_count: number
}

export interface NifflerCoreMappingSummary {
  legacy_count: number
  mapped_count: number
  blocked_count: number
  notes: string[]
}

export interface NifflerDisabledProviderReference {
  product_plan_id: string
  product_plan_name: string
  provider_id: string
  provider_name: string
  source_field: string
  source_field_label: string
  reason: string
  impact: string
  recommended_action: string
}

export interface NifflerKeyScopeResidue {
  subject_kind: string
  key_id: string
  key_name?: string | null
  owner_label?: string | null
  display_name: string
  provider_id?: string | null
  provider_name?: string | null
  account_label?: string | null
  residue_fields: string[]
  field_labels: string[]
  reason: string
  impact: string
  recommended_action: string
}

export interface NifflerGroupPolicyGap {
  product_plan_id: string
  product_plan_name: string
  gap_kind: string
  gap_label: string
  message: string
  impact: string
  recommended_action: string
}

export interface NifflerPriceGap {
  scope: string
  scope_label: string
  provider_id?: string | null
  provider_name?: string | null
  model_id?: string | null
  model_name: string
  missing_fields: string[]
  reason: string
  impact: string
  recommended_action: string
}

export interface NifflerUsageAnomaly {
  usage_id: string
  request_id: string
  created_at_unix_secs: number
  provider_name: string
  provider_id?: string | null
  provider_api_key_id?: string | null
  provider_display_name: string
  provider_api_key_name?: string | null
  provider_account_label?: string | null
  model: string
  status: string
  billing_status: string
  status_code?: number | null
  error_category?: string | null
  anomaly_kind: string
  anomaly_label: string
  diagnosis: string
  impact: string
  recommended_action: string
  total_cost_usd: number
  actual_total_cost_usd: number
  package_debit_usd?: number | null
  wallet_debit_usd?: number | null
}

export interface NifflerRouteSkipReasonSummary {
  reason: string
  label: string
  category: string
  count: number
  impact: string
  recommended_action: string
}

export interface NifflerRouteSkipSample {
  request_id: string
  created_at_unix_secs: number
  provider_id?: string | null
  provider_name?: string | null
  key_id?: string | null
  key_name?: string | null
  account_label?: string | null
  reason: string
  label: string
  impact: string
  recommended_action: string
}

export interface NifflerReadinessIssue {
  severity: NifflerReadinessSeverity
  code: string
  title: string
  message: string
}

export interface NifflerCoreReadinessReport {
  schema_version: number
  generated_at_unix_secs: number
  recent_days: number
  shadow_tables: NifflerShadowTableStatus
  summary: NifflerCoreReadinessSummary
  provider_mapping: NifflerCoreMappingSummary
  account_mapping: NifflerCoreMappingSummary
  product_plan_mapping: NifflerCoreMappingSummary
  provider_status_counts: Record<string, number>
  account_status_counts: Record<string, number>
  disabled_provider_references: NifflerDisabledProviderReference[]
  key_scope_residue: NifflerKeyScopeResidue[]
  group_policy_gaps: NifflerGroupPolicyGap[]
  price_gaps: NifflerPriceGap[]
  recent_usage_anomalies: NifflerUsageAnomaly[]
  route_skip_reasons: NifflerRouteSkipReasonSummary[]
  route_skip_samples: NifflerRouteSkipSample[]
  issues: NifflerReadinessIssue[]
}

export async function getNifflerCoreReadiness(params?: {
  recent_days?: number
}): Promise<NifflerCoreReadinessReport> {
  const response = await apiClient.get<NifflerCoreReadinessReport>(
    '/api/admin/niffler-core/readiness',
    { params }
  )
  return response.data
}
