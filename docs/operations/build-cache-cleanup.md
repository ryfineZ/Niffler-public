# 构建缓存清理

## 目标

本地构建目录、Docker 悬空缓存和临时 CI 镜像包不能长期堆积到占满磁盘。清理必须可预览、路径受限、不会碰源码、数据库和配置。

## 非目标

- 不清理生产数据库、Redis 数据、`.env`、源码和 Git 历史。
- 不替代系统级磁盘监控。
- 不自动定时执行清理。

## 行为变化

- 新增 `scripts/clean-build-artifacts.sh`。
- 脚本默认 dry-run，只输出将要清理的路径和 Docker 缓存摘要。
- 只有显式传入 `--execute` 才会删除文件或执行 Docker 清理。
- 文件清理只允许仓库内白名单路径：`target`、`frontend/dist`、`tmp/ci-artifacts`。
- Docker 清理只执行悬空镜像和构建缓存清理，不删除运行中的容器、卷、数据库目录或配置文件。

## 影响范围

- 影响本地开发机和临时构建机。
- 不影响生产发布流程；生产发布仍使用 CI 镜像产物，不在服务器编译。

## 使用方式

预览：

```bash
./scripts/clean-build-artifacts.sh
```

执行：

```bash
./scripts/clean-build-artifacts.sh --execute
```

只清理文件，不碰 Docker：

```bash
./scripts/clean-build-artifacts.sh --execute --no-docker
```

## 验证方式

- `bash -n scripts/clean-build-artifacts.sh`
- 默认运行脚本只输出预览，不删除文件。
- 使用 `--execute --no-docker` 时，只会删除白名单路径里的构建产物。
