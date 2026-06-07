# CI 构建分层策略

## 目标

降低日常提交和上线构建耗时，避免每次改代码都构建当前线上不需要的平台产物。

## 非目标

- 不降低正式发版时的校验能力。
- 不删除多平台发版能力。
- 不改变线上服务运行方式。
- 不改变 Postgres/Redis 部署架构。

## 行为变化

- 日常 Rust CI 保留格式检查、Clippy、测试和 Postgres 数据库冒烟测试。
- SQLite 和 MySQL 数据库冒烟测试改为手动全量 CI 时运行，避免日常提交反复启动不在线上使用的数据库服务。
- 应用镜像构建不再跟随 `main` 分支每次 push 自动运行，改为手动触发。
- 应用镜像构建只生成当前线上需要的 `linux/amd64` 版本，不再生成 `linux/arm64`。
- Docker 镜像只构建一次 `linux/amd64` 本地镜像，并导出为 `niffler-app-linux-amd64.tar`；不再重复构建并推送 GHCR。
- 正式 release 和 tunnel 多平台构建仍保留在标签或手动场景，不绑定日常上线。

## 影响范围

- GitHub Actions 的 Rust CI。
- GitHub Actions 的应用镜像构建。
- 当前应用镜像发布不再产出 `ghcr.io/ryfinez/niffler:main`，线上通过 CI 产出的 tar 镜像文件发布。
- 不影响本地构建命令。
- 不影响正式 tag release 的多平台产物。

## 验证方式

- 使用 GitHub Actions 语法检查或实际触发一次手动应用镜像构建。
- 日常 push 后确认不会自动触发应用镜像构建。
- 手动触发 Rust CI 且开启全量选项时，确认 SQLite/MySQL 冒烟测试会运行。
