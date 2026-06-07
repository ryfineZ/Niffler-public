<template>
  <Teleport to="body">
    <Transition name="drawer">
      <div
        v-if="open && localWallet"
        class="fixed inset-0 z-[80] flex justify-end"
      >
        <div
          class="absolute inset-0 bg-black/35 backdrop-blur-sm"
          @click="handleClose"
        />

        <div class="drawer-panel relative h-full w-full sm:w-[760px] lg:w-[860px] sm:max-w-[95vw] border-l border-border bg-background shadow-2xl overflow-y-auto">
          <div class="sticky top-0 z-10 border-b border-border bg-background/95 backdrop-blur px-4 py-3 sm:px-6 sm:py-4">
            <div class="flex items-start justify-between gap-3">
              <div class="flex items-center gap-3 min-w-0">
                <div
                  class="flex h-10 w-10 items-center justify-center rounded-xl shrink-0"
                  :class="accentClasses"
                >
                  <Wallet class="h-5 w-5" />
                </div>
                <div class="min-w-0">
                  <div class="flex items-center gap-1.5">
                    <h3 class="text-lg font-semibold text-foreground leading-tight">
                      {{ contextLabel || '钱包详情' }}
                    </h3>
                    <Badge
                      :variant="walletStatusBadge(localWallet.status)"
                      class="w-fit px-2 py-0.5 text-[11px] leading-none"
                    >
                      {{ walletStatusLabel(localWallet.status) }}
                    </Badge>
                  </div>
                  <p class="text-xs text-muted-foreground">
                    {{ ownerName || '-' }} <span v-if="ownerSubtitle">· {{ ownerSubtitle }}</span>
                  </p>
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                class="h-9 w-9 shrink-0"
                title="关闭"
                @click="handleClose"
              >
                <X class="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div class="p-4 sm:p-6 space-y-5">
            <div class="rounded-2xl border border-border/60 bg-muted/30 p-4">
              <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                <div class="rounded-xl bg-background/80 p-3">
                  <div class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
                    总可用额度
                  </div>
                  <div
                    class="mt-1 text-lg font-semibold"
                    :class="totalAvailableAmount !== null && totalAvailableAmount < 0 ? 'text-rose-600' : 'text-foreground'"
                  >
                    {{ totalAvailableAmount === null ? '不限额' : `$${formatFixed(totalAvailableAmount, 2)}` }}
                  </div>
                </div>
                <div class="rounded-xl bg-background/80 p-3">
                  <div class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
                    套餐今日额度
                  </div>
                  <div class="mt-1 text-lg font-semibold text-foreground">
                    {{ isApiKeyWallet ? '不适用' : `$${formatFixed(packageBalanceAmount, 2)}` }}
                  </div>
                  <div
                    v-if="dailyQuota?.has_active"
                    class="mt-1 text-[11px] text-muted-foreground"
                  >
                    已用 ${{ formatFixed(dailyQuota.used_usd, 2) }} / ${{ formatFixed(dailyQuota.total_usd, 2) }}
                  </div>
                </div>
                <div class="rounded-xl bg-background/80 p-3">
                  <div class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
                    钱包余额
                  </div>
                  <div
                    class="mt-1 text-lg font-semibold"
                    :class="walletBalanceAmount < 0 ? 'text-rose-600' : 'text-foreground'"
                  >
                    ${{ formatFixed(walletBalanceAmount, 2) }}
                  </div>
                </div>
                <div class="rounded-xl bg-background/80 p-3">
                  <div class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
                    充值余额
                  </div>
                  <div class="mt-1 text-lg font-semibold text-foreground">
                    ${{ formatFixed(localWallet.recharge_balance, 2) }}
                  </div>
                </div>
                <div class="rounded-xl bg-background/80 p-3">
                  <div class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
                    赠款余额
                  </div>
                  <div class="mt-1 text-lg font-semibold text-foreground">
                    {{ isApiKeyWallet ? '不支持' : `$${formatFixed(localWallet.gift_balance, 2)}` }}
                  </div>
                </div>
                <div class="rounded-xl bg-background/80 p-3">
                  <div class="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
                    累计消费
                  </div>
                  <div class="mt-1 text-lg font-semibold text-foreground">
                    ${{ formatFixed(localWallet.total_consumed, 2) }}
                  </div>
                </div>
              </div>
              <p
                v-if="!isApiKeyWallet"
                class="mt-3 text-xs text-muted-foreground"
              >
                实际扣费顺序为套餐每日额度、充值余额、赠款余额；套餐额度不通过资金操作调整，请在用户套餐中发放或替换。
              </p>
            </div>

            <Tabs v-model="activeTab">
              <TabsList :class="tabsListClass">
                <TabsTrigger value="actions">
                  资金操作
                </TabsTrigger>
                <TabsTrigger value="transactions">
                  资金流水
                </TabsTrigger>
                <TabsTrigger
                  v-if="showUsageRecords"
                  value="usage"
                >
                  消费记录
                </TabsTrigger>
                <TabsTrigger
                  v-if="showPlanRecords"
                  value="plans"
                >
                  套餐记录
                </TabsTrigger>
                <TabsTrigger
                  v-if="showRefunds"
                  value="refunds"
                >
                  退款审批
                </TabsTrigger>
              </TabsList>

              <TabsContent
                value="actions"
                class="mt-4 space-y-4"
              >
                <div
                  v-if="!isApiKeyWallet"
                  class="space-y-2"
                >
                  <Label class="text-sm font-medium">
                    操作类型
                  </Label>
                  <Select v-model="moneyActionType">
                    <SelectTrigger class="h-11">
                      <SelectValue placeholder="选择操作类型" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="recharge">
                        人工充值
                      </SelectItem>
                      <SelectItem value="adjust">
                        人工调账
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div class="space-y-2">
                  <Label
                    for="wallet-action-amount"
                    class="text-sm font-medium"
                  >金额 (USD)</Label>
                  <Input
                    id="wallet-action-amount"
                    :model-value="actionAmount ?? ''"
                    type="number"
                    step="0.01"
                    :placeholder="isApiKeyWallet || moneyActionType === 'adjust' ? '正数为入账，负数为调账扣减' : '输入正数金额'"
                    class="h-11"
                    @update:model-value="(value) => actionAmount = parseNumberInput(value, { allowFloat: true })"
                  />
                  <p class="text-xs text-muted-foreground">
                    {{
                      isApiKeyWallet || moneyActionType === 'adjust'
                        ? '调账扣减会先扣所选账户，不足自动扣另一账户，剩余计入充值余额。'
                        : '人工充值仅接受正数。'
                    }}
                  </p>
                </div>

                <div class="space-y-2">
                  <Label
                    for="wallet-action-description"
                    class="text-sm font-medium"
                  >说明</Label>
                  <Input
                    id="wallet-action-description"
                    v-model="actionDescription"
                    type="text"
                    placeholder="填写备注，便于财务追溯"
                    class="h-11"
                  />
                </div>

                <div
                  v-if="moneyActionType === 'adjust' && !isApiKeyWallet"
                  class="space-y-2"
                >
                  <Label class="text-sm font-medium">
                    调账账户
                  </Label>
                  <Select v-model="adjustBalanceType">
                    <SelectTrigger class="h-11">
                      <SelectValue placeholder="选择调账账户" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="recharge">
                        充值余额（可退款）
                      </SelectItem>
                      <SelectItem
                        v-if="!isApiKeyWallet"
                        value="gift"
                      >
                        赠款余额（不可退款）
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div
                  v-if="!isApiKeyWallet"
                  class="rounded-xl border border-border/60 p-3 text-xs text-muted-foreground"
                >
                  人工充值等同于用户充值余额，会产生充值订单和记录；调帐为后台调整，无充值订单。赠款余额不可退款。
                </div>

                <div class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                  <Button
                    variant="outline"
                    class="h-10 px-5"
                    @click="handleClose"
                  >
                    关闭
                  </Button>
                  <Button
                    class="h-10 px-5"
                    :disabled="submitMoneyDisabled"
                    @click="submitMoneyAction"
                  >
                    {{ submittingMoneyAction ? '处理中...' : submitMoneyLabel }}
                  </Button>
                </div>
              </TabsContent>

              <TabsContent
                value="transactions"
                class="mt-4 space-y-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <div class="text-sm text-muted-foreground">
                    共 {{ txTotal }} 条
                  </div>
                  <RefreshButton
                    :loading="loadingTx"
                    @click="loadTransactions"
                  />
                </div>

                <div class="rounded-2xl border border-border/60 overflow-hidden bg-background">
                  <div class="overflow-x-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>时间</TableHead>
                          <TableHead>类型</TableHead>
                          <TableHead>金额</TableHead>
                          <TableHead>余额变化</TableHead>
                          <TableHead>说明</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        <TableRow
                          v-for="tx in txItems"
                          :key="tx.id"
                        >
                          <TableCell class="text-xs text-muted-foreground whitespace-nowrap">
                            {{ formatDateTime(tx.created_at) }}
                          </TableCell>
                          <TableCell>
                            <div class="space-y-1">
                              <Badge
                                variant="outline"
                                class="font-mono"
                              >
                                {{ walletTransactionCategoryLabel(tx.category) }}
                              </Badge>
                              <div class="text-[11px] text-muted-foreground">
                                {{ walletTransactionReasonLabel(tx.reason_code) }}
                              </div>
                            </div>
                          </TableCell>
                          <TableCell
                            class="tabular-nums"
                            :class="toFiniteNumber(tx.amount) >= 0 ? 'text-emerald-600' : 'text-rose-600'"
                          >
                            {{ toFiniteNumber(tx.amount) >= 0 ? '+' : '' }}{{ formatFixed(tx.amount, 4) }}
                          </TableCell>
                          <TableCell class="text-xs tabular-nums whitespace-nowrap">
                            <div>{{ formatFixed(tx.balance_before, 4) }} → {{ formatFixed(tx.balance_after, 4) }}</div>
                            <div
                              v-if="tx.recharge_balance_before !== null && tx.recharge_balance_before !== undefined && tx.gift_balance_before !== null && tx.gift_balance_before !== undefined"
                              class="text-[11px] text-muted-foreground mt-0.5"
                            >
                              充 {{ formatFixed(tx.recharge_balance_before, 4) }}→{{ formatFixed(tx.recharge_balance_after, 4) }}
                              · 赠 {{ formatFixed(tx.gift_balance_before, 4) }}→{{ formatFixed(tx.gift_balance_after, 4) }}
                            </div>
                          </TableCell>
                          <TableCell class="text-xs text-muted-foreground max-w-[260px] truncate">
                            {{ tx.description || '-' }}
                          </TableCell>
                        </TableRow>
                        <TableRow v-if="!loadingTx && txItems.length === 0">
                          <TableCell
                            colspan="5"
                            class="py-10"
                          >
                            <EmptyState
                              title="暂无资金流水"
                              description="当前钱包没有资金动作记录"
                            />
                          </TableCell>
                        </TableRow>
                      </TableBody>
                    </Table>
                  </div>
                </div>

                <Pagination
                  :current="txPage"
                  :total="txTotal"
                  :page-size="txPageSize"
                  @update:current="handleTxPageChange"
                  @update:page-size="handleTxPageSizeChange"
                />
              </TabsContent>

              <TabsContent
                v-if="showUsageRecords"
                value="usage"
                class="mt-4 space-y-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <div class="text-sm text-muted-foreground">
                    最近 30 天 · 共 {{ usageTotal }} 条
                  </div>
                  <RefreshButton
                    :loading="loadingUsage"
                    @click="loadUsageRecords"
                  />
                </div>

                <div class="rounded-2xl border border-border/60 overflow-hidden bg-background">
                  <div class="overflow-x-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>时间</TableHead>
                          <TableHead>模型</TableHead>
                          <TableHead>状态</TableHead>
                          <TableHead class="text-right">
                            官方价格
                          </TableHead>
                          <TableHead class="text-right">
                            套餐扣除
                          </TableHead>
                          <TableHead class="text-right">
                            钱包扣除
                          </TableHead>
                          <TableHead class="text-right">
                            平台成本
                          </TableHead>
                          <TableHead class="text-right">
                            成本倍率
                          </TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        <TableRow
                          v-for="record in usageItems"
                          :key="record.id"
                          :title="usageCostTitle(record)"
                        >
                          <TableCell class="text-xs text-muted-foreground whitespace-nowrap">
                            {{ formatDateTime(record.created_at) }}
                          </TableCell>
                          <TableCell class="min-w-[220px]">
                            <div class="text-sm font-medium text-foreground">
                              {{ record.model || '-' }}
                            </div>
                            <div class="mt-1 text-[11px] text-muted-foreground">
                              {{ record.api_key?.name || record.api_key_name || '未命名 Key' }}
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge :variant="usageStatusBadge(record)">
                              {{ usageStatusLabel(record) }}
                            </Badge>
                          </TableCell>
                          <TableCell class="text-right text-xs tabular-nums text-primary">
                            {{ formatUsageCurrency(usageOfficialCost(record)) }}
                          </TableCell>
                          <TableCell class="text-right text-xs tabular-nums">
                            {{ formatUsageDebitWithMultiplier(usagePackageDebit(record), usagePackageMultiplier(record)) }}
                          </TableCell>
                          <TableCell class="text-right text-xs tabular-nums text-muted-foreground">
                            {{ formatUsageDebitWithMultiplier(usageWalletDebit(record), usageWalletMultiplier(record)) }}
                          </TableCell>
                          <TableCell class="text-right text-xs tabular-nums">
                            {{ hasUsagePlatformCost(record) ? formatUsageCurrency(usagePlatformCost(record)) : '-' }}
                          </TableCell>
                          <TableCell class="text-right text-xs tabular-nums text-muted-foreground">
                            {{ formatUsageCostMultiplier(record) }}
                          </TableCell>
                        </TableRow>
                        <TableRow v-if="!loadingUsage && usageItems.length === 0">
                          <TableCell
                            colspan="8"
                            class="py-10"
                          >
                            <EmptyState
                              title="暂无消费记录"
                              description="最近 30 天内没有可核账的模型请求"
                            />
                          </TableCell>
                        </TableRow>
                      </TableBody>
                    </Table>
                  </div>
                </div>

                <Pagination
                  :current="usagePage"
                  :total="usageTotal"
                  :page-size="usagePageSize"
                  @update:current="handleUsagePageChange"
                  @update:page-size="handleUsagePageSizeChange"
                />
              </TabsContent>

              <TabsContent
                v-if="showPlanRecords"
                value="plans"
                class="mt-4 space-y-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <div class="text-sm text-muted-foreground">
                    共 {{ planItems.length }} 条
                  </div>
                  <RefreshButton
                    :loading="loadingPlans"
                    @click="loadPlans"
                  />
                </div>

                <div class="rounded-2xl border border-border/60 overflow-hidden bg-background">
                  <div class="overflow-x-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>套餐</TableHead>
                          <TableHead>状态</TableHead>
                          <TableHead>权益</TableHead>
                          <TableHead>价格/额度</TableHead>
                          <TableHead>开始</TableHead>
                          <TableHead>到期</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        <TableRow
                          v-for="plan in planItems"
                          :key="plan.id"
                        >
                          <TableCell class="min-w-[160px]">
                            <div class="font-medium text-foreground">
                              {{ plan.plan_title || plan.plan?.title || plan.plan_id }}
                            </div>
                            <div class="mt-1 text-[11px] text-muted-foreground">
                              {{ plan.payment_order_id || '-' }}
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge
                              :variant="plan.active ? 'success' : 'secondary'"
                              class="whitespace-nowrap"
                            >
                              {{ plan.active ? '生效中' : planStatusLabel(plan.status) }}
                            </Badge>
                          </TableCell>
                          <TableCell class="max-w-[260px]">
                            <div class="flex flex-wrap gap-1.5">
                              <Badge
                                v-for="label in entitlementLabels(plan.entitlements)"
                                :key="label"
                                variant="outline"
                                class="h-5 px-1.5 py-0 text-[10px]"
                              >
                                {{ label }}
                              </Badge>
                            </div>
                          </TableCell>
                          <TableCell class="text-xs text-muted-foreground whitespace-nowrap">
                            <div class="text-foreground">
                              {{ formatPlanPrice(plan.plan) }} / {{ formatPlanQuota(plan.plan) }}
                            </div>
                            <div class="mt-1">
                              {{ formatPlanEquivalentMultiplier(plan.plan) }}
                            </div>
                          </TableCell>
                          <TableCell class="text-xs text-muted-foreground whitespace-nowrap">
                            {{ formatDateTime(plan.starts_at) }}
                          </TableCell>
                          <TableCell class="text-xs text-muted-foreground whitespace-nowrap">
                            {{ formatDateTime(plan.expires_at) }}
                          </TableCell>
                        </TableRow>
                        <TableRow v-if="!loadingPlans && planItems.length === 0">
                          <TableCell
                            colspan="6"
                            class="py-10"
                          >
                            <EmptyState
                              title="暂无套餐记录"
                              description="当前用户还没有套餐发放或购买记录"
                            />
                          </TableCell>
                        </TableRow>
                      </TableBody>
                    </Table>
                  </div>
                </div>
              </TabsContent>

              <TabsContent
                v-if="showRefunds"
                value="refunds"
                class="mt-4 space-y-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <div class="text-sm text-muted-foreground">
                    共 {{ refundTotal }} 条
                  </div>
                  <RefreshButton
                    :loading="loadingRefunds"
                    @click="loadRefunds"
                  />
                </div>

                <div
                  v-if="refundActionType && actionRefund"
                  class="rounded-xl border border-border/60 p-4 space-y-3"
                >
                  <div class="text-sm font-semibold">
                    {{ refundActionType === 'fail' ? '驳回退款' : '完成退款' }} - {{ actionRefund.refund_no }}
                  </div>
                  <template v-if="refundActionType === 'fail'">
                    <div class="space-y-1.5">
                      <Label>驳回原因</Label>
                      <Input
                        v-model="refundFailReason"
                        placeholder="请填写驳回原因"
                      />
                    </div>
                  </template>
                  <template v-else>
                    <div class="space-y-1.5">
                      <Label>网关退款号（可选）</Label>
                      <Input v-model="refundGatewayRefundId" />
                    </div>
                    <div class="space-y-1.5">
                      <Label>打款凭证 / 参考号（可选）</Label>
                      <Input v-model="refundPayoutReference" />
                    </div>
                  </template>
                  <div class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                    <Button
                      variant="outline"
                      @click="resetRefundActionForm"
                    >
                      取消
                    </Button>
                    <Button
                      v-if="refundActionType === 'fail'"
                      variant="destructive"
                      :disabled="submittingRefundAction"
                      @click="submitFailRefund"
                    >
                      {{ submittingRefundAction ? '提交中...' : '确认驳回' }}
                    </Button>
                    <Button
                      v-else
                      :disabled="submittingRefundAction"
                      @click="submitCompleteRefund"
                    >
                      {{ submittingRefundAction ? '提交中...' : '确认完成' }}
                    </Button>
                  </div>
                </div>

                <div class="rounded-2xl border border-border/60 overflow-hidden bg-background">
                  <div class="overflow-x-auto">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>退款单号</TableHead>
                          <TableHead>金额</TableHead>
                          <TableHead>模式</TableHead>
                          <TableHead>状态</TableHead>
                          <TableHead>原因</TableHead>
                          <TableHead class="text-right">
                            操作
                          </TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        <TableRow
                          v-for="refund in refundItems"
                          :key="refund.id"
                        >
                          <TableCell class="font-mono text-xs whitespace-nowrap">
                            {{ refund.refund_no }}
                          </TableCell>
                          <TableCell class="tabular-nums whitespace-nowrap">
                            ${{ formatFixed(refund.amount_usd, 4) }}
                          </TableCell>
                          <TableCell>
                            {{ refundModeLabel(refund.refund_mode) }}
                          </TableCell>
                          <TableCell>
                            <Badge :variant="refundStatusBadge(refund.status)">
                              {{ refundStatusLabel(refund.status) }}
                            </Badge>
                          </TableCell>
                          <TableCell class="text-xs text-muted-foreground max-w-[220px] truncate">
                            {{ refund.reason || refund.failure_reason || '-' }}
                          </TableCell>
                          <TableCell class="text-right">
                            <div class="flex justify-end gap-2">
                              <Button
                                v-if="canProcessRefund(refund.status)"
                                size="sm"
                                variant="outline"
                                :disabled="submittingRefundAction"
                                @click="processRefund(refund)"
                              >
                                处理
                              </Button>
                              <Button
                                v-if="canCompleteRefund(refund.status)"
                                size="sm"
                                :disabled="submittingRefundAction"
                                @click="openCompleteRefund(refund)"
                              >
                                完成
                              </Button>
                              <Button
                                v-if="canFailRefund(refund.status)"
                                size="sm"
                                variant="destructive"
                                :disabled="submittingRefundAction"
                                @click="openFailRefund(refund)"
                              >
                                驳回
                              </Button>
                            </div>
                          </TableCell>
                        </TableRow>
                        <TableRow v-if="!loadingRefunds && refundItems.length === 0">
                          <TableCell
                            colspan="6"
                            class="py-10"
                          >
                            <EmptyState
                              title="暂无退款申请"
                              description="当前钱包没有退款单"
                            />
                          </TableCell>
                        </TableRow>
                      </TableBody>
                    </Table>
                  </div>
                </div>

                <Pagination
                  :current="refundPage"
                  :total="refundTotal"
                  :page-size="refundPageSize"
                  @update:current="handleRefundPageChange"
                  @update:page-size="handleRefundPageSizeChange"
                />
              </TabsContent>
            </Tabs>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  Badge,
  Button,
  Input,
  Label,
  Pagination,
  RefreshButton,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui'
