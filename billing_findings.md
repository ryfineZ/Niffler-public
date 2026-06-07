# 计费链路排查发现

## 已确认
- 计费事件在终端用量事件生成后进入 `aether-billing` 补价：`enrich_usage_event_with_billing` 根据 provider、provider_api_key、model/global model 查询价格上下文。
- 模型价格优先级为 provider model 覆盖价，其次 global model 默认价；价格配置来自 `tiered_pricing` 和 `price_per_request`。
- 金额字段含义已确认：`base_cost_usd` 是按模型价格算出的基础成本；`total_cost_usd` 是 `base_cost_usd * sales_multiplier` 后的用户应扣金额；`actual_total_cost_usd` 是 `base_cost_usd * cost_multiplier/rate_multiplier` 后的实际成本。
- 结算时套餐额度先按 `base_cost_usd` 消耗；钱包只补扣套餐未覆盖部分，并对补扣部分乘 `sales_multiplier`。
- 失败请求不会计费；completed 和 cancelled 会进入 settled，failed 会进入 void。
- 用户钱包余额里的 `daily_quota.used_usd/remaining_usd/package_balance` 来自 `entitlement_usage_windows.used_usd`，这是套餐额度消耗口径。
- Provider Codex 账号的 5h/weekly 窗口统计来自 `provider_api_keys.status_snapshot.quota.windows[*].usage`，重建 SQL 汇总 `usage_billing_facts.total_cost_usd`。
- `usage_billing_facts.total_cost_usd` 优先读 `usage_settlement_snapshots.billing_total_cost_usd`，也就是用户售价；`actual_total_cost_usd` 才是 provider 实际成本。
- 销售倍率从 API key group 快照进入认证上下文，再进入执行 report context，最终允许写入 usage metadata；代码链路没有看到默认丢失倍率。
- Provider Codex 账号 5h/weekly 窗口统计原先没有过滤结算状态，可能把 pending/streaming/void 请求计入 request_count 和 total_tokens；已改为只统计 `billing_status = 'settled'` 且 `total_cost_usd > 0` 的用量。
- 钱包页“资金流水”里混入按日用量汇总，原先用负数展示 total_cost，容易被误读为钱包真实扣款；已改为“资金与用量”，按日汇总显示“用量金额，含套餐额度”。
- 线上抽样确认，截图中类似 `22.1M tokens / $14.12` 的 Codex 5H 窗口不是大量漏结算导致：样本 key `codex_plenty_beyond_8v+g4@icloud.com` 在 2026-06-02 21:17:36 +08 到 2026-06-03 02:17:36 +08 的窗口内有 444 条 usage，其中 442 条为已结算正费用。
- 同一样本的低金额主要由两类因素共同造成：一是 cache read 占比约 91%，`gpt-5.4` cache read 价为 0.25 USD/1M tokens，`gpt-5.5` cache read 价为 0.5 USD/1M tokens；二是部分用户 API key 绑定低销售倍率用户组，例如 `Codex 0.15倍率 CC 2.1倍率` 和 `下游0.05`。
- 样本中 `gpt-5.5` 的 `sales_multiplier=0.15` 来自用户 API key 所属用户组 `Codex 0.15倍率 CC 2.1倍率`，不是代码随机丢倍率；`sub2api-user-19` 的 API key 绑定该组，`gm-gpt-55` 的模型销售倍率为 0.15。
- 同一样本按用户组聚合：`下游中转` 组倍率 1 的 `gpt-5.5` 贡献约 $5.007638；`下游0.05` 组倍率 0.05 的 `gpt-5.5` 基础/实际成本约 $3.427119，但用户侧费用约 $0.171356；`Codex 0.15倍率 CC 2.1倍率` 组基础/实际成本约 $0.339653，用户侧费用约 $0.050948。
- 号池管理页面的账号窗口费用不应该受用户分组销售倍率影响。当前窗口统计汇总 `usage_billing_facts.total_cost_usd`，该字段是每条请求已写好的用户侧金额，已经包含销售倍率；因此这个页面使用该字段属于展示口径错误。
- 号池管理页面应展示账号基础价格费用：优先汇总 `usage_settlement_snapshots.settlement_snapshot.base_cost_usd`，用于反映请求按模型基础价格计算出的账号用量金额；旧数据缺少该快照时再退回 `usage_billing_facts.total_cost_usd`。
- 另有官方 5H 满额但按 `reset_at - 5h` 查不到本地 usage 的账号，例如 `codex_3oq2@niffler.org`：本地 usage 集中在 2026-06-02 15:24:46 +08 到 16:14:36 +08，总计 370 条、50.1M tokens、$30.620307；官方 quota 快照更新时间是 2026-06-03 00:54:19 +08，5H reset_at 是 2026-06-03 03:38:10 +08。这说明官方 5H reset_at 不能简单等同于“本地统计窗口结束时间”，窗口回推会和本地 usage 时间错位。

