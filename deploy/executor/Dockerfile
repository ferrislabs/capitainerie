FROM node:22-slim AS executor

RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g @anthropic-ai/claude-code@2.1.302 \
    && npm cache clean --force

RUN userdel -r node \
    && useradd --uid 1000 --user-group --create-home --home-dir /home/executor --shell /bin/bash executor \
    && mkdir /workspace \
    && chown executor:executor /workspace

COPY --chmod=0755 entrypoint.sh /usr/local/bin/entrypoint.sh

USER 1000:1000
ENV HOME=/home/executor
WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