import { EmptyState } from '@/components/common'
import {
  adminWalletApi,
  type AdminWallet,
} from '@/api/admin-wallets'
import { usageApi } from '@/api/usage'
import { usersApi, type AdminUserPlanEntitlement } from '@/api/users'
import type { BillingEntitlementsInput, BillingPlan, DailyQuotaEntitlement } from '@/api/billing'
import type { RefundRequest, WalletTransaction } from '@/api/wallet'
import type { UsageRecord } from '@/features/usage/types'
import { normalizeBillingEntitlements } from '@/utils/billingEntitlements'
import { parseApiError } from '@/utils/errorParser'
import { parseNumberInput } from '@/utils/form'
import { formatCurrency } from '@/utils/format'
import {
  refundModeLabel,
  refundStatusBadge,
  refundStatusLabel,
  walletStatusBadge,
  walletStatusLabel,
  walletTransactionCategoryLabel,
  walletTransactionReasonLabel,
} from '@/utils/walletDisplay'
import { useToast } from '@/composables/useToast'
import { useConfirm } from '@/composables/useConfirm'
import { Wallet, X } from 'lucide-vue-next'
import { log } from '@/utils/logger'

const props = withDefaults(
  defineProps<{
    open: boolean
    wallet: AdminWallet | null
    ownerName?: string
    ownerSubtitle?: string
    contextLabel?: string
    userId?: string | null
    accent?: 'emerald' | 'blue'
    showRefunds?: boolean
  }>(),
  {
    ownerName: '',
    ownerSubtitle: '',
    contextLabel: '钱包详情',
    userId: null,
    accent: 'emerald',
    showRefunds: true,
  }
)

