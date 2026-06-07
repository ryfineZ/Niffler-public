#!/bin/bash
# Download the app image built by GitHub Actions and load it on a server.
# The server does not build Rust, frontend assets, or Docker images.

set -euo pipefail

WORKFLOW_NAME="${WORKFLOW_NAME:-Build App Image}"
GH_REPO="${GH_REPO:-ryfineZ/Niffler}"
BRANCH="${BRANCH:-main}"
ARTIFACT_NAME="${ARTIFACT_NAME:-niffler-app-linux-amd64}"
REMOTE_TAR="${REMOTE_TAR:-/tmp/niffler-app-linux-amd64.tar}"
APP_IMAGE="${APP_IMAGE:-niffler-app:latest}"
APP_SERVICES="${APP_SERVICES:-app}"
SSH_OPTS="${SSH_OPTS:-}"

DEPLOY_HOST=""
REMOTE_DIR="/opt/niffler-app"
RUN_ID=""
COMMIT_REF=""
ALLOW_LATEST_FOR_LOCAL=false

usage() {
    cat <<'EOF'
Usage: scripts/deploy-ci-artifact.sh --host <ssh-host> [options]

Options:
  --host <ssh-host>        SSH host, for example hd0526
  --remote-dir <path>      Remote compose directory, default /opt/niffler-app
  --run-id <id>            GitHub Actions run id for the artifact to deploy
  --commit <sha>           Git commit SHA; script resolves the successful workflow run for it
  --allow-latest-for-local Allow latest successful run selection. Only for local verification or temporary diagnostics.
  -h, --help               Show help

Environment:
  APP_IMAGE                Image tag used by docker compose, default niffler-app:latest
  APP_SERVICES             Compose services to restart, default app
  GH_REPO                  GitHub repo used by gh, default ryfineZ/Niffler
  ARTIFACT_NAME            CI artifact name, default niffler-app-linux-amd64
  WORKFLOW_NAME            GitHub Actions workflow name, default Build App Image
  BRANCH                   Branch used when selecting latest successful run, default main
  SSH_OPTS                 Extra ssh/scp options
EOF
}

require_option_value() {
    local option_name="$1"
    local option_value="${2:-}"
    if [ -z "$option_value" ] || [[ "$option_value" == --* ]]; then
        echo "Missing value for $option_name"
        usage
        exit 1
    fi
}

while [ $# -gt 0 ]; do
    case "$1" in
        --host)
            require_option_value "$1" "${2:-}"
            DEPLOY_HOST="${2:-}"
            shift 2
            ;;
        --remote-dir)
            require_option_value "$1" "${2:-}"
            REMOTE_DIR="${2:-}"
            shift 2
            ;;
        --run-id)
            require_option_value "$1" "${2:-}"
            RUN_ID="${2:-}"
            shift 2
            ;;
        --commit)
            require_option_value "$1" "${2:-}"
            COMMIT_REF="${2:-}"
            shift 2
            ;;
        --allow-latest-for-local)
            ALLOW_LATEST_FOR_LOCAL=true
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

if [ -z "$DEPLOY_HOST" ]; then
    echo "Missing required option: --host"
    usage
    exit 1
fi

for command_name in gh ssh scp; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "Required command not found: $command_name"
        exit 1
    fi
done

if [ -n "$RUN_ID" ] && [ -n "$COMMIT_REF" ]; then
    echo "Use only one of --run-id or --commit"
    exit 1
fi

if [ -n "$COMMIT_REF" ]; then
    RUN_ID="$(gh run list \
        --repo "$GH_REPO" \
        --workflow "$WORKFLOW_NAME" \
        --commit "$COMMIT_REF" \
        --status success \
        --limit 1 \
        --json databaseId \
        --jq '.[0].databaseId // ""')"
    if [ -z "$RUN_ID" ] || [ "$RUN_ID" = "null" ]; then
        echo "No successful $WORKFLOW_NAME workflow run found for commit $COMMIT_REF"
        echo "Confirm the CI image workflow has completed successfully, or deploy with --run-id."
        exit 1
    fi
fi

if [ -z "$RUN_ID" ]; then
    if [ "$ALLOW_LATEST_FOR_LOCAL" != true ]; then
        echo "Production deployment requires --run-id or --commit."
        echo "Use --allow-latest-for-local only for local verification or temporary diagnostics."
        exit 1
    fi
    RUN_ID="$(gh run list \
        --repo "$GH_REPO" \
        --workflow "$WORKFLOW_NAME" \
        --branch "$BRANCH" \
        --status success \
        --limit 1 \
        --json databaseId \
        --jq '.[0].databaseId')"
fi

if [ -z "$RUN_ID" ] || [ "$RUN_ID" = "null" ]; then
    echo "No successful workflow run found for $WORKFLOW_NAME on $BRANCH"
    exit 1
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo ">>> Downloading CI image artifact from run $RUN_ID..."
gh run download "$RUN_ID" --repo "$GH_REPO" --name "$ARTIFACT_NAME" --dir "$TMP_DIR"

IMAGE_TAR="$TMP_DIR/niffler-app-linux-amd64.tar"
if [ ! -f "$IMAGE_TAR" ]; then
    echo "Artifact did not contain expected file: niffler-app-linux-amd64.tar"
    find "$TMP_DIR" -maxdepth 2 -type f -print
    exit 1
fi

echo ">>> Uploading image tar to $DEPLOY_HOST:$REMOTE_TAR..."
scp $SSH_OPTS "$IMAGE_TAR" "$DEPLOY_HOST:$REMOTE_TAR"

echo ">>> Loading image and restarting services on $DEPLOY_HOST..."
read -r -a LOCAL_SERVICES <<< "$APP_SERVICES"
ssh $SSH_OPTS "$DEPLOY_HOST" bash -s -- "$REMOTE_DIR" "$REMOTE_TAR" "$APP_IMAGE" "${LOCAL_SERVICES[@]}" <<'REMOTE_SCRIPT'
set -euo pipefail

REMOTE_DIR="$1"
REMOTE_TAR="$2"
APP_IMAGE="$3"
shift 3
SERVICES=("$@")

cd "$REMOTE_DIR"

if [ ! -f docker-compose.yml ]; then
    echo "Required file not found on server: $REMOTE_DIR/docker-compose.yml"
    exit 1
fi

docker load -i "$REMOTE_TAR"

if [ -f .env ]; then
    if grep -q '^APP_IMAGE=' .env; then
        sed -i.bak "s|^APP_IMAGE=.*|APP_IMAGE=$APP_IMAGE|" .env
    else
        printf '\nAPP_IMAGE=%s\n' "$APP_IMAGE" >> .env
    fi
else
    printf 'APP_IMAGE=%s\n' "$APP_IMAGE" > .env
fi

if docker compose version >/dev/null 2>&1; then
    DC=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
    DC=(docker-compose)
else
    echo "docker compose is not installed on server"
    exit 1
fi

"${DC[@]}" up -d --no-build --force-recreate "${SERVICES[@]}"
"${DC[@]}" ps
rm -f "$REMOTE_TAR"
REMOTE_SCRIPT

echo ">>> Done."
