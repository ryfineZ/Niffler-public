<template>
  <Dialog
    :model-value="open"
    title="用户分组"
    description="管理用户组、默认注册组、成员和组级访问控制"
    size="4xl"
    persistent
    @update:model-value="handleDialogUpdate"
  >
    <div class="grid gap-4 lg:min-h-[560px] lg:grid-cols-[17rem_minmax(0,1fr)]">
      <div class="rounded-xl border border-border/70 bg-muted/20 p-3">
        <div class="mb-3 flex justify-end">
          <Button
            variant="ghost"
            size="icon"
            class="nav-action h-8 w-8"
            title="新建分组"
            @click="startCreate"
          >
            <Plus class="h-4 w-4" />
          </Button>
        </div>

        <div
          v-if="loading"
          class="rounded-lg border border-dashed border-border/70 px-3 py-8 text-center text-xs text-muted-foreground"
        >
          正在加载...
        </div>
        <div
          v-else-if="groups.length === 0"
          class="rounded-lg border border-dashed border-border/70 px-3 py-8 text-center text-xs text-muted-foreground"
        >
          暂无分组
        </div>
        <div
          v-else
          class="max-h-60 space-y-1.5 overflow-y-auto lg:max-h-none lg:overflow-visible"
        >
          <button
            v-for="group in groups"
            :key="group.id"
            type="button"
            role="tab"
            :aria-selected="editingGroupId === group.id"
            :class="groupButtonClass(group.id)"
            @click="selectGroup(group.id)"
          >
            <span class="min-w-0 flex-1 text-left">
              <span class="flex items-center gap-1.5">
                <span class="truncate text-sm font-medium">{{ group.name }}</span>
                <Badge
                  v-if="group.is_default"
                  variant="secondary"
                  class="h-5 px-1.5 py-0 text-[10px]"
                >
                  默认
                </Badge>
              </span>
            </span>
            <ChevronRight class="h-4 w-4 shrink-0 text-muted-foreground" />
          </button>
        </div>
      </div>

      <div class="min-w-0 rounded-xl border border-border/70 bg-background p-3 sm:p-4">
        <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
          <div class="min-w-0">
            <h4 class="truncate text-base font-semibold text-foreground">
              {{ editingGroupId ? '编辑分组' : '新建分组' }}
            </h4>
            <p class="text-xs text-muted-foreground">
              {{ selectedGroup?.is_default ? '当前为所有用户的默认组' : '通过额外分组配置访问限制' }}
            </p>
          </div>
          <div
            v-if="editingGroupId"
            class="flex items-center gap-1"
          >
            <Button
              variant="ghost"
              size="icon"
              class="nav-action h-8 w-8"
              :class="selectedGroup?.is_default ? 'text-emerald-500 hover:text-emerald-500' : ''"
              :disabled="saving || selectedGroup?.is_default"
              :title="selectedGroup?.is_default ? '默认注册组' : '设为默认注册组'"
              @click="toggleDefault"
            >
              <BadgeCheck class="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="nav-action h-8 w-8"
              :disabled="saving || selectedGroup?.is_default"
              title="删除分组"
              @click="deleteSelectedGroup"
            >
              <Trash2 class="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div class="space-y-5">
          <div class="space-y-4">
            <div class="space-y-2">
              <Label class="text-sm font-medium">名称</Label>
              <Input
                v-model="form.name"
                class="h-10"
                placeholder="例如：生产团队"
              />
            </div>

            <div class="grid gap-3 sm:grid-cols-2">
              <div class="space-y-2">
                <Label class="text-sm font-medium">分组可见性</Label>
                <select
                  v-model="form.visibility"
                  class="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
                >
                  <option value="public">
                    公开分组，用户创建 API Key 时可见
                  </option>
                  <option value="internal">
                    内部分组，仅管理员分配后可见
                  </option>
                </select>
              </div>

              <div class="space-y-2">
                <Label class="text-sm font-medium">默认销售倍率</Label>
                <Input
                  :model-value="form.sales_multiplier"
                  type="number"
                  min="0"
                  step="0.01"
                  class="h-10"
                  placeholder="1 = 按官方价扣费"
                  @update:model-value="(value) => form.sales_multiplier = parseNumberInput(value, { allowFloat: true, min: 0, max: 100 }) ?? 1"
                />
              </div>
            </div>

            <div class="space-y-2">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <Label class="text-sm font-medium">模型销售倍率覆盖（可选）</Label>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  class="h-8"
                  :disabled="globalModelSelectOptions.length === 0"
                  @click="addModelSalesMultiplierRow"
                >
                  <Plus class="mr-1.5 h-3.5 w-3.5" />
                  添加模型
                </Button>
              </div>
              <div class="rounded-lg border border-border/70">
                <div
                  v-if="modelSalesMultiplierRows.length === 0"
                  class="px-3 py-4 text-sm text-muted-foreground"
                >
                  未单独设置的模型使用默认销售倍率。
                </div>
                <div
                  v-for="row in modelSalesMultiplierRows"
                  :key="row.id"
                  class="grid gap-2 border-b border-border/60 p-2 last:border-b-0 sm:grid-cols-[minmax(0,1fr)_8rem_2.25rem]"
                >
                  <select
                    v-model="row.modelId"
                    class="h-9 min-w-0 rounded-md border border-input bg-background px-3 text-sm"
                  >
                    <option value="">
                      选择模型
                    </option>
                    <option
                      v-for="option in globalModelSelectOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                  <Input
                    :model-value="row.multiplier ?? ''"
                    type="number"
                    min="0"
                    step="0.01"
                    class="h-9"
                    placeholder="倍率"
                    @update:model-value="(value) => row.multiplier = parseNumberInput(value, { allowFloat: true, min: 0, max: 100 }) ?? undefined"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    class="filter-action h-9 w-9"
                    title="删除"
                    @click="removeModelSalesMultiplierRow(row.id)"
                  >
                    <Trash2 class="h-4 w-4" />
                  </Button>
                </div>
              </div>
              <div
                v-if="providerModelMultiplierSourceOptions.length > 0"
                class="rounded-lg border border-border/70 bg-muted/20 p-3"
              >
                <div class="mb-2 text-xs font-medium text-muted-foreground">
                  按提供商批量设置
                </div>
                <div class="grid gap-2 sm:grid-cols-2">
                  <div
                    v-for="provider in providerModelMultiplierSourceOptions"
                    :key="provider.id"
                    class="rounded-lg border border-border/60 bg-background/70 p-2"
                  >
                    <div class="mb-2 truncate text-xs font-medium">
                      {{ provider.name }} · {{ provider.modelIds.length }} 个模型
                    </div>
                    <div class="flex gap-2">
                      <Input
                        :model-value="getProviderBatchSalesMultiplier(provider.id) ?? ''"
                        type="number"
                        min="0"
                        step="0.01"
                        class="h-8 min-w-0"
                        placeholder="例如 0.15"
                        @update:model-value="(value) => setProviderBatchSalesMultiplier(provider.id, value)"
                      />
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        class="h-8 shrink-0"
                        @click="applyProviderSalesMultiplier(provider)"
                      >
                        应用
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
              <p class="text-xs text-muted-foreground">
                不设置的模型使用默认销售倍率；批量按钮只是帮你快速填入这些模型。
              </p>
            </div>

            <div class="space-y-2">
              <Label class="text-sm font-medium">成员</Label>
              <MultiSelect
                v-model="memberUserIds"
                :options="userOptions"
                :search-threshold="0"
                :disabled="selectedGroup?.is_default"
                placeholder="选择用户"
                empty-text="暂无用户"
                no-results-text="未找到匹配用户"
              />
            </div>
          </div>

          <div class="space-y-4 border-t border-border/60 pt-5">
            <div class="flex flex-wrap items-baseline justify-between gap-x-2 gap-y-1 pb-2 border-b border-border/60">
              <span class="text-sm font-medium">组权限</span>
              <span class="text-[11px] text-muted-foreground">
                多个组与用户额外限制取交集
              </span>
            </div>

            <div class="space-y-2">
              <Label class="text-sm font-medium">允许的提供商</Label>
              <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <div class="flex w-full items-center sm:w-auto sm:shrink-0">
                  <Switch
                    :model-value="form.allowed_providers_mode === 'unrestricted'"
                    @update:model-value="(v) => (form.allowed_providers_mode = v ? 'unrestricted' : 'specific')"
                  />
                </div>
                <div class="min-w-0 flex-1">
                  <MultiSelect
                    v-model="form.allowed_providers"
                    :options="providerOptions"
                    :search-threshold="0"
                    :disabled="form.allowed_providers_mode === 'unrestricted'"
                    :placeholder="form.allowed_providers_mode === 'unrestricted' ? '不限制所有选项' : '选择提供商'"
                    empty-text="暂无选项"
                  />
                </div>
              </div>
            </div>

            <div class="space-y-2">
              <Label class="text-sm font-medium">允许的端点</Label>
              <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <div class="flex w-full items-center sm:w-auto sm:shrink-0">
                  <Switch
                    :model-value="form.allowed_api_formats_mode === 'unrestricted'"
                    @update:model-value="(v) => (form.allowed_api_formats_mode = v ? 'unrestricted' : 'specific')"
                  />
                </div>
                <div class="min-w-0 flex-1">
                  <MultiSelect
                    v-model="form.allowed_api_formats"
                    :options="apiFormatOptions"
                    :search-threshold="0"
                    :disabled="form.allowed_api_formats_mode === 'unrestricted'"
                    :placeholder="form.allowed_api_formats_mode === 'unrestricted' ? '不限制所有选项' : '选择端点'"
                    empty-text="暂无选项"
                  />
                </div>
              </div>
            </div>

            <div class="space-y-2">
              <Label class="text-sm font-medium">允许的模型</Label>
              <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <div class="flex w-full items-center sm:w-auto sm:shrink-0">
                  <Switch
                    :model-value="form.allowed_models_mode === 'unrestricted'"
                    @update:model-value="(v) => (form.allowed_models_mode = v ? 'unrestricted' : 'specific')"
                  />
                </div>
                <div class="min-w-0 flex-1">
                  <MultiSelect
                    v-model="form.allowed_models"
                    :options="modelOptions"
                    :search-threshold="0"
                    :disabled="form.allowed_models_mode === 'unrestricted'"
                    :placeholder="form.allowed_models_mode === 'unrestricted' ? '不限制所有选项' : '选择模型'"
                    empty-text="暂无选项"
                  />
                </div>
              </div>
              <div
                v-if="form.allowed_models_mode === 'specific' && providerModelNameSourceOptions.length > 0"
                class="rounded-lg border border-border/70 bg-muted/20 p-3"
              >
                <div class="mb-2 text-xs font-medium text-muted-foreground">
                  按提供商快速勾选
                </div>
                <div class="flex flex-wrap gap-2">
                  <button
                    v-for="provider in providerModelNameSourceOptions"
                    :key="provider.id"
                    type="button"
                    class="filter-chip rounded-full border px-3 py-1.5 text-xs font-medium transition-colors"
                    :class="[
                      provider.allSelected
                        ? 'border-primary bg-primary text-primary-foreground'
                        : provider.someSelected
                          ? 'border-primary/60 bg-primary/10 text-primary'
                          : 'border-border/60 bg-background text-muted-foreground hover:border-border hover:bg-muted/40'
                    ]"
                    @click="toggleProviderAllowedModels(provider)"
                  >
                    {{ provider.name }} · {{ provider.selectedCount }}/{{ provider.modelNames.length }}
                  </button>
                </div>
              </div>
            </div>

            <div class="space-y-2">
              <Label class="text-sm font-medium">速率限制 (请求/分钟)</Label>
              <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <div class="flex w-full items-center sm:w-auto sm:shrink-0">
                  <Switch
                    :model-value="form.rate_limit_mode === 'system'"
                    @update:model-value="(v) => (form.rate_limit_mode = v ? 'system' : 'custom')"
                  />
                </div>
                <div class="min-w-0 flex-1">
                  <Input
                    :model-value="form.rate_limit ?? ''"
                    type="number"
                    min="0"
                    max="10000"
                    class="h-10"
                    :disabled="form.rate_limit_mode === 'system'"
                    :placeholder="form.rate_limit_mode === 'system' ? '使用系统默认' : '0 = 不限速'"
                    @update:model-value="(value) => form.rate_limit = parseNumberInput(value, { min: 0, max: 10000 })"
                  />
                </div>
              </div>
            </div>

            <div class="space-y-2">
              <Label class="text-sm font-medium">并发上限</Label>
              <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <div class="flex w-full items-center sm:w-auto sm:shrink-0">
                  <Switch
                    :model-value="form.concurrent_limit_mode === 'system'"
                    @update:model-value="(v) => (form.concurrent_limit_mode = v ? 'system' : 'custom')"
                  />
                </div>
                <div class="min-w-0 flex-1">
                  <Input
                    :model-value="form.concurrent_limit ?? ''"
                    type="number"
                    min="0"
                    max="10000"
                    class="h-10"
                    :disabled="form.concurrent_limit_mode === 'system'"
                    :placeholder="form.concurrent_limit_mode === 'system' ? '使用系统默认' : '0 = 不限制'"
                    @update:model-value="(value) => form.concurrent_limit = parseNumberInput(value, { min: 0, max: 10000 })"
                  />
                </div>
              </div>
              <p class="text-xs text-muted-foreground">
                0 表示不限制并发。
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <Button
        variant="outline"
        :disabled="saving"
        @click="emit('close')"
      >
        关闭
      </Button>
      <Button
        :disabled="saving || !form.name.trim()"
        @click="saveGroup"
      >
        保存
      </Button>
    </template>
  </Dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { BadgeCheck, ChevronRight, Plus, Trash2 } from 'lucide-vue-next'
