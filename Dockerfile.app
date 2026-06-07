# syntax=docker/dockerfile:1
# Aether Gateway 运行时镜像（交叉编译方案）
# 二进制和前端产物均由 CI 预先构建，此 Dockerfile 仅做打包
# 用法: docker build --build-arg TARGETARCH=amd64 -f Dockerfile.app .
# 正式多架构发版时也可以传入 linux/amd64,linux/arm64。
#
# 构建上下文中须包含:
#   dist/aether-gateway-${TARGETARCH}
#   dist/frontend/              (npm run build 产物)

FROM gcr.io/distroless/static-debian12

# TARGETARCH 由 buildx 自动注入: amd64 或 arm64
ARG TARGETARCH

COPY dist/aether-gateway-${TARGETARCH} /usr/local/bin/aether-gateway
COPY dist/frontend/ /srv/frontend

WORKDIR /app

ENV RUST_LOG=aether_gateway=info \
    APP_PORT=8084 \
    AETHER_GATEWAY_STATIC_DIR=/srv/frontend

EXPOSE 8084

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/aether-gateway", "--healthcheck"]

USER root
ENTRYPOINT ["/usr/local/bin/aether-gateway"]