## 疑点
- Plus 账号 5 小时限额统计只有几美元。
- 用户消费比以前低。
- 如果 Plus 5h 额度显示读取套餐窗口 `used_usd`，它天然按 `base_cost_usd` 统计，不按用户销售价统计；当销售倍率大于 1 时，显示金额会低于用户实际扣费口径。
- 如果近期模型基础价格下调、provider 覆盖价缺失回落到更低的 global 默认价，或 `sales_multiplier`/模型销售倍率变小，都会让用户消费下降。
- 如果你说的“Plus 账号 5h 限额”是 Codex provider 账号窗口，而不是用户套餐额度，那么当前窗口金额已经是用户售价口径；只有几美元更可能来自模型价格/倍率/窗口范围/实际用量，而不是套餐基础价口径。
- 线上抽样已经能解释截图里的低费用：cache read 高占比叠加低销售倍率。产品口径已确认：Provider Codex 5H 窗口应展示账号基础价格费用，不能展示带用户分组倍率的 `total_cost_usd`。
- 官方 quota 的 `reset_at` 更像账号侧限额恢复时间，不一定适合作为本地 usage 窗口的唯一边界；若要准确解释“官方 5H 用完对应本地哪些请求”，应优先按 `last_used_at` 或 quota refresh 时间附近回看，而不是只按 `reset_at - window_minutes`。
- 钱包日用量聚合目前按 `usage_billing_facts.total_cost_usd` 汇总，表示用量金额，不等同于钱包实际扣款；如果产品想展示“真实钱包扣款”，需要改成基于 settlement 钱包余额差额或 `settlement_wallet_debit_usd` 汇总。
- In-memory settlement 仓库仍按 `total_cost_usd` 直接扣钱包，没有实现套餐额度抵扣逻辑；生产 Postgres/MySQL/SQLite 结算已实现套餐优先，但内存实现会影响测试或本地内存模式口径。

## 证据
- `crates/aether-billing/src/event_enrichment.rs`：`apply_billing_computation` 写入 `base_cost_usd`、`user_total_cost_usd`、`actual_total_cost`。
- `crates/aether-billing/src/pricing.rs`：`effective_tiered_pricing`、`effective_price_per_request` 定义模型价格优先级；`actual_cost_multiplier_for_api_format` 定义实际成本倍率优先级。
- `crates/aether-data/src/repository/settlement/postgres.rs`：`consume_daily_quota_postgres` 入参使用 `input.base_cost_usd`；钱包补扣使用 `(input.base_cost_usd - quota.debited_usd) * sales_multiplier`。
- `crates/aether-data/src/repository/settlement/mod.rs`：`settlement_wallet_charge_multiplier` 从 `total_cost_usd / base_cost_usd` 反推销售倍率。
- `apps/aether-gateway/src/handlers/public/support/wallet/reads.rs`：钱包余额接口把 quota availability 映射为 `daily_quota` 和 `package_balance`。
- `crates/aether-data/src/repository/usage/postgres/queries/rebuild_provider_api_key_codex_window_usage_stats_sql.sql`：Codex 窗口统计按 provider key 和窗口时间汇总 `usage_billing_facts.total_cost_usd`。
- `crates/aether-data/src/repository/usage/postgres/queries/summarize_provider_api_key_window_usage_sql.sql`：管理端按窗口实时汇总 provider key 用量。
- `apps/aether-gateway/src/ai_serving/mod.rs`、`apps/aether-gateway/src/control/auth/resolution.rs`：认证上下文携带 `sales_multiplier` 和 `model_sales_multipliers`。