import {
  Badge,
  Button,
  Dialog,
  Input,
  Label,
  Switch,
} from '@/components/ui'
import { MultiSelect } from '@/components/common'
import { useUsersStore } from '@/stores/users'
import { useToast } from '@/composables/useToast'
import { useConfirm } from '@/composables/useConfirm'
import { parseApiError } from '@/utils/errorParser'
import { parseNumberInput } from '@/utils/form'
import { cn } from '@/lib/utils'
import { useUserAccessControlOptions } from '@/features/users/composables/useUserAccessControlOptions'
import type {
  ListPolicyMode,
  RateLimitPolicyMode,
  UpsertUserGroupRequest,
  User,
  UserGroup,
} from '@/api/users'

const props = defineProps<{
  open: boolean
  users: User[]
}>()

const emit = defineEmits<{
  close: []
  changed: []
}>()

const usersStore = useUsersStore()
const { success, error } = useToast()
const { confirmDanger, confirmInfo } = useConfirm()
const {
  providers,
  globalModels,
  providerOptions,
  apiFormatOptions,
  modelOptions,
  loadAccessControlOptions,
} = useUserAccessControlOptions()

const loading = ref(false)
const saving = ref(false)
const groups = ref<UserGroup[]>([])
const editingGroupId = ref<string | null>(null)
const memberUserIds = ref<string[]>([])
const modelSalesMultiplierRows = ref<ModelSalesMultiplierRow[]>([])
const providerBatchSalesMultipliers = ref<Record<string, number | undefined>>({})
let modelSalesMultiplierRowSequence = 0

