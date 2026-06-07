# Task Plan: sub2api 到 Niffler 数据同步调研

## Goal
只盘点 tc-jp 上 sub2api 的用户数据、API Key、余额、套餐类型和 Codex 账号池，研究同步到 Niffler 的方案。当前阶段不写入 Niffler。

## Current Phase
Phase 1

## Phases

### Phase 1: 数据源定位
- [x] 连接 tc-jp
- [x] 找到 sub2api 容器、数据库、配置和数据表
- [x] 确认是否有可读凭据和数据库类型
- **Status:** completed

### Phase 2: 用户数据盘点
- [x] 列出用户：用户名、密码字段形式、API Key、余额、套餐类型
- [x] 对敏感值脱敏展示
- [x] 记录字段含义和单位
- **Status:** completed

### Phase 3: Codex 账号池盘点
- [x] 列出 Codex 账号
- [x] 记录账号状态、额度、分组、代理或绑定信息
- [x] 对 token/密钥脱敏展示
- **Status:** completed

### Phase 4: Niffler 映射研究
- [x] 查 Niffler 用户、API Key、钱包、套餐、供应商账号结构
- [x] 对比 sub2api 字段和 Niffler 字段
- [x] 给出同步步骤、风险和需要确认的点
- **Status:** completed

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| 先只读盘点，不写入 Niffler | 用户明确要求不要直接开干 |
| 密码和 API Key 默认脱敏 | 避免在聊天中泄露可用凭据 |
| 同步时不停止 sub2api | 用户已确认同步期间产生的消费视为赠送给用户，不需要追平 |

## Known Facts
- Niffler 当前数据层在 rn01，应用层在 hd0526。
- 用户要求数据源是 tc-jp 上的 sub2api。
- tc-jp sub2api 当前有效用户 31 个，有效 API Key 33 个，Codex 账号 42 个。
- rn01 Niffler 目标库当前只有 1 个管理员用户和 1 个钱包，无上游账号、无用户 API Key、无套餐。
- 后续执行同步时，按执行时刻重新读取一次源库数据作为迁移基准。

## Errors Encountered
| Error | Resolution |
|-------|------------|