const emit = defineEmits<{
  close: []
  changed: []
}>()

const { success, error } = useToast()
const { confirm } = useConfirm()

const activeTab = ref<'actions' | 'transactions' | 'usage' | 'plans' | 'refunds'>('actions')
const localWallet = ref<AdminWallet | null>(null)

const moneyActionType = ref<'recharge' | 'adjust'>('adjust')
const actionAmount = ref<number | undefined>(undefined)
const actionDescription = ref('')
const adjustBalanceType = ref<'recharge' | 'gift'>('recharge')
const submittingMoneyAction = ref(false)

const loadingTx = ref(false)
const txItems = ref<WalletTransaction[]>([])
const txTotal = ref(0)
const txPage = ref(1)
const txPageSize = ref(20)
const loadingUsage = ref(false)
const usageItems = ref<UsageRecord[]>([])
const usageTotal = ref(0)
const usagePage = ref(1)
const usagePageSize = ref(20)

const loadingRefunds = ref(false)
const refundItems = ref<RefundRequest[]>([])
const refundTotal = ref(0)
const refundPage = ref(1)
const refundPageSize = ref(20)
const submittingRefundAction = ref(false)
const loadingPlans = ref(false)
const planItems = ref<AdminUserPlanEntitlement[]>([])

const refundActionType = ref<'fail' | 'complete' | null>(null)
const actionRefund = ref<RefundRequest | null>(null)
const refundFailReason = ref('')
const refundGatewayRefundId = ref('')
const refundPayoutReference = ref('')

