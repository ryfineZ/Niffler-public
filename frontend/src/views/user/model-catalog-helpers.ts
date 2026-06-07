import type { PublicGlobalModel } from '@/api/public-models'

export function getCapabilityNames(model: Pick<PublicGlobalModel, 'supported_capabilities'>): string[] {
  const capabilities = model.supported_capabilities
  if (Array.isArray(capabilities)) {
    return capabilities
      .map(capability => String(capability).trim())
      .filter(Boolean)
  }
  if (capabilities && typeof capabilities === 'object') {
    return Object.entries(capabilities)
      .filter(([, enabled]) => enabled === true)
      .map(([capability]) => capability.trim())
      .filter(Boolean)
  }
  return []
}

export function supportsEmbedding(model: PublicGlobalModel): boolean {
  const capabilities = getCapabilityNames(model)
  return model.supports_embedding === true
    || capabilities.includes('embedding')
    || model.config?.embedding === true
    || model.config?.model_type === 'embedding'
    || (Array.isArray(model.config?.api_formats) && model.config.api_formats.some((format) => String(format).endsWith(':embedding')))
}

export function supportsRerank(model: PublicGlobalModel): boolean {
  return getCapabilityNames(model).includes('rerank')
    || model.config?.rerank === true
    || model.config?.model_type === 'rerank'
    || (Array.isArray(model.config?.api_formats) && model.config.api_formats.some((format) => String(format).endsWith(':rerank')))
}

export function hasVideoPricing(model: PublicGlobalModel): boolean {
  const billing = model.config?.billing
  const video = billing && typeof billing === 'object' && !Array.isArray(billing)
    ? (billing as Record<string, unknown>).video
    : null
  const priceByResolution = video && typeof video === 'object' && !Array.isArray(video)
    ? (video as Record<string, unknown>).price_per_second_by_resolution
    : null
  return !!priceByResolution && typeof priceByResolution === 'object' && Object.keys(priceByResolution).length > 0
}

export function getModelCapabilityLabels(model: PublicGlobalModel): string[] {
  const labels: string[] = []
  if (supportsRerank(model)) {
    labels.push('Rerank')
  } else if (supportsEmbedding(model)) {
    labels.push('Embedding')
  } else {
    labels.push('Chat')
  }
  if (model.config?.image_generation === true) labels.push('Image')
  if (hasVideoPricing(model)) labels.push('Video')
  return labels
}
