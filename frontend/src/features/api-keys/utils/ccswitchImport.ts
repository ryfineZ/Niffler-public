export type CcSwitchApp = 'claude' | 'codex' | 'gemini'

interface BuildCcSwitchImportUrlInput {
  app: CcSwitchApp
  baseUrl: string
  providerName: string
  apiKey: string
  model?: string
}

export function normalizeCcSwitchBaseUrl(baseUrl: string): string {
  return baseUrl.trim().replace(/\/+$/, '')
}

export function ccSwitchEndpoint(app: CcSwitchApp, baseUrl: string): string {
  const normalizedBaseUrl = normalizeCcSwitchBaseUrl(baseUrl)
  return app === 'codex' ? `${normalizedBaseUrl}/v1` : normalizedBaseUrl
}

export function buildCcSwitchUsageScript(model?: string): string {
  const modelQuery = model?.trim()
    ? `?model=${encodeURIComponent(model.trim())}`
    : ''

  return `({
    request: {
      url: "{{baseUrl}}/v1/usage${modelQuery}",
      method: "GET",
      headers: { "Authorization": "Bearer {{apiKey}}" }
    },
    extractor: function(response) {
      const remaining = response?.remaining ?? response?.quota?.remaining ?? response?.balance;
      const unit = response?.unit ?? response?.quota?.unit ?? "USD";
      return {
        isValid: response?.is_active ?? response?.isValid ?? true,
        remaining,
        unit
      };
    }
  })`
}

export function buildCcSwitchImportUrl(input: BuildCcSwitchImportUrlInput): string {
  const baseUrl = normalizeCcSwitchBaseUrl(input.baseUrl)
  const entries: [string, string][] = [
    ['resource', 'provider'],
    ['app', input.app],
    ['name', input.providerName.trim() || 'Niffler'],
    ['homepage', baseUrl],
    ['endpoint', ccSwitchEndpoint(input.app, baseUrl)],
    ['apiKey', input.apiKey],
    ['enabled', 'true'],
    ['configFormat', 'json'],
    ['usageEnabled', 'true'],
    ['usageBaseUrl', baseUrl],
    ['usageScript', encodeBase64(buildCcSwitchUsageScript(input.model))],
    ['usageAutoInterval', '30'],
  ]

  if (input.model?.trim()) {
    entries.splice(2, 0, ['model', input.model.trim()])
  }

  return `ccswitch://v1/import?${new URLSearchParams(entries).toString()}`
}

function encodeBase64(value: string): string {
  if (typeof btoa === 'function') {
    return btoa(value)
  }

  return Buffer.from(value, 'utf8').toString('base64')
}
