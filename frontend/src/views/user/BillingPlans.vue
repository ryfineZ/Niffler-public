<template>
  <PageContainer>
    <PageHeader
      title="套餐中心"
      description="购买周期额度或会员权益"
    />

    <div class="mt-6 space-y-6">
      <div
        v-if="loading"
        class="py-16"
      >
        <LoadingState message="正在加载套餐..." />
      </div>

      <template v-else>
        <CardSection
          title="当前权益"
          description="只展示仍在有效期内的套餐权益"
        >
          <div
            v-if="activeEntitlements.length"
            class="grid grid-cols-1 gap-3 lg:grid-cols-2"
          >
            <div
              v-for="item in activeEntitlements"
              :key="item.id"
              class="rounded-lg border border-border/60 bg-muted/20 p-4"
            >
              <div class="flex items-start justify-between gap-3">
                <div>
                  <div class="font-medium">
                    {{ planTitle(item.plan_id) }}
                  </div>
                  <div class="mt-1 text-xs text-muted-foreground">
                    {{ formatDate(item.starts_at) }} - {{ formatDate(item.expires_at) }}
                  </div>
                </div>
                <Badge variant="success">
                  生效中
                </Badge>
              </div>
              <div class="mt-3 flex flex-wrap gap-1.5">
                <Badge
                  v-for="label in entitlementLabels(item.entitlements)"
                  :key="label"
                  variant="outline"
                >
                  {{ label }}
                </Badge>
              </div>
            </div>
          </div>
          <EmptyState
            v-else
            title="暂无有效套餐"
            description="购买套餐后，有效权益会显示在这里"
          />
        </CardSection>

        <CardSection
          title="可购买套餐"
          description="支付成功后由回调自动发放权益"
        >
          <div class="grid grid-cols-1 gap-4 xl:grid-cols-3">
            <Card
              v-for="plan in purchaseablePlans"
              :key="plan.id"
              class="flex flex-col p-5"
            >
              <div class="flex items-start justify-between gap-3">
                <div>
                  <h3 class="text-base font-semibold">
                    {{ plan.title }}
                  </h3>
                  <p class="mt-1 min-h-[32px] text-xs text-muted-foreground">
                    {{ plan.description || '标准套餐' }}
                  </p>
                </div>
                <Badge variant="outline">
                  {{ formatDuration(plan.duration_unit, plan.duration_value) }}
                </Badge>
              </div>

              <div class="mt-5">
                <span class="text-3xl font-semibold tabular-nums">
                  {{ Number(plan.price_amount || 0).toFixed(2) }}
                </span>
                <span class="ml-1 text-sm text-muted-foreground">
                  {{ plan.price_currency }}
                </span>
              </div>

              <div class="mt-5 flex flex-wrap gap-1.5">
                <Badge
                  v-for="label in entitlementLabels(plan.entitlements)"
                  :key="label"
                  variant="outline"
                >
                  {{ label }}
                </Badge>
              </div>

              <div
                v-if="replacementNotice(plan)"
                class="mt-4 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs leading-5 text-amber-200"
              >
                {{ replacementNotice(plan) }}
              </div>

              <div class="mt-5 flex-1" />

              <div class="mt-5 space-y-3">
                <Select v-model="selectedPaymentOptionKey">
                  <SelectTrigger>
                    <SelectValue
                      :placeholder="paymentOptions.length ? '选择支付方式' : '暂无可用支付方式'"
                    />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem
                      v-for="option in paymentOptions"
                      :key="option.key"
                      :value="option.key"
                    >
                      {{ option.display_name }}
                    </SelectItem>
                  </SelectContent>
                </Select>
                <Button
                  class="w-full"
                  :disabled="
                    checkoutLoadingPlanId === plan.id
                      || paymentOptions.length === 0
                      || !selectedPaymentOption
                  "
                  @click="checkoutPlan(plan)"
                >
                  <CreditCard class="mr-2 h-4 w-4" />
                  {{ checkoutLoadingPlanId === plan.id ? '创建订单中...' : '购买套餐' }}
                </Button>
              </div>
            </Card>

            <div
              v-if="purchaseablePlans.length === 0"
              class="xl:col-span-3"
            >
              <EmptyState
                title="暂无可购买套餐"
                description="管理员上架套餐后会显示在这里"
              />
            </div>
          </div>
        </CardSection>

        <Card
          v-if="latestCheckout"
          class="p-4"
        >
          <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
            <div>
              <div class="text-sm font-medium">
                最新订单：<span class="font-mono">{{ latestCheckout.order.order_no }}</span>
              </div>
              <div class="mt-1 text-xs text-muted-foreground">
                应付 {{ latestCheckout.order.pay_amount ?? '-' }} {{ latestCheckout.order.pay_currency || '' }}
              </div>
            </div>
            <Button
              v-if="latestPaymentUrl"
              variant="outline"
              @click="openPaymentUrl(latestPaymentUrl)"
            >
              打开支付链接
            </Button>
          </div>
        </Card>
      </template>
    </div>
  </PageContainer>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { CreditCard } from 'lucide-vue-next'