const accentClasses = computed(() => {
  return props.accent === 'blue' ? 'bg-blue-500/10 text-blue-600' : 'bg-emerald-500/10 text-emerald-600'
})
const isApiKeyWallet = computed(() => localWallet.value?.owner_type === 'api_key')
const dailyQuota = computed(() => localWallet.value?.daily_quota ?? null)
const packageBalanceAmount = computed(() => toFiniteNumber(localWallet.value?.package_balance, 0))
const walletBalanceAmount = computed(() => toFiniteNumber(localWallet.value?.wallet_balance ?? localWallet.value?.balance, 0))
const totalAvailableAmount = computed(() => {
  if (!localWallet.value) return 0
  if (localWallet.value.unlimited || localWallet.value.total_available_balance === null) return null
  return toFiniteNumber(
    localWallet.value.total_available_balance,
    walletBalanceAmount.value + packageBalanceAmount.value
  )
})
const showRefunds = computed(() => props.showRefunds)
const showPlanRecords = computed(() => Boolean(props.userId && !isApiKeyWallet.value))
const showUsageRecords = computed(() => Boolean(props.userId && !isApiKeyWallet.value))
const tabsListClass = computed(() => {
  const columnCount = 2 + (showUsageRecords.value ? 1 : 0) + (showPlanRecords.value ? 1 : 0) + (showRefunds.value ? 1 : 0)
  return [
    'tabs-button-list',
    'grid',
    'w-full',
    columnCount === 5 ? 'grid-cols-5' : columnCount === 4 ? 'grid-cols-4' : columnCount === 3 ? 'grid-cols-3' : 'grid-cols-2',
  ]
})
const submitMoneyDisabled = computed(() => {
  if (submittingMoneyAction.value) return true
  if (!actionAmount.value) return true
  if (moneyActionType.value === 'recharge') {
    return actionAmount.value <= 0
  }
  return actionAmount.value === 0
})
const submitMoneyLabel = computed(() => {
  if (isApiKeyWallet.value) return '确认调账'
  return moneyActionType.value === 'recharge' ? '确认充值' : '确认调账'
})

