# Findings: sub2api 到 Niffler 数据同步调研

## 数据源
- tc-jp 上运行的是 `/opt/sub2api-niffler` 里的 sub2api。
- 应用容器：`sub2api-dev`。
- 数据库容器：`sub2api-postgres-dev`。
- 数据库：Postgres，库名 `sub2api`，用户 `sub2api`。
- 只读查询时间：2026-05-27 18:45:27 +08。

## 用户数据
- 有效用户：31 个。
- 有效 API Key：33 个。
- 密码字段是 `password_hash`，不是明文密码。当前有 `$2a$10$...` 和 `$2b$12$...` 两类 bcrypt 哈希。
- sub2api 用户余额在 `users.balance`，已充值总额在 `users.total_recharged`。
- sub2api 用户级限制包括 `users.concurrency` 和 `users.rpm_limit`。
- API Key 明文保存在 `api_keys.key`，聊天中只展示脱敏值。
- API Key 分组统计：
  - Codex：27 条
  - 下游中转：3 条
  - 自用：2 条
  - GPT包月套餐：1 条
- 有效套餐记录：`Zaoyoe / zaoyoe-8359174f@aether.local` 有 `GPT包月套餐`，状态 active，到期 `2026-06-15 11:03 +08`。
- 另有 `yunshu` 的 `日套餐` 已过期，不应作为有效套餐导入。

