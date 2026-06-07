# Codex 加密上下文保护

## 目标

阻止带 Codex 加密上下文的请求在无法确认原上游账号时继续转发，避免同一段上下文被多个上游账号解密失败，进而制造大量 400 错误。

## 非目标

- 不尝试破解、解析或改写 Codex 的 `encrypted_content`。
- 不把 `encrypted_content` 自身当成稳定会话编号。
- 不改变普通 OpenAI Responses 请求的调度逻辑。

## 行为变化

- Codex 请求如果包含 `encrypted_content`，必须同时带有明确会话标识。
- 明确会话标识包括 `x-aether-session-id`、`session_id`、`conversation_id`、`prompt_cache_key` 或已有通用会话字段。
- 如果没有明确会话标识，网关直接返回 400，错误提示用户重新开始会话或让客户端发送 `x-aether-session-id`。
- 这类请求不会进入上游账号选择，也不会继续尝试其他账号。

## 影响范围

- 只影响 Codex 客户端发出的 OpenAI Responses 请求。
- 不影响普通无加密上下文的请求。
- 不影响 Claude Code、OpenCode、Gemini CLI 的既有会话识别。

## 验证方式

- 单元测试覆盖 Codex 加密上下文无会话标识时必须拦截。
- 单元测试覆盖带 `x-aether-session-id` 时允许继续。
- 单元测试覆盖非 Codex 请求不受影响。
