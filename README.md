# Capitainerie

Plateforme open source d'orchestration d'agents IA, écrite en Rust et pensée pour Kubernetes.

Premier cas d'usage : on dépose le transcript d'une réunion, une chaîne d'agents en extrait les actions et les propose comme cartes dans un Kanban. Un humain valide chaque carte avant qu'elle entre au tableau.

Trois piliers :

- **Orchestration durable** : chaque run est une machine à états en Postgres. Ses étapes sont validées une à une et reprises après un crash.
- **Identité** : les humains et les agents passent par [FerrisKey](https://github.com/ferriskey/ferriskey). Un agent agit avec un jeton délégué (RFC 8693, claim `act`), et chaque écriture est décidée par une politique d'autorisation puis auditée.
- **Observabilité** : une trace OpenTelemetry par run, de l'API au serveur MCP (conventions GenAI). La console affiche une timeline par run, les tokens et les coûts par agent, utilisateur, modèle et jour.

## Comment ça marche

```
console ──POST transcript──▶ api ──outbox──▶ Iggy ──▶ worker
                                                        │ 1. Extraction    (LLM : actions + citations exactes)
                                                        │ 2. Dédoublonnage (LanceDB + juge LLM en zone grise)
                                                        │ 3. Rédaction     (agent, jeton délégué)
                                                        ▼
                                              mcp-kanban ──▶ cartes « Proposé » ──▶ validation humaine
```

- Une carte cite toujours le passage exact du transcript dont elle vient. La citation est vérifiée caractère par caractère : une citation inventée est refusée.
- Le serveur MCP Kanban est le seul outil des agents. Un agent ne peut jamais sortir une carte de « Proposé » : seul un humain le peut.
- Une action déjà présente au tableau ne crée pas de nouvelle carte : la carte existante reçoit un commentaire.

Le détail est dans [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Statut

MVP fonctionnel de bout en bout sur une stack locale, avec un modèle de 7 milliards de paramètres (`qwen2.5:7b-instruct`) sur Ollama. Limites connues :

- La qualité de l'extraction dépend fortement du modèle : sur le jeu d'évaluation (`libs/runtime-rig/eval`), le rappel est de 82 %.
- L'autorisation passe par un moteur de politique local. Il sera remplacé par le PDP AuthZEN de FerrisKey quand celui-ci exposera son endpoint ([`docs/ferriskey-needs.md`](docs/ferriskey-needs.md)).
- Une délégation reste bornée par la session FerrisKey de l'utilisateur, en attendant un grant de délégation durable côté FerrisKey.

## Démarrer en local

Prérequis : Docker, Rust 1.97.1 (`rust-toolchain.toml`), `protoc`, `sqlx-cli` 0.8.6, Node 22+ avec corepack (pnpm 9.15), Terraform et `jq`.

```bash
cp .env.example .env
docker compose up -d
docker compose exec ollama ollama pull qwen2.5:7b-instruct   # ~4,7 Go
docker compose exec ollama ollama pull bge-m3                # ~1,2 Go, embeddings
sqlx migrate run --source migrations
./scripts/bootstrap-ferriskey.sh   # realm, clients, policies d'échange, organisation de démo
```

Le script de bootstrap affiche `EXCHANGE_CLIENT_SECRET` et `AGENT_CREDENTIALS` sans les écrire. Reportez-les dans `.env`, avec une `DELEGATION_KEY` (`openssl rand -base64 32`, la même pour l'API et le worker). Ensuite, lancez chaque service dans son propre terminal :

```bash
cargo run -p capitainerie-api
cargo run -p capitainerie-mcp-kanban
cargo run -p capitainerie-worker     # relais outbox, orchestrateur, indexeur des cartes
cargo run -p capitainerie-exporter   # événements de run → Parquet (page Agents)
cd apps/console && cp .env.example .env && corepack pnpm install && corepack pnpm dev
```

Ouvrez http://localhost:4080, cliquez sur « Sign up » pour créer un compte, puis connectez-vous. Sur la stack locale, l'auto-inscription est le seul moyen d'obtenir un compte : FerrisKey n'a pas d'API admin pour définir un mot de passe.

Sous Docker Desktop, Ollama tourne sur CPU : comptez une à deux minutes par étape. Un Ollama natif (GPU) va beaucoup plus vite : pointez `LLM_BASE_URL` dessus.

| Service | URL |
|---|---|
| Console | http://localhost:4080 |
| API (+ `/docs`, `/api-docs/openapi.json`) | http://localhost:4100 |
| MCP Kanban (`/mcp`, `/.well-known/oauth-protected-resource`) | http://localhost:4101 |
| FerrisKey API / console | http://localhost:4110 / http://localhost:4111 |
| Iggy TCP / HTTP | localhost:4190 / http://localhost:4191 |
| Postgres | localhost:5442 |
| Ollama (OpenAI-compatible `/v1`) | http://localhost:4120 |
| Jaeger (UI / OTLP gRPC) | http://localhost:16686 / localhost:4327 |

### Exécutant (optionnel)

L'Exécutant lance Claude Code dans un Job Kubernetes. Prérequis : [kind](https://kind.sigs.k8s.io) et `kubectl`. `./scripts/kind-up.sh` crée le cluster `capitainerie`, le namespace `capitainerie-tasks`, construit l'image `capitainerie-executor:dev` et la charge dans kind, puis affiche le contexte kubeconfig à utiliser (`kind-capitainerie`).

Limite connue : le CNI par défaut de kind n'applique pas les `NetworkPolicy`. L'isolation réseau des conteneurs n'est donc pas vérifiée en local ; elle l'est seulement sur un cluster dont le CNI les applique (Cilium, Calico).

## Vérifier

```bash
cargo fmt --all --check
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace     # tests Postgres et Iggy sautés sans DATABASE_URL / IGGY_ADDRESS

# Comme en CI : une base de test dédiée, jamais la base de dev (un worker qui
# tourne publierait l'outbox pendant les tests).
export DATABASE_URL=postgres://capitainerie:capitainerie@localhost:5442/capitainerie_test
sqlx migrate run --source migrations
IGGY_ADDRESS=localhost:4190 REQUIRE_DATABASE_URL=1 REQUIRE_IGGY=1 \
  cargo test --workspace -- --test-threads=1

cd apps/console && corepack pnpm check && corepack pnpm test && corepack pnpm build
```

Une nouvelle requête `query!` demande une base vivante, puis `cargo sqlx prepare --workspace -- --all-targets`.

Le jeu d'évaluation de l'extraction s'exécute contre le vrai modèle : `cargo run -p capitainerie-runtime-rig --example eval`.

## Déployer

Une image par service (`Dockerfile` à cibles multiples : `api`, `worker`, `mcp-kanban`, `exporter`, `migrations`, `console`) et un chart Helm dans [`deploy/helm/capitainerie`](deploy/helm/capitainerie). Postgres, Iggy, FerrisKey et l'endpoint LLM sont des dépendances externes.

## Documentation

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) : domaine, ports et adapters, flux, identité, décisions.
- [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) : conventions de code Rust et front.
- [`docs/ferriskey-needs.md`](docs/ferriskey-needs.md) : ce que Capitainerie attend de FerrisKey.

## Licence

[Apache-2.0](LICENSE).
