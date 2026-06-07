# Provider OAuth Client 配置

## 目标

移除代码仓库中内置的第三方 OAuth Client ID / Secret。Provider OAuth 授权和 Refresh Token 刷新必须使用管理员配置的 OAuth Client，避免公开镜像仓库包含第三方凭据，也避免不同部署共用同一客户端。

## 非目标

- 不新增数据库表。
- 不改变 Codex、ChatGPT Web、Claude Code 现有公开客户端授权行为。
- 不扩展本次前端表单；管理员可先通过 Provider 配置 JSON 写入。

## 行为变化

- `gemini_cli`、`antigravity` 不再带默认 Google OAuth Client。
- 这两类 Provider 发起授权、Refresh Token 导入、后台自动刷新时，必须能从 Provider 配置或账号加密 `auth_config` 读取 `client_id` 和 `client_secret`。
- 授权或导入成功后，后台会把实际使用的 `client_id` / `client_secret` 写入账号加密 `auth_config`，后续自动刷新不依赖页面状态。
- 未配置时，后台直接返回明确错误，不会用空客户端请求上游。

## 配置格式

推荐在 Provider `config` 中写入：

```json
{
  "oauth_client": {
    "client_id": "your-google-oauth-client-id",
    "client_secret": "your-google-oauth-client-secret"
  }
}
```

兼容字段名：`client_id`、`clientId`、`oauth_client_id`、`oauthClientId`，以及嵌套对象 `oauth_client`、`oauthClient`、`provider_oauth`、`providerOAuth`、`google_oauth`、`googleOAuth`、`oauth`。

## 影响范围

- Provider OAuth 授权开始接口。
- Provider OAuth 授权完成接口。
- Provider OAuth Refresh Token 单条/批量导入。
- 本地 OAuth 自动刷新。
- 固定 Provider OAuth 模板。

## 验证方式

- 相关 OAuth 单测必须覆盖：未配置时报错、从 Provider 配置读取、授权成功后保存客户端字段、自动刷新读取账号 `auth_config`。
- 公开镜像推送前必须确认仓库内不再包含真实 Google OAuth Client ID / Secret。
