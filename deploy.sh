#!/bin/bash
# Niffler 发布脚本
#
# 服务器发布只拉取 CI 已构建好的镜像，不在服务器上编译 Rust、构建前端或 docker build。
#
# 用法:
#   发布/更新:     ./deploy.sh
#   强制重建容器:  ./deploy.sh --force

set -euo pipefail
cd "$(dirname "$0")"

if command -v docker-compose >/dev/null 2>&1; then
    DC=(docker-compose -f docker-compose.yml)
else
    DC=(docker compose -f docker-compose.yml)
fi

usage() {
    cat <<'EOF'
Usage: ./deploy.sh [options]

Options:
  --force, -f             强制重建 app 容器
  --no-pull               跳过拉取镜像，仅用本机已有镜像重启
  -h, --help              显示帮助

Environment:
  APP_IMAGE               CI 构建好的应用镜像，例如 ghcr.io/ryfinez/niffler:main
EOF
}

FORCE_RECREATE=false
SKIP_PULL=false

while [ $# -gt 0 ]; do
    case "$1" in
        --force|-f)
            FORCE_RECREATE=true
            shift
            ;;
        --no-pull)
            SKIP_PULL=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1"
            usage
            exit 1
            ;;
    esac
done

if [ ! -f docker-compose.yml ]; then
    echo "Required file not found: docker-compose.yml"
    exit 1
fi

echo ">>> Deploy mode: pull CI image and restart app; no server-side build."
if [ -n "${APP_IMAGE:-}" ]; then
    echo ">>> APP_IMAGE=${APP_IMAGE}"
else
    echo ">>> APP_IMAGE is not exported; Docker Compose will read .env or use docker-compose.yml default."
fi

if [ "$SKIP_PULL" = false ]; then
    echo ">>> Pulling app image..."
    "${DC[@]}" pull app
else
    echo ">>> Skipping image pull."
fi

echo ">>> Starting app..."
if [ "$FORCE_RECREATE" = true ]; then
    "${DC[@]}" up -d --no-build --force-recreate app
else
    "${DC[@]}" up -d --no-build app
fi

docker image prune -f >/dev/null 2>&1 || true

echo ">>> Done!"
echo ">>> Note: app image must be built by GitHub Actions before deployment."
"${DC[@]}" ps
