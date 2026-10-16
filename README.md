# Capitainerie

Plateforme open source d'orchestration d'agents IA, en Rust, pensée pour Kubernetes.

- Conventions : [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md)
- Architecture : [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Besoins côté FerrisKey : [`docs/ferriskey-needs.md`](docs/ferriskey-needs.md)

## Démarrer en local

```bash
cp .env.example .env
docker compose up -d
sqlx migrate run --source migrations
./scripts/bootstrap-ferriskey.sh          # realm, client console, organisation de démo
cargo run -p capitainerie-api
cargo run -p capitainerie-worker          # relaie l'outbox vers Iggy
cd apps/console && cp .env.example .env && pnpm install && pnpm dev
```

Ouvre http://localhost:4080, clique sur « Sign up » pour créer ton compte, puis connecte-toi.
FerrisKey n'a pas d'API admin pour définir un mot de passe : l'auto-inscription est le seul
moyen d'entrer sur la stack locale. Après l'inscription, il faut se connecter une fois.

| Service | URL |
|---|---|
| Console | http://localhost:4080 |
| API (+ `/docs`, `/api-docs/openapi.json`) | http://localhost:4100 |
| MCP Kanban (`/mcp`, `/.well-known/oauth-protected-resource`) | http://localhost:4101 |
| FerrisKey API / console | http://localhost:4110 / http://localhost:4111 |
| Iggy TCP / HTTP | localhost:4190 / http://localhost:4191 |
| Postgres | localhost:5442 |
| Jaeger (UI / OTLP gRPC) | http://localhost:16686 / localhost:4327 |

## Vérifier

```bash
cargo fmt --all --check
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace     # tests DB et Iggy sautés sans DATABASE_URL / IGGY_ADDRESS

# comme en CI : base de test dédiée, jamais la base de dev (un worker qui tourne
# publierait l'outbox sous le nez des tests)
DATABASE_URL=postgres://capitainerie:capitainerie@localhost:5442/capitainerie_test \
  sqlx migrate run --source migrations
DATABASE_URL=postgres://capitainerie:capitainerie@localhost:5442/capitainerie_test \
  IGGY_ADDRESS=localhost:4190 REQUIRE_DATABASE_URL=1 REQUIRE_IGGY=1 \
  cargo test --workspace -- --test-threads=1
```

Toute nouvelle requête `query!` demande une base vivante puis
`cargo sqlx prepare --workspace -- --all-targets`.
