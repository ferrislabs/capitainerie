FROM node:22-slim AS base

RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN userdel -r node \
    && useradd --uid 1000 --user-group --create-home --home-dir /home/executor --shell /bin/bash executor \
    && mkdir /workspace \
    && chown executor:executor /workspace

COPY --chmod=0755 entrypoint.sh /usr/local/bin/entrypoint.sh

ENV HOME=/home/executor
WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

FROM base AS codex
RUN npm install -g @openai/codex@0.167.0 \
    && npm cache clean --force
USER 1000:1000

FROM base AS antigravity
ARG TARGETARCH
ARG AGY_VERSION=1.5.3
ARG AGY_SHA256_AMD64=1b4850599cce73a17f94ffb422df3bf9531531b77dbddc40c1a435e0c2c16807
ARG AGY_SHA256_ARM64=2d50060c7b1327995e5c81ec58829c91c2564ebb9fec165d3f9f1325ec188675
RUN set -eu; \
    case "$TARGETARCH" in \
      amd64) asset=agy_cli_linux_x64.tar.gz; sha="$AGY_SHA256_AMD64" ;; \
      arm64) asset=agy_cli_linux_arm64.tar.gz; sha="$AGY_SHA256_ARM64" ;; \
      *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    apt-get update; \
    apt-get install -y --no-install-recommends curl; \
    curl -fsSL -o "/tmp/$asset" "https://github.com/google-antigravity/antigravity-cli/releases/download/${AGY_VERSION}/${asset}"; \
    echo "$sha  /tmp/$asset" | sha256sum -c -; \
    tar -xzf "/tmp/$asset" -C /usr/local/bin antigravity; \
    mv /usr/local/bin/antigravity /usr/local/bin/agy; \
    chmod 755 /usr/local/bin/agy; \
    rm -f "/tmp/$asset"; \
    apt-get purge -y --auto-remove curl; \
    rm -rf /var/lib/apt/lists/*
USER 1000:1000

FROM python:3.12-slim-bookworm AS mistral-vibe-build
ENV UV_TOOL_DIR=/opt/uv-tools UV_TOOL_BIN_DIR=/usr/local/bin UV_PYTHON_DOWNLOADS=never
RUN pip install --no-cache-dir uv==0.9.5 \
    && uv tool install --python /usr/local/bin/python3.12 mistral-vibe==3.1.4 \
    && chmod -R a+rX /opt/uv-tools

FROM python:3.12-slim-bookworm AS mistral-vibe
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --uid 1000 --user-group --create-home --home-dir /home/executor --shell /bin/bash executor \
    && mkdir /workspace \
    && chown executor:executor /workspace
COPY --from=mistral-vibe-build /opt/uv-tools /opt/uv-tools
COPY --from=mistral-vibe-build /usr/local/bin/vibe /usr/local/bin/vibe-app-server /usr/local/bin/
COPY --chmod=0755 entrypoint.sh /usr/local/bin/entrypoint.sh
ENV HOME=/home/executor
WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
USER 1000:1000

FROM base AS executor
RUN npm install -g @anthropic-ai/claude-code@2.1.302 \
    && npm cache clean --force
USER 1000:1000
