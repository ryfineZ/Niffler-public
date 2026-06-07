# Codex Responses 字符串输入兼容

## 目标

当客户端调用 OpenAI Responses 接口且最终走 Codex OAuth 上游时，兼容 `input` 为字符串的请求，避免 ChatGPT Codex 内部接口返回 `Input must be a list`。

## 非目标

- 不改变普通 OpenAI API Key 上游的请求格式。
- 不改变图片生成、工具调用、模型映射和计费逻辑。
- 不把所有 Responses 请求强制改写，只处理 Codex 上游不接受的字符串输入。

## 行为变化

发往 Codex OAuth 上游前，如果请求体里存在非空字符串 `input`，系统会改成一条用户消息列表：

```json
{
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": "原始输入文本"
    }
  ]
}
```

如果字符串只包含空白字符，则改成空列表，避免继续向 Codex 上游提交不合法结构。

## 影响范围

影响范围只限 `provider_type=codex` 且接口格式属于 OpenAI Responses 家族的上游请求。用户原始请求记录仍保留原始内容，上游请求记录保存转换后的内容，便于排查。

## 验证方式

- 单元测试覆盖非空字符串输入会转成用户消息列表。
- 单元测试覆盖空白字符串输入会转成空列表。
- 运行 Codex Responses 相关测试，确认现有工具、图片和默认字段处理不受影响。