watch(
  () => [props.open, props.wallet?.id, props.userId] as const,
  async ([open]) => {
    if (!open || !props.wallet) {
      return
    }
    localWallet.value = { ...props.wallet }
    resetActionForm()
    resetRefundActionForm()
    activeTab.value = 'actions'
    txPage.value = 1
    usagePage.value = 1
    refundPage.value = 1
    usageItems.value = []
    usageTotal.value = 0
    planItems.value = []
    await refreshDrawerData()
  },
  { immediate: true }
)

function handleClose() {
  emit('close')
}

function resetActionForm() {
  moneyActionType.value = isApiKeyWallet.value ? 'adjust' : 'recharge'
  actionAmount.value = undefined
  actionDescription.value = ''
  adjustBalanceType.value = 'recharge'
}

watch(
  () => [moneyActionType.value, isApiKeyWallet.value] as const,
  () => {
    if (isApiKeyWallet.value && moneyActionType.value !== 'adjust') {
      moneyActionType.value = 'adjust'
      return
    }
    if (moneyActionType.value !== 'adjust') {
      adjustBalanceType.value = 'recharge'
      return
    }
    if (isApiKeyWallet.value && adjustBalanceType.value === 'gift') {
      adjustBalanceType.value = 'recharge'
    }
  }
)

function resetRefundActionForm() {
  refundActionType.value = null
  actionRefund.value = null
  refundFailReason.value = ''
  refundGatewayRefundId.value = ''
  refundPayoutReference.value = ''
}

async function loadTransactions() {
  if (!localWallet.value) return
  loadingTx.value = true
  try {
    const offset = (txPage.value - 1) * txPageSize.value
    const resp = await adminWalletApi.getWalletTransactions(localWallet.value.id, {
      limit: txPageSize.value,
      offset,
    })
    localWallet.value = resp.wallet
    txItems.value = resp.items
    txTotal.value = resp.total
  } catch (err) {
    log.error('加载钱包流水失败:', err)
    error(parseApiError(err, '加载钱包流水失败'))
  } finally {
    loadingTx.value = false
  }
}

function usageDateRangeParams(): { preset: string; timezone: string; tz_offset_minutes: number } {
  return {
    preset: 'last30days',
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    tz_offset_minutes: -new Date().getTimezoneOffset(),
  }
}

async function loadUsageRecords() {
  if (!props.userId || !showUsageRecords.value) {
    usageItems.value = []
    usageTotal.value = 0
    return
  }
  loadingUsage.value = true
  try {
    const offset = (usagePage.value - 1) * usagePageSize.value
    const resp = await usageApi.getAllUsageRecords({
      user_id: props.userId,
      limit: usagePageSize.value,
      offset,
      ...usageDateRangeParams(),
    })
    usageItems.value = (resp.records || []) as UsageRecord[]
    usageTotal.value = resp.total || 0
  } catch (err) {
    log.error('加载消费记录失败:', err)
    error(parseApiError(err, '加载消费记录失败'))
    usageItems.value = []
    usageTotal.value = 0
  } finally {
    loadingUsage.value = false
  }
}

async function loadPlans() {
  if (!props.userId || !showPlanRecords.value) {
    planItems.value = []
    return
  }
  loadingPlans.value = true
  try {
    const resp = await usersApi.listUserPlanEntitlements(props.userId)
    planItems.value = resp.items
  } catch (err) {
    log.error('加载套餐记录失败:', err)
    error(parseApiError(err, '加载套餐记录失败'))
    planItems.value = []
  } finally {
    loadingPlans.value = false
  }
}

async function loadRefunds() {
  if (!showRefunds.value || !localWallet.value) {
    refundItems.value = []
    refundTotal.value = 0
    return
  }
  loadingRefunds.value = true
  try {
    const offset = (refundPage.value - 1) * refundPageSize.value
    const resp = await adminWalletApi.getWalletRefunds(localWallet.value.id, {
      limit: refundPageSize.value,
      offset,
    })
    localWallet.value = resp.wallet
    refundItems.value = resp.items
    refundTotal.value = resp.total
    if (actionRefund.value) {
      const latest = refundItems.value.find((item) => item.id === actionRefund.value?.id)
      if (latest) actionRefund.value = latest
    }
  } catch (err) {
    log.error('加载钱包退款失败:', err)
    error(parseApiError(err, '加载钱包退款失败'))
  } finally {
    loadingRefunds.value = false
  }
}

function handleTxPageChange(page: number) {
  txPage.value = page
  void loadTransactions()
}

