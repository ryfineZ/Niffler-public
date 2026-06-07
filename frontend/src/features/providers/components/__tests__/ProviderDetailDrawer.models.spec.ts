import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, defineComponent, h, nextTick, reactive, type App } from 'vue'
import ProviderDetailDrawer from '@/features/providers/components/ProviderDetailDrawer.vue'

const endpointMocks = vi.hoisted(() => ({
  getProvider: vi.fn(),
  getProviderEndpoints: vi.fn(),
  getProviderModels: vi.fn(),
  getProviderMappingPreview: vi.fn(),
  getProviderKeysPage: vi.fn(),
  sortApiFormats: vi.fn((formats: string[]) => formats),
}))

vi.mock('@/api/endpoints', () => ({
  getProvider: endpointMocks.getProvider,
  getProviderEndpoints: endpointMocks.getProviderEndpoints,
  getProviderModels: endpointMocks.getProviderModels,
  getProviderMappingPreview: endpointMocks.getProviderMappingPreview,
  getProviderKeysPage: endpointMocks.getProviderKeysPage,
  sortApiFormats: endpointMocks.sortApiFormats,
  API_FORMAT_ORDER: ['openai:chat', 'openai:responses', 'claude:messages'],
  updateProvider: vi.fn(),
  deleteEndpointKey: vi.fn(),
  recoverKeyHealth: vi.fn(),
  updateProviderKey: vi.fn(),
  revealEndpointKey: vi.fn(),
  exportKey: vi.fn(),
  refreshProviderOAuth: vi.fn(),
  refreshProviderQuota: vi.fn(),
  clearOAuthInvalid: vi.fn(),
}))

vi.mock('@/api/admin', () => ({
  adminApi: {
    getSystemConfig: vi.fn().mockResolvedValue({ value: false }),
  },
}))

vi.mock('@/components/ui/button.vue', () => ({
  default: defineComponent({
    name: 'ButtonStub',
    setup(_, { slots }) {
      return () => h('button', slots.default?.())
    },
  }),
}))

vi.mock('@/components/ui/card.vue', () => ({
  default: defineComponent({
    name: 'CardStub',
    setup(_, { slots }) {
      return () => h('section', slots.default?.())
    },
  }),
}))

vi.mock('@/components/ui/badge.vue', () => ({
  default: defineComponent({
    name: 'BadgeStub',
    setup(_, { slots }) {
      return () => h('span', slots.default?.())
    },
  }),
}))

vi.mock('@/components/ui', () => {
  const passthrough = (name: string) => defineComponent({
    name,
    setup(_, { slots }) {
      return () => h('div', slots.default?.())
    },
  })
  return {
    Popover: passthrough('PopoverStub'),
    PopoverTrigger: passthrough('PopoverTriggerStub'),
    PopoverContent: passthrough('PopoverContentStub'),
  }
})

vi.mock('@/features/providers/components', () => {
  const EmptyStub = defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  })
  const ModelsTab = defineComponent({
    name: 'ModelsTabStub',
    props: {
      provider: { type: Object, required: true },
      models: { type: Array, default: () => [] },
      loading: Boolean,
    },
    setup(props) {
      return () => h('div', {
        'data-testid': 'models-tab',
        'data-provider-id': (props.provider as { id: string }).id,
        'data-models': (props.models as Array<{ provider_model_name?: string }>)
          .map(model => model.provider_model_name)
          .join(','),
        'data-loading': String(props.loading),
      })
    },
  })
  return {
    KeyFormDialog: EmptyStub,
    KeyAllowedModelsDialog: EmptyStub,
    KeyAllowedModelsEditDialog: EmptyStub,
    ModelsTab,
    BatchAssignModelsDialog: EmptyStub,
    OAuthAccountDialog: EmptyStub,
    OAuthKeyEditDialog: EmptyStub,
  }
})

vi.mock('@/features/providers/components/provider-tabs/ModelMappingTab.vue', () => ({
  default: defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  }),
}))

vi.mock('@/features/providers/components/EndpointFormDialog.vue', () => ({
  default: defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  }),
}))

vi.mock('@/features/providers/components/ProviderModelFormDialog.vue', () => ({
  default: defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  }),
}))

vi.mock('@/components/common/AlertDialog.vue', () => ({
  default: defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  }),
}))

vi.mock('@/features/providers/components/AntigravityQuotaDialog.vue', () => ({
  default: defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  }),
}))

vi.mock('@/features/providers/components/FailoverRulesDialog.vue', () => ({
  default: defineComponent({
    name: 'EmptyStub',
    setup() {
      return () => null
    },
  }),
}))

vi.mock('@/features/providers/components/ProxyNodeSelect.vue', () => ({
    default: defineComponent({
      name: 'EmptyStub',
      setup() {
        return () => null
      },
    }),
}))

