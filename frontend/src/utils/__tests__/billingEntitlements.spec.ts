import { describe, expect, it } from 'vitest'

import { hasPackageBillingEntitlement, normalizeBillingEntitlements } from '../billingEntitlements'

describe('billingEntitlements', () => {
  it('normalizes legacy synced usage object into a displayable entitlement array', () => {
    const entitlements = normalizeBillingEntitlements({
      limits: {
        five_hour_limit_usd: '200.00000000',
        weekly_limit_usd: '2000.00000000',
        rpm_limit: 8,
      },
      source: 'sub2api',
    } as never)

    expect(entitlements).toEqual([
      {
        type: 'daily_quota',
        limits: {
          five_hour_limit_usd: '200.00000000',
          weekly_limit_usd: '2000.00000000',
          rpm_limit: 8,
        },
      },
    ])
    expect(hasPackageBillingEntitlement(entitlements)).toBe(true)
  })
})
