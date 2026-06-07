# Progress Log: sub2api 到 Niffler 数据同步调研

## 2026-05-27

### Phase 1: 数据源定位
- 开始只读盘点 tc-jp 上的 sub2api 数据。
- 已确认 tc-jp 的 sub2api 应用容器、Postgres 容器、数据库名和关键表。
- 已读取用户、API Key、分组、订阅和 Codex 账号池数据。
- 已确认 Niffler 目标库当前只有管理员用户和管理员钱包，没有用户 API Key、上游账号和套餐数据。
- 没有向 Niffler 写入任何数据。

### Phase 2: 用户数据盘点
- 有效用户 31 个，有效 API Key 33 个。
- 密码字段是 bcrypt 哈希，不能还原明文密码。
- API Key 以明文保存在 sub2api，但调查记录和聊天输出只展示脱敏值。

### Phase 3: Codex 账号池盘点
- Codex 分组有效账号 42 个，全部是 OpenAI OAuth 账号。
- 计划类型：free 10 个，plus 31 个，team 1 个。

### Phase 4: Niffler 映射研究
- 已确认 Niffler 可承接用户、钱包、API Key、套餐权益和 Codex OAuth 账号。
- 关键限制：API Key、OAuth 凭证必须按 Niffler 规则重新加密或重新计算哈希，不能原样复制。
- 用户确认同步时不停止 sub2api；同步期间继续产生的消费不用追平，视为赠送。

## Test Results
| Test | Result |
|------|--------|
| tc-jp sub2api 只读连接 | 通过 |
| rn01 Niffler 目标库只读连接 | 通过 |
| 同步脚本试跑 | 通过，未写库 |
| 正式写入 Niffler | 通过 |
| 写入后数量校验 | 通过：31 个用户、33 个用户 API Key、42 个 Codex OAuth 账号、2 个套餐、1 个有效会员 |
| 加密字段解密校验 | 通过：用户 API Key 和 Codex OAuth 配置均可用 Niffler 服务密钥解开 |
| hd0526 服务健康检查 | 通过：`/health` 连续两次返回 200 |

## Error Log
| Error | Resolution |
|-------|------------|
| Niffler 数量查询中远程 shell 引号错误 | 改用固定 `postgres/aether` 连接参数重新查询 |
| 脚本远程 SQL 字符串引号被 shell 吃掉 | 改成远程 shell 安全引号 |
| 分组写入缺少 `created_at` | 补充读取源库分组创建和更新时间 |
| 源库用户名和 Niffler 本地管理员重名 | 导入登录名统一加源库用户编号，避免重名 |
| 部分导入 ID 超过目标库 36 位限制 | 缩短接口和订单 ID |

### Phase 5: 正式同步完成
- 已执行 `scripts/oneoff/sync_sub2api_to_niffler.py --apply`。
- Niffler 目标库当前共有 32 个用户，其中 31 个来自 sub2api，另 1 个是 Niffler 本地管理员。
- 已导入 31 个用户钱包、33 个用户 API Key、5 个 sub2api 分组、1 个 Codex provider、1 个 Codex endpoint、42 个 Codex OAuth 账号、2 个套餐、1 个有效套餐权益。
- 42 个 Codex OAuth 账号中，41 个在 Niffler 中启用，1 个源库已是错误状态，已保留但停用。
- 33 个用户 API Key 均已绑定到 Codex provider。
- 不展示、不记录任何完整 API Key、OAuth token 或加密密钥。
