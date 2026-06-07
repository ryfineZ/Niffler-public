# CI 镜像发布流程

## 目标

生产发布不再在服务器上编译 Rust、构建前端或执行 `docker build`。应用镜像由 GitHub Actions 构建，发布时服务器只负责加载镜像并重启容器。

## 非目标

- 不改变 Postgres、Redis 的部署方式。
- 不把 GitHub、GHCR 或服务器密钥写入仓库。
- 不要求服务器登录 GHCR。
- 不要求服务器安装 Rust、Node.js 或前端依赖。

## 行为变化

- 主应用镜像构建不再跟随 `main` 推送自动执行，需要在 GitHub Actions 手动触发 `Build App Image`。
- 当前线上只使用 Linux amd64，因此 `Build App Image` 只构建 amd64 的 `aether-gateway`。
- `Build App Image` 只产出 `niffler-app-linux-amd64` 镜像文件，不再推送 GHCR 镜像，避免重复构建和上传。
- `deploy.sh` 不再使用 `Dockerfile.app.local`，也不再计算代码哈希。
- `deploy.sh` 只执行镜像拉取和 `docker compose up -d --no-build`。
- `scripts/deploy-ci-artifact.sh` 会从 CI 下载镜像文件，上传到服务器，执行 `docker load`，再重启指定服务。
- 生产执行 `scripts/deploy-ci-artifact.sh` 必须显式传入 `--run-id` 或 `--commit`，不能默认部署“最新成功产物”。
- 使用 `--commit` 时，脚本会按提交号查找对应的成功 `Build App Image` 工作流；如果没有找到成功产物，脚本必须停止，不能退回到默认分支的最新产物。
- `--allow-latest-for-local` 只允许本地验证或临时排查使用，不能作为生产发布命令。

## 影响范围

- GitHub Actions 主应用镜像构建流程只产出 amd64 镜像文件。
- 线上发布使用 CI 产出的镜像文件，不依赖服务器访问私有 GHCR。
- 服务器 `.env` 中的 `APP_IMAGE` 应设置为 `niffler-app:latest`，由 `docker load` 后的本地镜像提供。

## 发布方式

使用 CI 镜像文件发布。以 hd0526 为例：

```bash
APP_SERVICES="frontdoor background" \
APP_IMAGE=niffler-app:latest \
GH_REPO=ryfineZ/Niffler \
./scripts/deploy-ci-artifact.sh \
  --host hd0526 \
  --remote-dir /opt/niffler-app \
  --commit <git-commit-sha>
```

这个脚本会下载指定提交对应的 `Build App Image` 工作流产物，把镜像文件传到服务器，服务器加载成 `niffler-app:latest`，再重启 `frontdoor` 和 `background`。Postgres 和 Redis 不需要重启。

如果指定提交没有成功的 `Build App Image` 工作流，脚本会直接报错并停止。需要先触发并等待该提交的 CI 镜像构建成功，或者改用明确的 `--run-id`。

如果已经知道 GitHub Actions run id，也可以使用：

```bash
APP_SERVICES="frontdoor background" \
APP_IMAGE=niffler-app:latest \
GH_REPO=ryfineZ/Niffler \
./scripts/deploy-ci-artifact.sh \
  --host hd0526 \
  --remote-dir /opt/niffler-app \
  --run-id <github-actions-run-id>
```

本地验证或临时排查时，才可以显式选择最新成功产物：

```bash
./scripts/deploy-ci-artifact.sh \
  --host <test-host> \
  --allow-latest-for-local
```

## 验证方式

- `bash -n deploy.sh`
- `bash -n scripts/deploy-ci-artifact.sh`
- 不传 `--run-id`、`--commit` 和 `--allow-latest-for-local` 时，`scripts/deploy-ci-artifact.sh` 必须拒绝执行。
- GitHub Actions 的 `Build App Image` 工作流成功。
- 服务器执行发布脚本后，`docker compose ps` 显示应用容器健康。