function handleTxPageSizeChange(size: number) {
  txPageSize.value = size
  txPage.value = 1
  void loadTransactions()
}

function handleUsagePageChange(page: number) {
  if (!showUsageRecords.value) return
  usagePage.value = page
  void loadUsageRecords()
}

function handleUsagePageSizeChange(size: number) {
  if (!showUsageRecords.value) return
  usagePageSize.value = size
  usagePage.value = 1
  void loadUsageRecords()
}

function handleRefundPageChange(page: number) {
  if (!showRefunds.value) return
  refundPage.value = page
  void loadRefunds()
}

function handleRefundPageSizeChange(size: number) {
  if (!showRefunds.value) return
  refundPageSize.value = size
  refundPage.value = 1
  void loadRefunds()
}

async function refreshDrawerData() {
  const tasks: Array<Promise<void>> = [loadTransactions()]
  if (showUsageRecords.value) {
    tasks.push(loadUsageRecords())
  }
  if (showPlanRecords.value) {
    tasks.push(loadPlans())
  }
  if (showRefunds.value) {
    tasks.push(loadRefunds())
  }
  await Promise.all(tasks)
}

async function submitRecharge() {
  if (!localWallet.value) return
  if (!actionAmount.value || actionAmount.value <= 0) {
    error('人工充值金额必须大于 0')
    return
  }

  const rechargeBefore = localWallet.value.recharge_balance
  const rechargeAfter = rechargeBefore + actionAmount.value
  const totalBefore = localWallet.value.balance
  const totalAfter = totalBefore + actionAmount.value
  const confirmed = await confirm({
    title: '确认人工充值',
    message: `将为 ${props.ownerName || '该钱包'} 充值 **$${formatFixed(actionAmount.value, 4)}**\n该账户**充值余额**将从 **$${formatFixed(rechargeBefore, 4)}** 变为 **$${formatFixed(rechargeAfter, 4)}**，**钱包余额**将从 **$${formatFixed(totalBefore, 4)}** 变为 **$${formatFixed(totalAfter, 4)}**`,
    confirmText: '确认充值',
    variant: 'warning',
  })
  if (!confirmed) return

  submittingMoneyAction.value = true
  try {
    const response = await adminWalletApi.rechargeWallet(localWallet.value.id, {
      amount_usd: actionAmount.value,
      payment_method: 'admin_manual',
      description: actionDescription.value || `管理员为 ${props.ownerName || '钱包'} 人工充值`,
    })
    localWallet.value = response.wallet
    success('人工充值已入账')
    resetActionForm()
    await refreshDrawerData()
    emit('changed')
  } catch (err) {
    log.error('钱包人工充值失败:', err)
    error(parseApiError(err, '人工充值失败'))
  } finally {
    submittingMoneyAction.value = false
  }
}

function previewAdjustResult(
  rechargeBefore: number,
  giftBefore: number,
  amount: number,
  balanceType: 'recharge' | 'gift'
) {
  let rechargeAfter = rechargeBefore
  let giftAfter = giftBefore

  if (amount > 0) {
    if (balanceType === 'gift') {
      giftAfter += amount
    } else {
      rechargeAfter += amount
    }
    return {
      rechargeAfter,
      giftAfter,
      totalAfter: rechargeAfter + giftAfter,
    }
  }

  let remaining = Math.abs(amount)
  const consumePositiveBucket = (value: number) => {
    const available = Math.max(value, 0)
    const used = Math.min(available, remaining)
    remaining -= used
    return value - used
  }

  if (balanceType === 'gift') {
    giftAfter = consumePositiveBucket(giftAfter)
    rechargeAfter = consumePositiveBucket(rechargeAfter)
  } else {
    rechargeAfter = consumePositiveBucket(rechargeAfter)
    giftAfter = consumePositiveBucket(giftAfter)
  }

  if (remaining > 0) {
    rechargeAfter -= remaining
  }

  return {
    rechargeAfter,
    giftAfter,
    totalAfter: rechargeAfter + giftAfter,
  }
}

async function submitAdjust() {
  if (!localWallet.value) return
  if (!actionAmount.value || actionAmount.value === 0) {
    error('调账金额不能为 0')
    return
  }

  if (isApiKeyWallet.value && adjustBalanceType.value === 'gift') {
    error('独立密钥钱包不支持赠款调账')
    return
  }

  const rechargeBefore = localWallet.value.recharge_balance
  const giftBefore = localWallet.value.gift_balance
  const currentBucketBalance = adjustBalanceType.value === 'gift' ? giftBefore : rechargeBefore
  const preview = previewAdjustResult(
    rechargeBefore,
    giftBefore,
    actionAmount.value,
    adjustBalanceType.value
  )
  const afterBalance = adjustBalanceType.value === 'gift' ? preview.giftAfter : preview.rechargeAfter
  const totalBefore = localWallet.value.balance
  const totalAfter = preview.totalAfter
  const balanceTypeLabel = adjustBalanceType.value === 'gift' ? '赠款余额' : '充值余额'
  const isDeduct = actionAmount.value < 0
  const detailLine = isDeduct
    ? `该账户**充值余额**将从 **$${formatFixed(rechargeBefore, 4)}** 变为 **$${formatFixed(preview.rechargeAfter, 4)}**，**赠款余额**将从 **$${formatFixed(giftBefore, 4)}** 变为 **$${formatFixed(preview.giftAfter, 4)}**`
    : `该账户**${balanceTypeLabel}**将从 **$${formatFixed(currentBucketBalance, 4)}** 变为 **$${formatFixed(afterBalance, 4)}**`
  const confirmed = await confirm({
    title: '确认钱包调账',
    message: `将对 ${props.ownerName || '该钱包'} 的**${balanceTypeLabel}**${actionAmount.value > 0 ? '增加' : '扣减'} **$${formatFixed(Math.abs(actionAmount.value), 4)}**\n${detailLine}，**钱包余额**将从 **$${formatFixed(totalBefore, 4)}** 变为 **$${formatFixed(totalAfter, 4)}**`,
    confirmText: '确认调账',
    variant: 'warning',
  })
  if (!confirmed) return

  submittingMoneyAction.value = true
  try {
    const response = await adminWalletApi.adjustWallet(localWallet.value.id, {
      amount_usd: actionAmount.value,
      balance_type: adjustBalanceType.value,
      description: actionDescription.value || `管理员为 ${props.ownerName || '钱包'} 执行钱包调账`,
    })
    localWallet.value = response.wallet
    success('钱包调账已完成')
    resetActionForm()
    await refreshDrawerData()
    emit('changed')
  } catch (err) {
    log.error('钱包调账失败:', err)
    error(parseApiError(err, '钱包调账失败'))
  } finally {
    submittingMoneyAction.value = false
  }
}

