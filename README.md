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
cd apps/console && cp .env.example .env && pnpm install && pnpm dev
```

Ouvre http://localhost:4080, clique sur « Sign up » pour créer ton compte, puis connecte-toi.
FerrisKey n'a pas d'API admin pour définir un mot de passe : l'auto-inscription est le seul
moyen d'entrer sur la stack locale. Après l'inscription, il faut se connecter une fois.

| Service | URL |
|---|---|
| Console | http://localhost:4080 |
| API (+ `/docs`, `/api-docs/openapi.json`) | http://localhost:4100 |
| MCP Kanban | http://localhost:4101 |
| FerrisKey API / console | http://localhost:4110 / http://localhost:4111 |
| Iggy TCP / HTTP | localhost:4190 / http://localhost:4191 |
| Postgres | localhost:5442 |
| Jaeger (UI / OTLP gRPC) | http://localhost:16686 / localhost:4327 |

## Vérifier

```bash
cargo fmt --all --check
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                          # les tests DB sont sautés sans DATABASE_URL
REQUIRE_DATABASE_URL=1 cargo test --workspace   # échoue si la base manque, comme en CI
```

Toute nouvelle requête `query!` demande une base vivante puis
`cargo sqlx prepare --workspace -- --all-targets`.