vi.mock('@/stores/proxy-nodes', () => ({
  useProxyNodesStore: () => ({
    nodes: [],
    ensureLoaded: vi.fn(),
  }),
}))

vi.mock('@/composables/useToast', () => ({
  useToast: () => ({
    error: vi.fn(),
    success: vi.fn(),
    warning: vi.fn(),
  }),
}))

vi.mock('@/composables/useConfirm', () => ({
  useConfirm: () => ({ confirm: vi.fn().mockResolvedValue(true) }),
}))

vi.mock('@/composables/useClipboard', () => ({
  useClipboard: () => ({ copyToClipboard: vi.fn() }),
}))

vi.mock('@/composables/useEscapeKey', () => ({
  useEscapeKey: vi.fn(),
}))

vi.mock('@/composables/useCountdownTimer', () => ({
  getCodexResetCountdown: vi.fn(() => null),
  useCountdownTimer: () => ({
    tick: { value: 0 },
    start: vi.fn(),
    stop: vi.fn(),
  }),
}))

vi.mock('lucide-vue-next', async () => {
  const Icon = defineComponent({
    name: 'IconStub',
    setup() {
      return () => h('span')
    },
  })
  return {
    Plus: Icon,
    Key: Icon,
    Loader2: Icon,
    Edit: Icon,
    Trash2: Icon,
    RefreshCw: Icon,
    X: Icon,
    Power: Icon,
    GripVertical: Icon,
    Copy: Icon,
    Download: Icon,
    Shield: Icon,
    Shuffle: Icon,
    BarChart3: Icon,
    ShieldX: Icon,
    Globe: Icon,
    GitBranch: Icon,
  }
})

const mountedApps: Array<{ app: App, root: HTMLElement }> = []

function createProvider(id: string) {
  return {
    id,
    name: id === 'provider-a' ? 'Provider A' : 'Provider B',
    provider_type: 'openai',
    api_formats: ['openai:chat'],
    active_keys: 1,
    total_keys: 1,
    active_models: 1,
    total_models: 1,
    is_active: true,
    enable_format_conversion: false,
    pool_advanced: null,
    proxy: null,
  }
}

async function settle() {
  await nextTick()
  await Promise.resolve()
  await nextTick()
}

function mountDrawer(props: {
  providerId: string
  open: boolean
  initialProvider: ReturnType<typeof createProvider>
}) {
  const root = document.createElement('div')
  document.body.appendChild(root)
  const reactiveProps = reactive(props)
  const app = createApp({
    setup() {
      return () => h(ProviderDetailDrawer, reactiveProps)
    },
  })
  app.mount(root)
  mountedApps.push({ app, root })
  return { root, props: reactiveProps }
}

beforeEach(() => {
  endpointMocks.getProvider.mockImplementation((providerId: string) => Promise.resolve(createProvider(providerId)))
  endpointMocks.getProviderEndpoints.mockResolvedValue([])
  endpointMocks.getProviderKeysPage.mockResolvedValue({
    keys: [],
    total: 0,
    page: 1,
    page_size: 3,
  })
  endpointMocks.getProviderMappingPreview.mockResolvedValue(null)
  endpointMocks.getProviderModels.mockImplementation((providerId: string) => Promise.resolve([
    {
      id: `${providerId}-model-id`,
      provider_id: providerId,
      provider_model_name: `${providerId}-model`,
      is_active: true,
      is_available: true,
    },
  ]))
})

afterEach(() => {
  for (const { app, root } of mountedApps.splice(0)) {
    app.unmount()
    root.remove()
  }
  vi.clearAllMocks()
})

describe('ProviderDetailDrawer models state', () => {
  it('clears previous provider models immediately when provider changes', async () => {
    endpointMocks.getProviderModels.mockImplementation((providerId: string) => {
      if (providerId === 'provider-b') {
        return new Promise(() => undefined)
      }
      return Promise.resolve([
        {
          id: `${providerId}-model-id`,
          provider_id: providerId,
          provider_model_name: `${providerId}-model`,
          is_active: true,
          is_available: true,
        },
      ])
    })

    const mounted = mountDrawer({
      providerId: 'provider-a',
      open: true,
      initialProvider: createProvider('provider-a'),
    })
    await settle()
    await settle()

    let modelsTab = document.body.querySelector<HTMLElement>('[data-testid="models-tab"]')
    expect(modelsTab?.dataset.providerId).toBe('provider-a')
    expect(modelsTab?.dataset.models).toBe('provider-a-model')

    mounted.props.providerId = 'provider-b'
    mounted.props.initialProvider = createProvider('provider-b')
    await settle()

    modelsTab = document.body.querySelector<HTMLElement>('[data-testid="models-tab"]')
    expect(modelsTab?.dataset.providerId).toBe('provider-b')
    expect(modelsTab?.dataset.models).toBe('')
  })
})
