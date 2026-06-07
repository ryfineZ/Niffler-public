import { beforeEach, describe, expect, it, vi } from 'vitest'

const apiClientMocks = vi.hoisted(() => ({
  get: vi.fn(),
}))

vi.mock('@/api/client', () => ({
  default: apiClientMocks,
}))

describe('useSiteInfo', () => {
  beforeEach(() => {
    vi.resetModules()
    apiClientMocks.get.mockReset()
  })

  it('uses Niffler as the default site name before public info loads', async () => {
    apiClientMocks.get.mockRejectedValue(new Error('network unavailable'))

    const { useSiteInfo } = await import('../useSiteInfo')
    const { siteName } = useSiteInfo()

    expect(siteName.value).toBe('Niffler')
  })

  it('loads public site info', async () => {
    apiClientMocks.get.mockResolvedValue({
      data: {
        site_name: 'Custom Niffler',
        site_subtitle: 'Gateway',
      },
    })

    const { useSiteInfo } = await import('../useSiteInfo')
    const { siteName, siteSubtitle, refreshSiteInfo } = useSiteInfo()
    await refreshSiteInfo()

    expect(siteName.value).toBe('Custom Niffler')
    expect(siteSubtitle.value).toBe('Gateway')
  })
})