import {
  billingApi,
  type BillingDurationUnit,
  type BillingCheckoutResponse,
  type DailyQuotaEntitlement,
  type BillingPlan,
  type UserPlanEntitlement,
} from '@/api/billing'
import { walletApi, type WalletRechargeOption } from '@/api/wallet'
import {
  Badge,
  Button,
  Card,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui'
import { EmptyState, LoadingState } from '@/components/common'
import { CardSection, PageContainer, PageHeader } from '@/components/layout'
import { useToast } from '@/composables/useToast'
import { parseApiError } from '@/utils/errorParser'
import { log } from '@/utils/logger'
import {
  hasPackageBillingEntitlement,
  normalizeBillingEntitlements,
  type BillingEntitlementsInput,
} from '@/utils/billingEntitlements'

const { success, error: showError } = useToast()

const loading = ref(true)
const plans = ref<BillingPlan[]>([])
const entitlements = ref<UserPlanEntitlement[]>([])
const rechargeOptions = ref<WalletRechargeOption[]>([])
const selectedPaymentOptionKey = ref('')
const checkoutLoadingPlanId = ref<string | null>(null)
const latestCheckout = ref<BillingCheckoutResponse | null>(null)

const paymentOptions = computed(() =>
  rechargeOptions.value
    .filter((option) => option.payment_provider === 'epay' || option.payment_method === 'epay' || option.payment_provider === 'dodopay' || option.payment_method === 'dodopay')
    .map((option, index) => ({
      ...option,
      key: [
        option.payment_provider || option.provider || option.payment_method,
        option.payment_method,
        option.payment_channel || '',
        index,
      ].join(':'),
    }))
)

const selectedPaymentOption = computed(() => {
  if (paymentOptions.value.length === 0) return null
  return paymentOptions.value.find(option => option.key === selectedPaymentOptionKey.value)
    || paymentOptions.value[0]
})

const activeEntitlements = computed(() =>
  entitlements.value.filter((item) =>
    item.active !== false
    && item.status === 'active'
    && hasPackageEntitlement(item.entitlements)
  )
)

const purchaseablePlans = computed(() =>
  plans.value.filter((plan) => hasPackageEntitlement(plan.entitlements))
)

const latestPaymentUrl = computed(() => {
  const value = latestCheckout.value?.payment_instructions?.payment_url
  return typeof value === 'string' && value ? value : ''
})

watch(paymentOptions, (options) => {
  const keys = options.map(option => option.key)
  if (!keys.includes(selectedPaymentOptionKey.value)) {
    selectedPaymentOptionKey.value = keys[0] || ''
  }
}, { immediate: true })

onMounted(async () => {
  await Promise.all([
    loadPlans(),
    loadEntitlements(),
    loadRechargeOptions(),
  ])
  loading.value = false
})

async function loadPlans() {
  try {
    const response = await billingApi.listPlans()
    plans.value = response.items
  } catch (err) {
    log.error('加载套餐失败:', err)
    showError(parseApiError(err, '加载套餐失败'))
  }
}

async function loadEntitlements() {
  try {
    const response = await billingApi.listEntitlements()
    entitlements.value = response.items
  } catch (err) {
    log.error('加载套餐权益失败:', err)
    showError(parseApiError(err, '加载套餐权益失败'))
  }
}

async function loadRechargeOptions() {
  try {
    const response = await walletApi.listRechargeOptions()
    rechargeOptions.value = response.items
    if (!selectedPaymentOptionKey.value && paymentOptions.value.length > 0) {
      selectedPaymentOptionKey.value = paymentOptions.value[0].key
    }
  } catch (err) {
    log.error('加载支付通道失败:', err)
    showError(parseApiError(err, '加载支付通道失败'))
  }
}

async function checkoutPlan(plan: BillingPlan) {
  if (hasMatchingActivePlan(plan)) {
    const confirmed = window.confirm('你已经有这个套餐，购买成功后会从当前到期时间后继续生效。确定继续购买吗？')
    if (!confirmed) return
  }
  checkoutLoadingPlanId.value = plan.id
  try {
    const option = selectedPaymentOption.value
    if (!option) {
      showError('请选择支付方式')
      return
    }
    const response = await billingApi.checkout(plan.id, {
      payment_method: option.payment_method,
      payment_provider: option.payment_provider || option.provider || option.payment_method,
      payment_channel: option.payment_channel,
    })
    latestCheckout.value = response
    success('套餐订单已创建')
    submitPaymentInstructions(response.payment_instructions)
  } catch (err) {
    log.error('创建套餐订单失败:', err)
    showError(parseApiError(err, '创建套餐订单失败'))
  } finally {
    checkoutLoadingPlanId.value = null
  }
}

function openPaymentUrl(url: string) {
  submitPaymentInstructions(latestCheckout.value?.payment_instructions || { payment_url: url })
}

function submitPaymentInstructions(instructions: Record<string, unknown> | null | undefined) {
  if (!instructions) return
  const paymentUrl = instructions.payment_url
  if (typeof paymentUrl !== 'string' || !paymentUrl) return
  const paymentParams = instructions.payment_params
  if (paymentParams && typeof paymentParams === 'object' && !Array.isArray(paymentParams)) {
    submitPaymentForm(paymentUrl, paymentParams as Record<string, unknown>)
    return
  }
  const opened = window.open(paymentUrl, '_blank', 'noopener,noreferrer')
  if (!opened) {
    window.location.href = paymentUrl
  }
}

function submitPaymentForm(url: string, params: Record<string, unknown>) {
  const form = document.createElement('form')
  form.action = url
  form.method = 'POST'
  if (!isSafariBrowser()) {
    form.target = '_blank'
  }
  Object.entries(params).forEach(([key, value]) => {
    if (value === null || value === undefined) return
    const input = document.createElement('input')
    input.type = 'hidden'
    input.name = key
    input.value = String(value)
    form.appendChild(input)
  })
  document.body.appendChild(form)
  form.submit()
  document.body.removeChild(form)
}

function isSafariBrowser(): boolean {
  return navigator.userAgent.includes('Safari') && !navigator.userAgent.includes('Chrome')
}

function planTitle(planId: string): string {
  return plans.value.find((plan) => plan.id === planId)?.title || planId
}

function hasMatchingActivePlan(plan: BillingPlan): boolean {
  return activeEntitlements.value.some((item) => item.plan_id === plan.id)
}

function replacementNotice(plan: BillingPlan): string {
  if (hasMatchingActivePlan(plan)) {
    return '你已经有这个套餐，购买成功后会从当前到期时间后继续生效。'
  }
  return ''
}

function entitlementLabels(items: BillingEntitlementsInput): string[] {
  return normalizeBillingEntitlements(items).map((item) => {
    if (item.type === 'wallet_credit') {
      return `附赠余额 $${Number(item.amount_usd || 0).toFixed(2)}`
    }
    if (item.type === 'daily_quota') {
      return quotaEntitlementLabel(item)
    }
    if (item.type === 'membership_group') {
      return `会员组 ${item.grant_user_groups.join(', ')}`
    }
    return item.type
  })
}

function hasPackageEntitlement(items: BillingEntitlementsInput): boolean {
  return hasPackageBillingEntitlement(items)
}

function quotaEntitlementLabel(item: DailyQuotaEntitlement): string {
  const limits = item.limits || {}
  const parts = []
  const daily = Number(item.daily_quota_usd ?? limits.daily_limit_usd ?? 0)
  const fiveHour = Number(item.five_hour_quota_usd ?? limits.five_hour_limit_usd ?? 0)
  const weekly = Number(item.weekly_quota_usd ?? limits.weekly_limit_usd ?? 0)
  const monthly = Number(item.monthly_quota_usd ?? limits.monthly_limit_usd ?? 0)
  if (daily > 0) parts.push(`24小时 $${daily.toFixed(2)}`)
  if (fiveHour > 0) parts.push(`5H $${fiveHour.toFixed(2)}`)
  if (weekly > 0) parts.push(`7天 $${weekly.toFixed(2)}`)
  if (monthly > 0) parts.push(`30天 $${monthly.toFixed(2)}`)
  const quotaText = parts.join(' / ') || '用量额度'
  return `${quotaText} · ${quotaModelScopeLabel(item.allowed_global_model_ids)}`
}

function quotaModelScopeLabel(modelIds?: string[]): string {
  if (!Array.isArray(modelIds) || modelIds.length === 0) {
    return '全部模型'
  }
  return `可用模型 ${modelIds.length} 个`
}

function formatDuration(unit: BillingDurationUnit, value: number): string {
  const labels: Record<BillingDurationUnit, string> = {
    day: '天',
    month: '个月',
    year: '年',
    custom: '自定义周期',
  }
  return unit === 'custom' ? `${value} ${labels[unit]}` : `${value}${labels[unit]}`
}

function formatDate(value: string | null | undefined): string {
  if (!value) return '-'
  return new Date(value).toLocaleDateString('zh-CN')
}
</script>
