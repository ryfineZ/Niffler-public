# sub2api 同步权限默认值

## 目标

修正 `scripts/oneoff/sync_sub2api_to_niffler.py` 的导入默认值，避免同步导入的用户和 API Key 被隐藏限制到 Codex provider。

## 非目标

不改变现有分组、模型、供应商和号池账号的业务配置。

## 行为变化

同步导入的用户默认不设置用户级供应商、接口格式、模型限制。同步导入的 API Key 默认也不设置 Key 级供应商、接口格式、模型限制，并且不再写入 API Key 到 Codex provider 的直接映射。可用范围由用户所属分组和控制台显式配置决定。

## 影响范围

只影响以后重新运行 `sync_sub2api_to_niffler.py` 生成的 SQL。已经写入线上库的历史数据需要用一次性数据修复清理。

## 验证方式

运行 `python3 -m py_compile scripts/oneoff/sync_sub2api_to_niffler.py` 验证脚本语法。线上数据修复后，检查 `metadata->>'source' = 'sub2api'` 的用户和 API Key 不再残留用户级或 Key 级供应商、接口格式、模型限制。