const form = ref({
  name: '',
  visibility: 'public' as 'public' | 'internal',
  sales_multiplier: 1,
  allowed_providers_mode: 'unrestricted' as ListPolicyMode,
  allowed_api_formats_mode: 'unrestricted' as ListPolicyMode,
  allowed_models_mode: 'unrestricted' as ListPolicyMode,
  allowed_providers: [] as string[],
  allowed_api_formats: [] as string[],
  allowed_models: [] as string[],
  rate_limit_mode: 'system' as RateLimitPolicyMode,
  rate_limit: undefined as number | undefined,
  concurrent_limit_mode: 'system' as RateLimitPolicyMode,
  concurrent_limit: undefined as number | undefined,
})

const selectedGroup = computed(() => groups.value.find((group) => group.id === editingGroupId.value) ?? null)
const userOptions = computed(() => props.users.map((user) => ({
  label: `${user.username}${user.email ? ` (${user.email})` : ''}`,
  value: user.id,
})))

interface ModelSalesMultiplierRow {
  id: string
  modelId: string
  multiplier?: number
}

interface ProviderModelNameSource {
  id: string
  name: string
  modelNames: string[]
  selectedCount: number
  allSelected: boolean
  someSelected: boolean
}

interface ProviderModelMultiplierSource {
  id: string
  name: string
  modelIds: string[]
}

