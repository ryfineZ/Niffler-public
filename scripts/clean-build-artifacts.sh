#!/bin/bash
# Safely clean local build artifacts. Dry-run by default.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd -P)"
EXECUTE=false
CLEAN_DOCKER=true

usage() {
    cat <<'EOF'
Usage: scripts/clean-build-artifacts.sh [options]

Options:
  --execute      Actually delete whitelisted build artifacts. Default is dry-run.
  --no-docker    Skip Docker dangling image and build cache cleanup.
  -h, --help     Show help.

The script only cleans whitelisted paths under the repository:
  target
  frontend/dist
  tmp/ci-artifacts
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --execute)
            EXECUTE=true
            shift
            ;;
        --no-docker)
            CLEAN_DOCKER=false
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

say_mode() {
    if [ "$EXECUTE" = true ]; then
        echo "Mode: execute"
    else
        echo "Mode: dry-run"
    fi
}

safe_path() {
    local relative_path="$1"
    local absolute_path
    absolute_path="$ROOT_DIR/$relative_path"

    case "$relative_path" in
        target|frontend/dist|tmp/ci-artifacts)
            ;;
        *)
            echo "Refusing non-whitelisted path: $relative_path" >&2
            exit 1
            ;;
    esac

    case "$absolute_path" in
        "$ROOT_DIR"/*)
            printf '%s\n' "$absolute_path"
            ;;
        *)
            echo "Refusing path outside repository: $absolute_path" >&2
            exit 1
            ;;
    esac
}

clean_path() {
    local relative_path="$1"
    local absolute_path
    absolute_path="$(safe_path "$relative_path")"

    if [ ! -e "$absolute_path" ]; then
        echo "skip missing: $relative_path"
        return
    fi

    if [ "$EXECUTE" = true ]; then
        echo "delete: $relative_path"
        rm -rf -- "$absolute_path"
    else
        echo "would delete: $relative_path"
        du -sh "$absolute_path" 2>/dev/null || true
    fi
}

clean_docker() {
    if [ "$CLEAN_DOCKER" != true ]; then
        echo "skip docker cleanup"
        return
    fi
    if ! command -v docker >/dev/null 2>&1; then
        echo "skip docker cleanup: docker not found"
        return
    fi

    if [ "$EXECUTE" = true ]; then
        echo "delete docker dangling images and build cache"
        docker image prune -f
        docker builder prune -f
    else
        echo "would prune docker dangling images and build cache"
        docker system df || true
    fi
}

say_mode
clean_path target
clean_path frontend/dist
clean_path tmp/ci-artifacts
clean_docker