async function submitMoneyAction() {
  if (!isApiKeyWallet.value && moneyActionType.value === 'recharge') {
    await submitRecharge()
    return
  }
  await submitAdjust()
}

function canProcessRefund(status: string) {
  return status === 'pending_approval' || status === 'approved'
}

function canFailRefund(status: string) {
  return status === 'processing' || status === 'pending_approval' || status === 'approved'
}

function canCompleteRefund(status: string) {
  return status === 'processing'
}

async function processRefund(refund: RefundRequest) {
  if (!localWallet.value) return
  submittingRefundAction.value = true
  try {
    const resp = await adminWalletApi.processRefund(localWallet.value.id, refund.id)
    localWallet.value = resp.wallet
    success('退款已进入 processing')
    await refreshDrawerData()
    emit('changed')
  } catch (err) {
    log.error('处理退款失败:', err)
    error(parseApiError(err, '处理退款失败'))
  } finally {
    submittingRefundAction.value = false
  }
}

function openFailRefund(refund: RefundRequest) {
  refundActionType.value = 'fail'
  actionRefund.value = refund
  refundFailReason.value = ''
}

function openCompleteRefund(refund: RefundRequest) {
  refundActionType.value = 'complete'
  actionRefund.value = refund
  refundGatewayRefundId.value = ''
  refundPayoutReference.value = ''
}

async function submitFailRefund() {
  if (!localWallet.value || !actionRefund.value) return
  if (!refundFailReason.value.trim()) {
    error('请填写驳回原因')
    return
  }

  submittingRefundAction.value = true
  try {
    const resp = await adminWalletApi.failRefund(localWallet.value.id, actionRefund.value.id, {
      reason: refundFailReason.value.trim(),
    })
    localWallet.value = resp.wallet
    success('退款已驳回')
    resetRefundActionForm()
    await refreshDrawerData()
    emit('changed')
  } catch (err) {
    log.error('驳回退款失败:', err)
    error(parseApiError(err, '驳回退款失败'))
  } finally {
    submittingRefundAction.value = false
  }
}

async function submitCompleteRefund() {
  if (!localWallet.value || !actionRefund.value) return

  submittingRefundAction.value = true
  try {
    await adminWalletApi.completeRefund(localWallet.value.id, actionRefund.value.id, {
      gateway_refund_id: refundGatewayRefundId.value || undefined,
      payout_reference: refundPayoutReference.value || undefined,
    })
    success('退款已完成')
    resetRefundActionForm()
    await refreshDrawerData()
    emit('changed')
  } catch (err) {
    log.error('完成退款失败:', err)
    error(parseApiError(err, '完成退款失败'))
  } finally {
    submittingRefundAction.value = false
  }
}

function finiteNumber(value: number | null | undefined): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

const USAGE_COST_EPSILON = 0.0000001

interface UsageChargeBreakdownView {
  officialCost: number
  packageDebit: number
  packageMultiplier: number | null
  walletDebit: number
  walletMultiplier: number | null
  userDebit: number
}

function usageSalesMultiplier(record: UsageRecord): number | null {
  return finiteNumber(record.sales_multiplier)
}

function resolveUsageChargeBreakdown(record: UsageRecord): UsageChargeBreakdownView {
  const rawBreakdown = record.charge_breakdown
  const salesMultiplier = usageSalesMultiplier(record)
  const recordCost = finiteNumber(record.cost) ?? 0
  let officialCost = finiteNumber(rawBreakdown?.official_cost) ?? finiteNumber(record.official_cost)
  if (officialCost === null && salesMultiplier !== null && salesMultiplier > 0) {
    officialCost = recordCost / salesMultiplier
  } else if (officialCost === null) {
    officialCost = recordCost
  }
  const resolvedOfficialCost = officialCost ?? recordCost
  const packageDebit = Math.max(finiteNumber(rawBreakdown?.package_debit) ?? 0, 0)
  const hasBreakdown = rawBreakdown !== null && rawBreakdown !== undefined
  const walletDebit = Math.max(
    finiteNumber(rawBreakdown?.wallet_debit) ?? (hasBreakdown ? 0 : recordCost),
    0,
  )
  const userDebit = Math.max(
    finiteNumber(rawBreakdown?.user_debit) ?? (packageDebit + walletDebit),
    0,
  )
  const packageMultiplier = finiteNumber(rawBreakdown?.package_multiplier)
    ?? (packageDebit > USAGE_COST_EPSILON ? 1 : null)
  const walletMultiplier = finiteNumber(rawBreakdown?.wallet_multiplier)
    ?? salesMultiplier
    ?? (resolvedOfficialCost > USAGE_COST_EPSILON && walletDebit > USAGE_COST_EPSILON
      ? walletDebit / resolvedOfficialCost
      : null)

  return {
    officialCost: resolvedOfficialCost,
    packageDebit,
    packageMultiplier,
    walletDebit,
    walletMultiplier,
    userDebit,
  }
}

function usageUserCharge(record: UsageRecord): number {
  return resolveUsageChargeBreakdown(record).userDebit
}

function usageOfficialCost(record: UsageRecord): number {
  return resolveUsageChargeBreakdown(record).officialCost
}

function usagePackageDebit(record: UsageRecord): number {
  return resolveUsageChargeBreakdown(record).packageDebit
}

function usagePackageMultiplier(record: UsageRecord): number | null {
  return resolveUsageChargeBreakdown(record).packageMultiplier
}

function usageWalletDebit(record: UsageRecord): number {
  return resolveUsageChargeBreakdown(record).walletDebit
}

function usageWalletMultiplier(record: UsageRecord): number | null {
  return resolveUsageChargeBreakdown(record).walletMultiplier
}

function hasUsagePlatformCost(record: UsageRecord): boolean {
  return finiteNumber(record.actual_cost) !== null
}

function usagePlatformCost(record: UsageRecord): number {
  return finiteNumber(record.actual_cost) ?? usageOfficialCost(record)
}

function usageCostMultiplier(record: UsageRecord): number | null {
  const saved = finiteNumber(record.rate_multiplier)
  if (saved !== null) return saved
  const official = usageOfficialCost(record)
  if (official <= 0 || !hasUsagePlatformCost(record)) return null
  return usagePlatformCost(record) / official
}