const globalModelById = computed(() => {
  const map = new Map<string, { id: string; name: string; display_name?: string | null }>()
  for (const model of globalModels.value) {
    map.set(model.id, model)
  }
  return map
})

const providerNamesByGlobalModelId = computed(() => {
  const map = new Map<string, string[]>()
  for (const provider of providers.value) {
    for (const modelId of provider.global_model_ids || []) {
      const names = map.get(modelId) ?? []
      names.push(provider.name)
      map.set(modelId, names)
    }
  }
  return map
})

const globalModelSelectOptions = computed(() => {
  const knownModelIds = new Set(globalModels.value.map((model) => model.id))
  const loadedOptions = globalModels.value.map((model) => {
    const providerNames = providerNamesByGlobalModelId.value.get(model.id) ?? []
    const providerText = providerNames.length ? ` · ${providerNames.join(' / ')}` : ''
    const modelText = model.display_name && model.display_name !== model.name
      ? `${model.display_name} · ${model.name}`
      : (model.name || model.id)
    return {
      value: model.id,
      label: `${modelText}${providerText}`,
    }
  })
  const missingModelIds = Array.from(new Set(
    modelSalesMultiplierRows.value
      .map((row) => row.modelId)
      .filter((modelId) => modelId && !knownModelIds.has(modelId)),
  ))
  const missingOptions = missingModelIds
    .map((modelId) => ({
      value: modelId,
      label: `${modelId} · 已失效`,
    }))
  return [...loadedOptions, ...missingOptions]
})

