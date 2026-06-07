import type { BillingEntitlement, DailyQuotaEntitlement } from '@/api/billing'

type LegacyUsageEntitlements = {
  limits?: DailyQuotaEntitlement['limits']
}

export type BillingEntitlementsInput = BillingEntitlement[] | LegacyUsageEntitlements | null | undefined

export function normalizeBillingEntitlements(input: BillingEntitlementsInput): BillingEntitlement[] {
  if (Array.isArray(input)) return input
  if (!input || typeof input !== 'object') return []

  if ('limits' in input && input.limits && typeof input.limits === 'object') {
    return [
      {
        type: 'daily_quota',
        limits: input.limits,
      },
    ]
  }

  return []
}

export function hasPackageBillingEntitlement(input: BillingEntitlementsInput): boolean {
  return normalizeBillingEntitlements(input).some((item) =>
    item.type === 'daily_quota'
    || item.type === 'membership_group'
    || Boolean((item as DailyQuotaEntitlement).limits)
  )
}
