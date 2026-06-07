const AUTO_QUOTA_REFRESH_COOLDOWN_SECONDS = 5 * 60

const lastAutoQuotaRefreshAttemptAtByProvider = new Map<string, number>()

function normalizeUnixSeconds(value: number): number {
  return Math.max(Math.floor(value), 0)
}

export function isProviderQuotaAutoRefreshCoolingDown(
  providerId: string | null | undefined,
  nowSeconds = Math.floor(Date.now() / 1000),
): boolean {
  const id = String(providerId || '').trim()
  if (!id) return false

  const lastAttemptAt = lastAutoQuotaRefreshAttemptAtByProvider.get(id)
  if (lastAttemptAt == null) return false

  return normalizeUnixSeconds(nowSeconds) - lastAttemptAt < AUTO_QUOTA_REFRESH_COOLDOWN_SECONDS
}

export function markProviderQuotaAutoRefreshAttempt(
  providerId: string | null | undefined,
  nowSeconds = Math.floor(Date.now() / 1000),
): void {
  const id = String(providerId || '').trim()
  if (!id) return

  lastAutoQuotaRefreshAttemptAtByProvider.set(id, normalizeUnixSeconds(nowSeconds))
}

export function resetProviderQuotaAutoRefreshCooldownForTests(): void {
  lastAutoQuotaRefreshAttemptAtByProvider.clear()
}

export { AUTO_QUOTA_REFRESH_COOLDOWN_SECONDS }