const providerModelNameSourceOptions = computed<ProviderModelNameSource[]>(() => {
  const selectedModelNames = new Set(form.value.allowed_models)
  return providers.value
    .map((provider) => {
      const modelNames = Array.from(new Set(
        (provider.global_model_ids || [])
          .map((modelId) => globalModelById.value.get(modelId)?.name)
          .filter((name): name is string => !!name),
      ))
      const selectedCount = modelNames.filter((name) => selectedModelNames.has(name)).length
      return {
        id: provider.id,
        name: provider.name,
        modelNames,
        selectedCount,
        allSelected: modelNames.length > 0 && selectedCount === modelNames.length,
        someSelected: selectedCount > 0,
      }
    })
    .filter((provider) => provider.modelNames.length > 0)
})

const providerModelMultiplierSourceOptions = computed<ProviderModelMultiplierSource[]>(() =>
  providers.value
    .map((provider) => {
      const modelIds = Array.from(new Set(
        (provider.global_model_ids || []).filter((modelId) => globalModelById.value.has(modelId)),
      ))
      return {
        id: provider.id,
        name: provider.name,
        modelIds,
      }
    })
    .filter((provider) => provider.modelIds.length > 0),
)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    void loadDialogData()
    void loadAccessControlOptions().catch((err) => {
      error(parseApiError(err, '加载访问控制选项失败'))
    })
  },
)

function handleDialogUpdate(value: boolean): void {
  if (!value) emit('close')
}

async function loadDialogData(): Promise<void> {
  loading.value = true
  try {
    const response = await usersStore.listUserGroups()
    groups.value = response.items
    if (editingGroupId.value && !groups.value.some((group) => group.id === editingGroupId.value)) {
      editingGroupId.value = null
    }
    const nextGroup = editingGroupId.value
      ? groups.value.find((group) => group.id === editingGroupId.value) ?? null
      : groups.value[0] ?? null
    if (nextGroup) {
      await selectGroup(nextGroup.id)
    } else {
      startCreate()
    }
  } catch (err) {
    error(parseApiError(err, '加载用户分组失败'))
  } finally {
    loading.value = false
  }
}

async function selectGroup(groupId: string): Promise<void> {
  const group = groups.value.find((item) => item.id === groupId)
  if (!group) return
  editingGroupId.value = group.id
  form.value = {
    name: group.name,
    visibility: group.visibility === 'internal' ? 'internal' : 'public',
    sales_multiplier: group.sales_multiplier ?? 1,
    allowed_providers_mode: normalizeListMode(group.allowed_providers_mode),
    allowed_api_formats_mode: normalizeListMode(group.allowed_api_formats_mode),
    allowed_models_mode: normalizeListMode(group.allowed_models_mode),
    allowed_providers: group.allowed_providers ? [...group.allowed_providers] : [],
    allowed_api_formats: group.allowed_api_formats ? [...group.allowed_api_formats] : [],
    allowed_models: group.allowed_models ? [...group.allowed_models] : [],
    rate_limit_mode: normalizeRateMode(group.rate_limit_mode),
    rate_limit: group.rate_limit ?? undefined,
    concurrent_limit_mode: normalizeRateMode(group.concurrent_limit_mode),
    concurrent_limit: group.concurrent_limit ?? undefined,
  }
  modelSalesMultiplierRows.value = rowsFromModelSalesMultipliers(group.model_sales_multipliers)
  providerBatchSalesMultipliers.value = {}
  try {
    const members = await usersStore.listUserGroupMembers(group.id)
    memberUserIds.value = members.map((member) => member.user_id)
  } catch (err) {
    memberUserIds.value = []
    error(parseApiError(err, '加载分组成员失败'))
  }
}