### 用户清单（脱敏）
| ID | 用户名 | 邮箱 | 角色 | 余额 | 并发 | 每分钟请求 | 密码哈希 | API Key | 有效套餐 |
|---:|---|---|---|---:|---:|---:|---|---|---|
| 1 | admin | admin@niffler.org | admin | 6720.87968230 | 100 | 0 | `$2a$10$...pm/6` | admin:`sk-bbbed1b...16c4`[自用] | 无 |
| 2 | 18318999155 | 18318999155-aec2a605@aether.local | user | 83.39748050 | 2 | 7 | `$2b$12$...h/y6` | Key-2026-05-13:`sk-8333X1h...V5Re`[Codex] | 无 |
| 3 | admin | admin@example.com | admin | 0.00000000 | 5 | 0 | `$2b$12$...c1Pe` | cc-switch-niffler-local:`sk-jT9uxZn...A1JD`[Codex] | 无 |
| 4 | Dreamwalker | dreamwalker-dc5fe7e0@aether.local | user | 299.99773000 | 2 | 8 | `$2b$12$...b0ue` | 七星幻月:`sk-i0DuosQ...LQtb`[Codex] | 无 |
| 5 | hhh | hhh-90f58958@aether.local | user | 219.50833900 | 2 | 8 | `$2b$12$...UlpS` | Key-2026-05-08:`sk-wp0Li1B...ZvVX`[Codex] | 无 |
| 6 | huiyouyongdeyu | huiyouyongdeyu-f34f6444@aether.local | user | 171.56424200 | 2 | 7 | `$2b$12$...74/O` | 1:`sk-5BcXTnD...qJTE`; 2:`sk-rYly0zj...lX0g`[Codex] | 无 |
| 7 | Invictus | invictus-4ba2624a@aether.local | user | 38.69027850 | 2 | 8 | `$2b$12$...3gPq` | key:`sk-9Yi4YqJ...QPBB`[Codex] | 无 |
| 8 | Jeremy | jeremy-1f30305a@aether.local | user | 19.91706020 | 2 | 8 | `$2b$12$...IrV2` | Key-2026-05-10:`sk-tIkJzn8...sEYM`[Codex] | 无 |
| 9 | LiLi | lili-8b1b0360@aether.local | user | 200.00000000 | 2 | 7 | `$2b$12$...BjEG` | Key-2026-05-11:`sk-cwXDFQQ...k9wa`[Codex] | 无 |
| 10 | moximoxi | moximoxi-5b864017@aether.local | user | 200.00000000 | 2 | 8 | `$2b$12$...Sz2.` | Key-2026-05-10:`sk-SGTbi92...AyDt`[Codex] | 无 |
| 11 | Nicky | nicky-507fad7e@aether.local | user | -0.05435100 | 5 | 0 | `$2b$12$...iv4y` | Key-2026-05-14:`sk-MH4PlMQ...jHWx`[Codex] | 无 |
| 12 | remember | remember-ea5b0b36@aether.local | user | 197.55014890 | 1000 | 0 | `$2b$12$...pXmS` | Key-2026-05-09:`sk-fu5TeKU...haoS`[下游中转] | 无 |
| 13 | shenghuolequ | shenghuolequ-39f488d1@aether.local | user | 356.03569240 | 2 | 8 | `$2b$12$...O7ce` | shenghuolequ:`sk-OnUjxBd...brrb`[Codex] | 无 |
| 14 | test_dudu | test_dudu-399fbb97@aether.local | user | 92.02801232 | 2 | 8 | `$2b$12$...OEka` | 测试:`sk-6Vo32SZ...09Y4`[Codex] | 无 |
| 15 | txj123 | txj123-cb0ff2cf@aether.local | user | 211.80661925 | 2 | 7 | `$2b$12$...LN06` | Key-2026-05-11:`sk-cAggEBv...0Lpl`[Codex] | 无 |
| 16 | Will | will-590922ef@aether.local | user | 13438.70714470 | 5000 | 0 | `$2b$12$...OrCO` | Key-2026-05-07:`sk-GTG6ZIt...1fR8`[下游中转] | 无 |
| 17 | wocao111 | wocao111-dd16e772@aether.local | user | 963.44386315 | 2 | 7 | `$2b$12$...MyCK` | 1111:`sk-2RpVS2n...wSv5`; Key-2026-05-12:`sk-CnQL0kd...QjlC`[Codex] | 无 |
| 18 | xiamoyanyu | xiamoyanyu-50cae1c8@aether.local | user | 31.09656570 | 5000 | 0 | `$2b$12$...MVO.` | Key-2026-05-12:`sk-QuYsVL5...OHwD`[下游中转] | 无 |
| 19 | yunshu | yunshu-e094e19a@aether.local | user | 102.17575840 | 2 | 8 | `$2b$12$...J1YG` | Key-2026-05-10:`sk-UfuAZhk...guRg`[Codex] | 无 |
| 20 | zan_max | zan_max-55068632@aether.local | user | 155.31124825 | 2 | 8 | `$2b$12$...ZWY.` | Key-2026-05-09:`sk-0mCLfvb...e8Ui`[Codex] | 无 |
| 21 | Zaoyoe | zaoyoe-8359174f@aether.local | user | 4698.08690345 | 2 | 8 | `$2b$12$...ciAa` | my:`sk-521db4a...ddb5`[GPT包月套餐] | GPT包月套餐，到期 2026-06-15 11:03 |
| 22 | - | 2486174753@qq.com | user | 325.65403010 | 2 | 7 | `$2a$10$...ULbW` | win10:`sk-60a3791...da5c`; win11:`sk-b349a4c...0fed`; 主机:`sk-925ca7f...69af`[Codex] | 无 |
| 23 | 676704649@qq.com | 676704649@qq.com | user | 436.56466200 | 10 | 0 | `$2a$10$...uVjW` | niu:`sk-83496e1...70c5`[Codex] | 无 |
| 24 | - | 1007801860@qq.com | user | 946.50176400 | 5 | 0 | `$2a$10$...3Fdm` | H:`sk-2b6f8a7...ba25`[Codex] | 无 |
| 25 | baba@baba.baba | baba@baba.baba | user | 100.00000000 | 1 | 0 | `$2a$10$...VU8y` | 无 | 无 |
| 26 | - | 15032224056@163.com | user | 42.68873900 | 1 | 0 | `$2a$10$...qYH6` | 开发测试:`sk-8b00732...72b5`[Codex] | 无 |
| 27 | - | jibai@jibai.jibai | user | 67.38358810 | 100 | 0 | `$2a$10$...cOh2` | yue:`sk-b6e0a8d...d405`[Codex] | 无 |
| 28 | - | houzesheng@houzesheng.houzesheng | user | 100.00000000 | 100 | 0 | `$2a$10$...E5Q2` | 无 | 无 |
| 29 | - | mala@mala.mala | user | 20.00000000 | 100 | 0 | `$2a$10$...6pzu` | Codex:`sk-b354163...9b57`[Codex] | 无 |
| 30 | - | songxin@songxin.songxin | user | 99.53749610 | 100 | 0 | `$2a$10$...vU.W` | songxin@songxin.songxin:`sk-0364ffc...e518`[自用] | 无 |
| 31 | - | 2787075106@qq.com | user | 90.00000000 | 100 | 0 | `$2a$10$...Z5l6` | 1:`sk-8416a25...1be2`[Codex] | 无 |

