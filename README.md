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

**GitHub App** : chaque organisation crée sa propre GitHub App depuis la console, sans variable d'environnement côté opérateur ni étape manuelle sur GitHub.

1. Une clé `SECRETS_KEY` (elle scelle aussi les secrets de l'App) et `CONSOLE_URL=http://localhost:4080` sont définies pour l'API.
2. Un admin ouvre Paramètres › Intégrations › GitHub, choisit « Compte personnel » ou « Organisation GitHub » (avec le login de l'organisation GitHub), puis clique sur « Créer l'application GitHub ». La console envoie à GitHub, par un formulaire POST du navigateur, le manifeste que l'API a préparé.
3. Après la validation sur GitHub, le navigateur revient sur la console, qui échange le code temporaire contre l'identité de l'App ; son identifiant, sa clé privée, son secret OAuth et son secret de webhook sont stockés scellés par organisation.
4. L'admin clique sur « Installer sur vos dépôts », choisit les dépôts sur GitHub, et revient connecté.

En local, `CONSOLE_URL` n'a besoin d'être joignable que par le navigateur : GitHub redirige le navigateur, il n'appelle jamais Capitainerie (pas de tunnel ni de webhook). Pour GitHub Enterprise, `GITHUB_API_BASE` et `GITHUB_WEB_BASE` remplacent les hôtes publics. Supprimer l'App dans la console l'oublie côté Capitainerie, ainsi que son installation ; elle reste sur GitHub, où il faut aussi la supprimer (lien affiché).

Isolation et moindre privilège :

- **DNS** : le pod ne résout rien. Le worker résout l'hôte de `EXECUTOR_PROXY_URL` à la création du Job et l'inscrit dans `hostAliases` ; le pod a `dnsPolicy: None` avec `127.0.0.1` pour seul serveur, et la `NetworkPolicy` des tâches n'autorise plus le DNS. Cela ferme l'exfiltration par requêtes DNS. L'IP du Service du proxy doit rester stable : utiliser un Service `ClusterIP` (le défaut du chart). Les pods déjà lancés gardent l'IP résolue à leur création.
- **Base du proxy** : le proxy ne fait que lire `task_grants`, `provider_credentials`, `github_installations` et `github_apps`. `deploy/postgres/egress-proxy-role.sql` crée le rôle `capitainerie_egress_proxy` avec `SELECT` sur ces seules tables ; lui donner un mot de passe (`ALTER ROLE capitainerie_egress_proxy PASSWORD '…'`), puis, dans le chart, pointer `egressProxy.databaseUrlSecretKey` vers la clé du Secret qui contient sa `DATABASE_URL`. Une faille du proxy ne donne alors ni lecture des autres tables ni écriture.
- **Révocation** : le proxy garde une grant valide 10 s en cache et refuse pendant 10 s un jeton inconnu ou révoqué. La révocation peut donc mettre jusqu'à 10 s à s'appliquer. Il sert au plus 16 requêtes simultanées par exécution (429 au-delà).
- **Modèle et outils** : le proxy ne laisse passer que le modèle de l'exécution, des réponses plafonnées à 32 000 tokens, des outils côté client (ni `web_search`, `code_execution`, `computer`, ni `mcp_servers`) et une liste fermée d'en-têtes `anthropic-beta`.
- **Usage** : les tokens et le coût viennent du flux `stream-json` de Claude Code, qui tourne dans le conteneur : ils sont indicatifs, pas une preuve de facturation.

Risque résiduel, GitHub Actions : une branche poussée par l'Exécutant peut déclencher des workflows qui disposent de secrets du dépôt. Restreindre les workflows qui utilisent des secrets aux branches protégées ou à des environnements (`environment:` avec relecteurs requis), et ne pas les déclencher sur `push` d'une branche `capitainerie/*`.

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
