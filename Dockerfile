# syntax=docker/dockerfile:1.7

FROM lukemathwalker/cargo-chef:0.1.77-rust-1.97.1-bookworm AS chef
WORKDIR /usr/local/src/capitainerie
ENV SQLX_OFFLINE=true
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS cook
COPY --from=planner /usr/local/src/capitainerie/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY . .

FROM cook AS build-api
RUN cargo build --release --locked -p capitainerie-api

FROM cook AS build-worker
RUN cargo build --release --locked -p capitainerie-worker

FROM cook AS build-exporter
RUN cargo build --release --locked -p capitainerie-exporter

FROM cook AS build-mcp-kanban
RUN cargo build --release --locked -p capitainerie-mcp-kanban

FROM cook AS build-egress-proxy
RUN cargo build --release --locked -p capitainerie-egress-proxy

FROM rust:1.97.1-bookworm AS build-sqlx
RUN cargo install sqlx-cli --version 0.8.6 --no-default-features --features rustls,postgres --locked

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f AS runtime
USER 65532:65532
WORKDIR /

FROM runtime AS api
COPY --from=build-api /usr/local/src/capitainerie/target/release/capitainerie-api /usr/local/bin/capitainerie-api
EXPOSE 4100
ENTRYPOINT ["/usr/local/bin/capitainerie-api"]

FROM runtime AS worker
COPY --from=build-worker /usr/local/src/capitainerie/target/release/capitainerie-worker /usr/local/bin/capitainerie-worker
ENTRYPOINT ["/usr/local/bin/capitainerie-worker"]

FROM runtime AS exporter
COPY --from=build-exporter /usr/local/src/capitainerie/target/release/capitainerie-exporter /usr/local/bin/capitainerie-exporter
ENTRYPOINT ["/usr/local/bin/capitainerie-exporter"]

FROM runtime AS mcp-kanban
COPY --from=build-mcp-kanban /usr/local/src/capitainerie/target/release/capitainerie-mcp-kanban /usr/local/bin/capitainerie-mcp-kanban
EXPOSE 4101
ENTRYPOINT ["/usr/local/bin/capitainerie-mcp-kanban"]

FROM runtime AS egress-proxy
COPY --from=build-egress-proxy /usr/local/src/capitainerie/target/release/capitainerie-egress-proxy /usr/local/bin/capitainerie-egress-proxy
EXPOSE 4130
ENTRYPOINT ["/usr/local/bin/capitainerie-egress-proxy"]

FROM runtime AS migrations
COPY --from=build-sqlx /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY migrations /migrations
ENTRYPOINT ["/usr/local/bin/sqlx", "migrate", "run", "--source", "/migrations"]

FROM node:24-alpine AS build-console
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate
COPY apps/console/package.json apps/console/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY apps/console ./
RUN pnpm build

FROM nginxinc/nginx-unprivileged:1.28-alpine@sha256:7377697a821c131a924a7105fafbe7414db4e9fcc77a6f08f776f33f141ec3f8 AS console
COPY --chmod=0644 apps/console/nginx.conf /tmp/nginx.conf
USER root
RUN sed 's/listen 80;/listen 8080;/' /tmp/nginx.conf > /etc/nginx/conf.d/default.conf \
    && rm /tmp/nginx.conf \
    && chown -R 101:101 /usr/share/nginx/html
USER 101
COPY --from=build-console --chown=101:101 /app/dist/client /usr/local/src/capitainerie
COPY --chmod=0755 apps/console/docker-entrypoint.sh /docker-entrypoint-app.sh
EXPOSE 8080
ENTRYPOINT ["/docker-entrypoint-app.sh"]
CMD ["nginx", "-g", "daemon off;"]