function normalizeListMode(mode: ListPolicyMode): ListPolicyMode {
  return mode === 'specific' ? 'specific' : 'unrestricted'
}

function normalizeRateMode(mode: RateLimitPolicyMode): RateLimitPolicyMode {
  return mode === 'custom' ? 'custom' : 'system'
}

function startCreate(): void {
  editingGroupId.value = null
  form.value = {
    name: '',
    visibility: 'public',
    sales_multiplier: 1,
    allowed_providers_mode: 'unrestricted',
    allowed_api_formats_mode: 'unrestricted',
    allowed_models_mode: 'unrestricted',
    allowed_providers: [],
    allowed_api_formats: [],
    allowed_models: [],
    rate_limit_mode: 'system',
    rate_limit: undefined,
    concurrent_limit_mode: 'system',
    concurrent_limit: undefined,
  }
  modelSalesMultiplierRows.value = []
  providerBatchSalesMultipliers.value = {}
  memberUserIds.value = []
}

function groupButtonClass(groupId: string): string {
  return cn(
    'flex w-full items-center gap-2 rounded-lg border px-3 py-2 transition-colors',
    editingGroupId.value === groupId
      ? 'border-primary/50 bg-primary/10'
      : 'border-transparent hover:border-border hover:bg-background',
  )
}

async function toggleDefault(): Promise<void> {
  const group = selectedGroup.value
  if (!group || group.is_default) return
  const confirmed = await confirmInfo(
    `确定将「${group.name}」设为默认注册组吗？后续本地注册和 OAuth 自动创建的用户将加入该分组。`,
    '设为默认注册组',
  )
  if (!confirmed) return
  saving.value = true
  try {
    await usersStore.setDefaultUserGroup(group.id)
    success('已更新默认注册组')
    emit('changed')
    await loadDialogData()
  } catch (err) {
    error(parseApiError(err, '设置默认注册组失败'))
  } finally {
    saving.value = false
  }
}

function buildPayload(): UpsertUserGroupRequest {
  return {
    name: form.value.name.trim(),
    visibility: form.value.visibility,
    sales_multiplier: form.value.sales_multiplier,
    model_sales_multipliers: parseModelSalesMultipliers(),
    allowed_providers_mode: form.value.allowed_providers_mode,
    allowed_api_formats_mode: form.value.allowed_api_formats_mode,
    allowed_models_mode: form.value.allowed_models_mode,
    allowed_providers: form.value.allowed_providers_mode === 'specific'
      ? [...form.value.allowed_providers]
      : null,
    allowed_api_formats: form.value.allowed_api_formats_mode === 'specific'
      ? [...form.value.allowed_api_formats]
      : null,
    allowed_models: form.value.allowed_models_mode === 'specific'
      ? [...form.value.allowed_models]
      : null,
    rate_limit_mode: form.value.rate_limit_mode,
    rate_limit: form.value.rate_limit_mode === 'custom'
      ? (form.value.rate_limit ?? 0)
      : null,
    concurrent_limit_mode: form.value.concurrent_limit_mode,
    concurrent_limit: form.value.concurrent_limit_mode === 'custom'
      ? (form.value.concurrent_limit ?? 0)
      : null,
  }
}

function nextModelSalesMultiplierRowId(): string {
  modelSalesMultiplierRowSequence += 1
  return `model-sales-${modelSalesMultiplierRowSequence}`
}

function rowsFromModelSalesMultipliers(value: unknown): ModelSalesMultiplierRow[] {
  if (!value || Array.isArray(value) || typeof value !== 'object') return []
  return Object.entries(value as Record<string, unknown>)
    .filter(([modelId, multiplier]) =>
      modelId.trim()
      && typeof multiplier === 'number'
      && Number.isFinite(multiplier)
      && multiplier >= 0,
    )
    .map(([modelId, multiplier]) => ({
      id: nextModelSalesMultiplierRowId(),
      modelId,
      multiplier: multiplier as number,
    }))
}

