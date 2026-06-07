export interface AccountDisplaySource {
  name?: string | null
  key_name?: string | null
  oauth_email?: string | null
  oauth_phone?: string | null
  phone?: string | null
  phone_number?: string | null
  mobile?: string | null
}

function normalizeAccountText(value: unknown): string | null {
  const normalized = String(value ?? '').trim()
  return normalized || null
}

export function getAccountDisplayParts(source: AccountDisplaySource): string[] {
  const values = [
    normalizeAccountText(source.oauth_email),
    normalizeAccountText(source.oauth_phone),
    normalizeAccountText(source.phone),
    normalizeAccountText(source.phone_number),
    normalizeAccountText(source.mobile),
  ]
  return [...new Set(values.filter((value): value is string => Boolean(value)))]
}

export function getAccountDisplayName(source: AccountDisplaySource, fallback = '未命名账号'): string {
  const parts = getAccountDisplayParts(source)
  if (parts.length > 0) return parts.join(' / ')
  return normalizeAccountText(source.key_name) || normalizeAccountText(source.name) || fallback
}

export function getAccountCopyText(source: AccountDisplaySource): string | null {
  const parts = getAccountDisplayParts(source)
  if (parts.length > 0) return parts.join('\n')
  return normalizeAccountText(source.key_name) || normalizeAccountText(source.name)
}