## Codex 账号池
- Codex 分组下有效账号：42 个。
- 全部是 `openai / oauth / active`，不是普通 OpenAI API Key。
- 计划类型统计：
  - free：10 个
  - plus：31 个
  - team：1 个
- 分组账号数量：
  - Codex：42 个
  - 下游中转：42 个
  - GPT包月套餐：39 个
  - 日套餐：39 个
  - 自用：35 个
- Codex 账号的敏感凭证在 `accounts.credentials` JSON 里，包括 `access_token`、`refresh_token`、`id_token`、`client_id` 等。不能在聊天里完整展示。
- 每个 Codex 账号还带有额度快照字段，例如 5 小时用量百分比、7 天用量百分比、套餐类型、隐私模式状态。

### Codex 账号清单（不含 token）
| ID | 账号 | 类型 | 并发 | 优先级 | 5小时用量 | 7天用量 | 隐私模式 |
|---:|---|---|---:|---:|---:|---:|---|
| 256 | xa@4.dododo.edu.pl | plus | 5 | 0 | 1% | 100% | - |
| 227 | n@dododo.edu.pl | team | 10 | 1 | 2% | 88% | training_off |
| 236 | ly@cd.dododo.edu.pl | plus | 5 | 1 | 37% | 100% | - |
| 241 | zo@07.dododo.edu.pl | plus | 5 | 1 | 17% | 100% | - |
| 242 | xmx@5m.dododo.edu.pl | plus | 5 | 1 | 24% | 92% | - |
| 243 | 0c4@u.dododo.edu.pl | plus | 5 | 1 | 64% | 99% | - |
| 245 | a5b@pa0.dododo.edu.pl | plus | 5 | 1 | 1% | 100% | - |
| 254 | q@s.dododo.edu.pl | plus | 5 | 1 | 93% | 89% | - |
| 257 | tq@8.dododo.edu.pl | plus | 5 | 1 | 4% | 96% | - |
| 258 | 5yer@4.dododo.edu.pl | plus | 5 | 1 | 17% | 94% | - |
| 270 | wen26051502@163.com | plus | 10 | 1 | 6% | 100% | training_off |
| 271 | 6z2n@5.dododo.edu.pl | plus | 5 | 1 | 2% | 53% | - |
| 275 | at@edu.dododo.edu.pl | plus | 3 | 1 | 7% | 34% | - |
| 320 | a@edu.cyber-cyber.org | plus | 10 | 1 | 12% | 35% | training_off |
| 335 | uh6@edu.cyber-cyber.net | plus | 10 | 1 | 1% | 59% | training_off |
| 348 | m3@edu.dododo.edu.pl | plus | 10 | 1 | 29% | 23% | training_off |
| 349 | i8@edu.no3realms.com | plus | 10 | 1 | 15% | 41% | training_off |
| 350 | cp@edu.no3realms.com | plus | 10 | 1 | 70% | 40% | training_off |
| 351 | p7@edu.cyber-cyber.org | plus | 10 | 1 | 40% | 23% | training_off |
| 352 | oa@edu.dodododo.org | plus | 10 | 1 | 21% | 12% | training_set_failed |
| 353 | 7@edu.dododo.edu.pl | plus | 10 | 1 | 1% | 13% | training_set_failed |
| 354 | 4p@edu.cyber-cyber.net | plus | 10 | 1 | 31% | 20% | training_set_failed |
| 355 | a64g@edu.cyber-cyber.org | plus | 10 | 1 | 1% | 34% | training_set_failed |
| 356 | ua@edu.dodododo.org | plus | 10 | 1 | 20% | 24% | training_off |
| 357 | o3qz@edu.cyber-cyber.org | plus | 10 | 1 | 14% | 14% | training_off |
| 358 | m25g@edu.dodododo.org | plus | 10 | 1 | 79% | 17% | training_off |
| 359 | 8@edu.cyber-cyber.org | plus | 10 | 1 | 26% | 15% | training_off |
| 360 | wen00241621@163.com | plus | 10 | 1 | 70% | 11% | training_off |
| 361 | 4u2@edu.dodododo.org | plus | 10 | 1 | 100% | 16% | training_set_failed |
| 362 | na7@edu.cyber-cyber.net | plus | 10 | 1 | 62% | 10% | training_set_failed |
| 363 | tdgc@edu.dodododo.org | plus | 10 | 1 | 40% | 6% | training_set_failed |
| 4 | ryfine@139.com | free | 3 | 2 | 0% | 100% | - |
| 16 | moumoushu@hotmail.com | free | 3 | 2 | 0% | 100% | training_off |
| 20 | ryfine.sanjiezhiwai@gmail.com | free | 3 | 2 | 0% | 100% | training_off |
| 21 | ryfine2025@gmail.com | free | 3 | 2 | 0% | 100% | training_off |
| 22 | ryfine2026@hotmail.com | free | 3 | 2 | 0% | 97% | - |
| 23 | ryfine40@gmail.com | free | 3 | 2 | 0% | 100% | - |
| 24 | ryfine@163.com | free | 3 | 2 | 0% | 100% | - |
| 25 | shuangwenzhou069@gmail.com | free | 3 | 2 | 0% | 100% | - |
| 29 | v2x@c.7pg.niffler.org | plus | 3 | 2 | 100% | 78% | - |
| 31 | zhangyufanfanfan@gmail.com | free | 3 | 2 | 0% | 100% | - |
| 69 | pgwj@4e.dodododo.org | free | 3 | 2 | 0% | 100% | training_off |