function formatMultiplier(value: number | null): string {
  if (value === null) return '-'
  return `${value.toFixed(4).replace(/0+$/, '').replace(/\.$/, '')}x`
}

function formatUsageCostMultiplier(record: UsageRecord): string {
  return formatMultiplier(usageCostMultiplier(record))
}

function formatUsageCurrency(value: number): string {
  return formatCurrency(value)
}

function formatUsageDebitWithMultiplier(amount: number, multiplier: number | null): string {
  if (amount <= USAGE_COST_EPSILON) return '-'
  return `${formatUsageCurrency(amount)} · ${formatMultiplier(multiplier)}`
}

function usageCostTitle(record: UsageRecord): string {
  const packageDebit = usagePackageDebit(record)
  const walletDebit = usageWalletDebit(record)
  const lines = [
    `官方价格: ${formatUsageCurrency(usageOfficialCost(record))}`,
  ]
  if (packageDebit > USAGE_COST_EPSILON) {
    lines.push(`套餐扣除: ${formatUsageDebitWithMultiplier(packageDebit, usagePackageMultiplier(record))}`)
  }
  if (walletDebit > USAGE_COST_EPSILON) {
    lines.push(`钱包扣除: ${formatUsageDebitWithMultiplier(walletDebit, usageWalletMultiplier(record))}`)
  }
  if (packageDebit <= USAGE_COST_EPSILON && walletDebit <= USAGE_COST_EPSILON) {
    lines.push('本次没有产生用户扣费')
  }
  if (hasUsagePlatformCost(record)) {
    lines.push(`平台成本: ${formatUsageCurrency(usagePlatformCost(record))}`)
    lines.push(`成本倍率: ${formatUsageCostMultiplier(record)}`)
  }
  return lines.join('\n')
}

function usageStatusLabel(record: UsageRecord): string {
  if (record.status === 'failed' || (record.status_code ?? 0) >= 400) return '失败'
  if (record.status === 'cancelled') return '已取消'
  if (record.status === 'pending') return '等待中'
  if (record.status === 'streaming') return '传输中'
  return '完成'
}

function usageStatusBadge(record: UsageRecord): 'default' | 'secondary' | 'outline' | 'destructive' | 'success' {
  if (record.status === 'failed' || (record.status_code ?? 0) >= 400) return 'destructive'
  if (record.status === 'cancelled') return 'outline'
  if (record.status === 'pending' || record.status === 'streaming') return 'secondary'
  return 'success'
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return '-'
  return new Date(value).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function planStatusLabel(status: string | null | undefined): string {
  const labels: Record<string, string> = {
    active: '已发放',
    cancelled: '已取消',
    expired: '已过期',
  }
  return labels[String(status || '')] || String(status || '-')
}

function planQuotaValues(plan: BillingPlan | null | undefined): number[] {
  if (!plan) return []
  return normalizeBillingEntitlements(plan.entitlements)
    .filter((item): item is DailyQuotaEntitlement => item.type === 'daily_quota')
    .flatMap((item) => {
      const limits = item.limits || {}
      return [
        item.daily_quota_usd ?? limits.daily_limit_usd,
        item.five_hour_quota_usd ?? limits.five_hour_limit_usd,
        item.weekly_quota_usd ?? limits.weekly_limit_usd,
        item.monthly_quota_usd ?? limits.monthly_limit_usd,
      ]
    })
    .map((value) => Number(value))
    .filter((value) => Number.isFinite(value) && value > 0)
}

function planQuotaUsd(plan: BillingPlan | null | undefined): number | null {
  const values = planQuotaValues(plan)
  if (values.length === 0) return null
  return Math.max(...values)
}

function formatPlanPrice(plan: BillingPlan | null | undefined): string {
  if (!plan) return '-'
  return `${Number(plan.price_amount || 0).toFixed(2)} ${plan.price_currency || 'CNY'}`
}

function formatPlanQuota(plan: BillingPlan | null | undefined): string {
  const quota = planQuotaUsd(plan)
  return quota === null ? '无用量额度' : `$${quota.toFixed(2)}`
}

function formatPlanEquivalentMultiplier(plan: BillingPlan | null | undefined): string {
  const quota = planQuotaUsd(plan)
  if (!plan || quota === null || quota <= 0) return '等效倍率 -'
  const price = Number(plan.price_amount || 0)
  return `等效倍率 ${formatMultiplier(price / quota)}`
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
      return '会员权益'
    }
    return item.type
  })
}

function quotaEntitlementLabel(item: DailyQuotaEntitlement): string {
  const limits = item.limits || {}
  const parts = []
  const daily = Number(item.daily_quota_usd ?? limits.daily_limit_usd ?? 0)
  const fiveHour = Number(item.five_hour_quota_usd ?? limits.five_hour_limit_usd ?? 0)
  const weekly = Number(item.weekly_quota_usd ?? limits.weekly_limit_usd ?? 0)
  const monthly = Number(item.monthly_quota_usd ?? limits.monthly_limit_usd ?? 0)
  if (daily > 0) parts.push(`每日 $${daily.toFixed(2)}`)
  if (fiveHour > 0) parts.push(`5小时 $${fiveHour.toFixed(2)}`)
  if (weekly > 0) parts.push(`每周 $${weekly.toFixed(2)}`)
  if (monthly > 0) parts.push(`每月 $${monthly.toFixed(2)}`)
  const quotaText = parts.join(' / ') || '用量额度'
  const modelIds = item.allowed_global_model_ids || []
  return modelIds.length > 0 ? `${quotaText} · ${modelIds.length} 个模型` : `${quotaText} · 全部模型`
}

function toFiniteNumber(value: unknown, fallback = 0): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function formatFixed(value: unknown, digits: number): string {
  return toFiniteNumber(value).toFixed(digits)
}
</script>

<style scoped>
.drawer-enter-active,
.drawer-leave-active {
  transition: opacity 0.3s ease;
}

.drawer-enter-active .drawer-panel,
.drawer-leave-active .drawer-panel {
  transition: transform 0.3s ease;
}

.drawer-enter-from,
.drawer-leave-to {
  opacity: 0;
}

.drawer-enter-from .drawer-panel,
.drawer-leave-to .drawer-panel {
  transform: translateX(100%);
}
</style>
