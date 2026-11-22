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

FROM base AS executor
RUN npm install -g @anthropic-ai/claude-code@2.1.302 \
    && npm cache clean --force
USER 1000:1000