function addModelSalesMultiplierRow(): void {
  const usedModelIds = new Set(modelSalesMultiplierRows.value.map((row) => row.modelId).filter(Boolean))
  const firstAvailableModel = globalModelSelectOptions.value.find((option) => !usedModelIds.has(option.value))
  modelSalesMultiplierRows.value.push({
    id: nextModelSalesMultiplierRowId(),
    modelId: firstAvailableModel?.value ?? '',
    multiplier: form.value.sales_multiplier,
  })
}

function removeModelSalesMultiplierRow(rowId: string): void {
  modelSalesMultiplierRows.value = modelSalesMultiplierRows.value.filter((row) => row.id !== rowId)
}

function parseModelSalesMultipliers(): Record<string, number> | null {
  const result: Record<string, number> = {}
  const seenModelIds = new Set<string>()
  for (const row of modelSalesMultiplierRows.value) {
    const modelId = row.modelId.trim()
    if (!modelId && row.multiplier === undefined) continue
    if (!modelId) throw new Error('请选择要单独设置倍率的模型')
    if (seenModelIds.has(modelId)) throw new Error('同一个模型不能重复设置销售倍率')
    if (row.multiplier === undefined || !Number.isFinite(row.multiplier) || row.multiplier < 0) {
      throw new Error('模型销售倍率必须是大于等于 0 的数字')
    }
    seenModelIds.add(modelId)
    result[modelId] = row.multiplier
  }
  return Object.keys(result).length ? result : null
}

function toggleProviderAllowedModels(provider: ProviderModelNameSource): void {
  const nextModelNames = new Set(form.value.allowed_models)
  if (provider.allSelected) {
    for (const modelName of provider.modelNames) {
      nextModelNames.delete(modelName)
    }
  } else {
    for (const modelName of provider.modelNames) {
      nextModelNames.add(modelName)
    }
  }
  form.value.allowed_models = Array.from(nextModelNames)
}

function getProviderBatchSalesMultiplier(providerId: string): number | undefined {
  return providerBatchSalesMultipliers.value[providerId]
}

function setProviderBatchSalesMultiplier(providerId: string, value: string | number | null | undefined): void {
  providerBatchSalesMultipliers.value = {
    ...providerBatchSalesMultipliers.value,
    [providerId]: parseNumberInput(value, { allowFloat: true, min: 0, max: 100 }),
  }
}

function applyProviderSalesMultiplier(provider: ProviderModelMultiplierSource): void {
  const multiplier = getProviderBatchSalesMultiplier(provider.id)
  if (multiplier === undefined) {
    error('请先填写这个提供商的销售倍率')
    return
  }
  const nextByModelId = new Map<string, number>()
  for (const row of modelSalesMultiplierRows.value) {
    if (row.modelId && row.multiplier !== undefined) {
      nextByModelId.set(row.modelId, row.multiplier)
    }
  }
  for (const modelId of provider.modelIds) {
    nextByModelId.set(modelId, multiplier)
  }
  modelSalesMultiplierRows.value = Array.from(nextByModelId.entries()).map(([modelId, multiplier]) => ({
    id: nextModelSalesMultiplierRowId(),
    modelId,
    multiplier,
  }))
}

async function saveGroup(): Promise<void> {
  if (!form.value.name.trim()) return
  saving.value = true
  try {
    const saved = editingGroupId.value
      ? await usersStore.updateUserGroup(editingGroupId.value, buildPayload())
      : await usersStore.createUserGroup(buildPayload())
    if (!saved.is_default) {
      await usersStore.replaceUserGroupMembers(saved.id, memberUserIds.value)
    }
    success('用户分组已保存')
    emit('changed')
    editingGroupId.value = saved.id
    await loadDialogData()
  } catch (err) {
    error(parseApiError(err, '保存用户分组失败'))
  } finally {
    saving.value = false
  }
}

async function deleteSelectedGroup(): Promise<void> {
  if (!selectedGroup.value) return
  const group = selectedGroup.value
  const confirmed = await confirmDanger(
    `确定要删除用户分组 ${group.name} 吗？成员关系会一并清理。`,
    '删除用户分组',
  )
  if (!confirmed) return
  saving.value = true
  try {
    await usersStore.deleteUserGroup(group.id)
    success('用户分组已删除')
    emit('changed')
    editingGroupId.value = null
    await loadDialogData()
  } catch (err) {
    error(parseApiError(err, '删除用户分组失败'))
  } finally {
    saving.value = false
  }
}
</script>
