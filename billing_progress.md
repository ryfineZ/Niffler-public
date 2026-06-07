# 计费链路排查进度

## 2026-06-02
- 开始排查模型定价、token 计算、基础价格、倍率、套餐额度和实际扣费链路。
- 已确认当前工作区存在两处原有未提交后端改动，本次排查先只读，不触碰。

## 2026-06-03
- 继续排查计费全链路，确认扣费主链路按“套餐基础成本优先、钱包销售价补扣”执行。
- 修复 Provider Codex 5h/weekly 窗口统计，把未结算、失败和零金额请求排除在窗口用量外。
- 调整钱包页按日用量汇总文案，避免把含套餐额度的用量金额误读为钱包扣款。
- 线上只读抽样 `rn01 / niffler-postgres / aether`，确认 `22.1M tokens / $14.12` 这类低金额样本主要由 cache read 高占比和低销售倍率造成，不是大量 pending、void 或零费用 usage 漏掉。
- 抽样账号 `codex_plenty_beyond_8v+g4@icloud.com` 的 5H 窗口有 444 条 usage，442 条为已结算正费用；cache read 占比约 91%，其中部分 `gpt-5.5` 请求来自 0.15 或 0.05 倍率用户组。
- 查到官方 5H 满额但按 `reset_at - 5h` 查不到本地 usage 的账号，原因是官方 `reset_at` 和本地 usage 时间不一定对齐；不能把官方 reset_at 直接当成本地用量窗口结束时间。
- 修正号池管理页窗口费用口径：Provider Codex 5H/weekly 窗口不再汇总带用户分组销售倍率的 `usage_billing_facts.total_cost_usd`，改为优先汇总结算快照里的 `base_cost_usd`；历史数据缺少快照时才退回旧字段。
- 确认 `cache_ttl_minutes` 是 provider 账号级缓存创建 TTL 字段，本轮按 1h 缓存创建口径把新账号默认值改为 60，并准备把线上已有账号批量改为 60。
- 线上 `rn01 / niffler-postgres / aether` 已把 33 个已有 provider 账号的 `cache_ttl_minutes` 批量改为 60，并把线上 `provider_api_keys.cache_ttl_minutes` 列默认值改为 60；复查结果为 `60|33`，列默认值为 `60`。

## 验证记录
- `cargo test -p aether-gateway gateway_pool_list_overrides_stale_codex_cycle_usage_from_usage_facts` 通过。
- `cargo test -p aether-gateway gateway_wallet_flow_today_entry_uses_live_settled_usage` 通过。
- `cargo test -p aether-gateway gateway_handles_wallet_today_cost_locally_without_proxying_upstream` 通过。
- `cargo test -p aether-data usage_sql_rebuilds_provider_key_window_usage_into_status_snapshot` 通过。
- `cargo test -p aether-data summarizes_provider_api_key_window_usage_with_zero_rows` 通过。
- `cargo test -p aether-data provider_api_key_window -- --nocapture` 通过。
- `cargo test -p aether-data usage_sql_rebuilds_provider_key_window_usage_into_status_snapshot -- --nocapture` 通过。
- `cargo fmt --check -p aether-data` 通过。
- `npm run type-check` 通过。
- `git diff --check` 通过。
- `cargo test -p aether-gateway provider_key_concurrent_limit_create_and_list_responses` 通过。
- `npm run test:run -- provider-key-concurrent_limit.spec.ts` 通过。
- `cargo fmt --check` 通过。