## Niffler 映射
- 同步时不停止 tc-jp 上的 sub2api。以执行同步时读到的数据为准；同步开始后 sub2api 里继续产生的消费不再追平，视为赠送给用户。
- Niffler 目标库在 rn01 的 `niffler-postgres`，库名 `aether`，用户 `postgres`。
- Niffler 当前目标数据很干净：
  - `users`：1 条，仅管理员 `admin@niffler.local`
  - `wallets`：1 条
  - `api_keys`：0 条
  - `providers`：0 条
  - `provider_api_keys`：0 条
  - `billing_plans`：0 条
  - `user_plan_entitlements`：0 条
  - `api_key_provider_mappings`：0 条
- 用户可映射到 Niffler `users`。
- 余额可映射到 Niffler `wallets.balance` 和 `wallets.total_recharged`。
- 用户 API Key 可映射到 Niffler `api_keys`，需要将 sub2api 明文 key 计算 SHA-256 写入 `key_hash`，并按 Niffler 加密方式写入 `key_encrypted`。
- sub2api 的 `users.rpm_limit` 可映射到 Niffler `users.rate_limit`；sub2api 的 `users.concurrency` 应映射到 Niffler 用户分组或 API Key 并发设置，取决于最终产品策略。
- Codex 账号可映射到 Niffler `providers` + `provider_api_keys`：
  - 创建一个 Codex provider，`provider_type='codex'`。
  - 每个 sub2api Codex 账号写成一条 `provider_api_keys`。
  - `auth_type='oauth'`。
  - OAuth JSON 需要用 Niffler 的 Fernet 加密方式写入 `auth_config`，不能原样复制。
  - `concurrency` 可映射到 `provider_api_keys.concurrent_limit`。
  - `priority` 可映射到 `provider_api_keys.internal_priority`。
  - `extra` 中的额度快照可放到 `upstream_metadata`、`status_snapshot` 或 `metadata`。
- 套餐可映射到 Niffler `billing_plans` 和 `user_plan_entitlements`：
  - `GPT包月套餐`：每周 2000 USD、5 小时 200 USD、rpm 8。
  - `日套餐`：每天 100 USD，当前有效订阅为 0。
  - sub2api `subscription_plans` 是空表，套餐来源实际是 `groups` 和 `user_subscriptions`。
