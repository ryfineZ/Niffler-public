import { afterEach, describe, expect, it } from 'vitest'

import {
  AUTO_QUOTA_REFRESH_COOLDOWN_SECONDS,
  isProviderQuotaAutoRefreshCoolingDown,
  markProviderQuotaAutoRefreshAttempt,
  resetProviderQuotaAutoRefreshCooldownForTests,
} from '../quotaAutoRefreshCooldown'

describe('quota auto refresh cooldown', () => {
  afterEach(() => {
    resetProviderQuotaAutoRefreshCooldownForTests()
  })

  it('starts cooldown after recording an auto refresh attempt', () => {
    markProviderQuotaAutoRefreshAttempt('provider-1', 1_000)

    expect(isProviderQuotaAutoRefreshCoolingDown('provider-1', 1_000)).toBe(true)
    expect(
      isProviderQuotaAutoRefreshCoolingDown(
        'provider-1',
        1_000 + AUTO_QUOTA_REFRESH_COOLDOWN_SECONDS - 1,
      ),
    ).toBe(true)
  })

  it('expires cooldown after the configured window', () => {
    markProviderQuotaAutoRefreshAttempt('provider-1', 1_000)

    expect(
      isProviderQuotaAutoRefreshCoolingDown(
        'provider-1',
        1_000 + AUTO_QUOTA_REFRESH_COOLDOWN_SECONDS,
      ),
    ).toBe(false)
  })

  it('tracks cooldown independently per provider', () => {
    markProviderQuotaAutoRefreshAttempt('provider-1', 1_000)
    markProviderQuotaAutoRefreshAttempt('provider-2', 1_200)

    expect(isProviderQuotaAutoRefreshCoolingDown('provider-1', 1_301)).toBe(false)
    expect(isProviderQuotaAutoRefreshCoolingDown('provider-2', 1_301)).toBe(true)
  })

  it('ignores empty provider ids', () => {
    markProviderQuotaAutoRefreshAttempt('', 1_000)

    expect(isProviderQuotaAutoRefreshCoolingDown('', 1_001)).toBe(false)
    expect(isProviderQuotaAutoRefreshCoolingDown(null, 1_001)).toBe(false)
  })
})
