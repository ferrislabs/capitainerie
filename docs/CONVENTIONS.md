# Conventions Capitainerie

Synthèse tirée des deux repos de référence : `../mestier` (ERP, le plus récent) et `../autharie` (control plane FerrisKey).
Chaque règle cite sa source. Les points où les deux repos divergent sont regroupés en fin de document, section **À trancher**. Tant qu'un point n'est pas tranché, il reste ouvert.

Légende des sources : **M** = mestier, **A** = autharie.

---

## 1. Workspace Rust

| Règle | Source |
|---|---|
| Edition 2024. | M `Cargo.toml:5`, A `Cargo.toml` |
| `apps/` = binaires sans logique métier (assemblent routeur, état, OpenAPI). `libs/` = tout le reste. | M `CLAUDE.md`, A `AGENTS.md` |
| Un seul crate domaine (`core`) découpé en `domain/` → `application/` → `infrastructure/`, un sous-dossier par bounded context dans chaque couche. | M `libs/core/src/` |
| Handlers HTTP dans des crates `handlers-<zone>`, un fichier par opération. `libs/handlers` porte `AppState` et `ApiError`. | M `libs/handlers-*`, `libs/handlers/src/state.rs` |
| Crate `events` : trait `DomainEvent`, enveloppe persistée, catalogue. Il ne dépend pas de `core`. | M `libs/events` |
| Crate `macros` : `#[transactional]` et `#[repository]`. | M `libs/macros`, A `libs/autharie-macros` |
| Crate `args` : config clap partagée. | M `libs/args` |
| Ajouter un module = éditer un registre, jamais un `match` (membres du workspace, marqueurs de repository, routeur, tags OpenAPI). | M `CLAUDE.md` « Where things register » |

Direction des dépendances : `apps → handlers-* → core → {common, events, auth, authz}`. Le domaine n'importe rien de l'infrastructure.

## 2. Architecture hexagonale

```
libs/core/src/
  domain/<contexte>/      entités, value objects, ports.rs (traits), services purs génériques sur les ports
  application/<contexte>/ use cases = méthodes #[transactional] sur une struct applicative
  infrastructure/<contexte>/postgres/{model.rs, repository.rs}
```

- **Ports** : traits définis dans `domain/<contexte>/ports.rs`. Exemple : M `libs/core/src/domain/customer/ports.rs:7`, A `libs/autharie-domain/src/deployments/ports.rs:162`.
- **Services de domaine** : structs génériques sur leurs ports, sans I/O directe. Exemple : A `libs/autharie-domain/src/deployments/service.rs:110` (`DeploymentServiceImpl<D, U, DP, O, CP, P>`).
- **Couche application** (décision D1, comme mestier) : une struct `CapitainerieUseCase` (sur le modèle de `MestierUseCase`, M `libs/core/src/application/mod.rs:133`). Chaque use case est une méthode `#[transactional(repo1, repo2, authz, emitter)]` : la macro ouvre la transaction, injecte les repositories nommés, puis commit. La méthode instancie le service de domaine avec ces repositories et lui délègue. Aucune règle métier dans cette couche.

```rust
impl CapitainerieUseCase {
    #[transactional(run, transcript, authz, emitter)]
    pub async fn submit_transcript(&self, command: SubmitTranscript) -> Result<Run, CoreError> {
        RunService::new(run_repository, transcript_repository, emitter, authz)
            .submit(command)
            .await
    }
}
```

- `acting_as(user_id)` renvoie une vue de la struct applicative dont les événements sont attribués à l'utilisateur (M `application/mod.rs`).
- **Handlers** : extraient, vérifient l'appartenance, appellent `state.usecase.<méthode>()`, convertissent en réponse. Forme type (M `libs/handlers-organization/src/get_one.rs`) :

```rust
#[utoipa::path(get, path = "/api/v1/...", tag = super::TAG, responses(...), security(("bearer_auth" = [])))]
pub async fn handler(
    XPath { id }: XPath,
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<Response<XResponse>, ApiError> { ... }
```

- Tout ce qui est request-scoped reste hors de la struct applicative : elle vit longtemps et elle est clonée à chaque requête. M `CLAUDE.md` signale une fuite d'événements entre organisations causée par ce type d'erreur.

## 3. Async dans les traits

**Confirmé dans les deux repos** : `fn ... -> impl Future<Output = ...> + Send`, et `async fn` dans les `impl`.

```rust
pub trait XRepository: Send {
    fn find_by_id(&mut self, id: XId) -> impl Future<Output = Result<Option<X>, CoreError>> + Send;
}
impl<'tx> XRepository for PgXRepository<'tx> {
    async fn find_by_id(&mut self, id: XId) -> Result<Option<X>, CoreError> { ... }
}
```

M : `async-trait` n'est jamais utilisé directement. A : il apparaît seulement dans `autharie-operator-core` et `genesis-core`, à la frontière avec des API tierces. **Règle : pas de `async-trait`.**

## 4. Dispatch

**Confirmé** : dispatch statique.
- Services génériques sur leurs ports (A `service.rs:110`, M `apps/mcp-gateway/src/app.rs:81` `V: TokenVerifier, P: PolicyDecisionPoint, X: TokenExchanger`).
- Alias de type concrets à l'assemblage (M `libs/core/src/application/mod.rs:72-79`, par exemple `MestierAuthorizer = LocalPolicyEngine`).
- `dyn` toléré seulement en bordure d'infrastructure quand il n'y a pas d'autre choix (A `herald-core/.../otlp_receiver.rs:33`).

## 5. Erreurs

| Règle | Source |
|---|---|
| `thiserror` 2 dans les libs. | M + A |
| Un `CoreError` partagé : `NotFound`, `Conflict(String)`, `Forbidden { reason }`, `Database(String)`, `Internal(String)`. | M `libs/common/src/lib.rs:67`, A `libs/autharie-domain/src/lib.rs:108` |
| `ApiError` dans la couche HTTP, `impl IntoResponse`, corps JSON `{ code, message, status, details? }`, `From<CoreError> for ApiError`. | M `libs/handlers/src/errors.rs:192`, A `libs/autharie-api/src/errors.rs:170` |
| Erreurs SQL mappées par une fonction unique (`map_sqlx_error`). | M `infrastructure/postgres/error.rs` |
| `anyhow` **seulement dans les binaires** (`apps/*/src/main.rs` et leurs modules), jamais dans `libs/` : les libs sont 100 % `thiserror` (décision D5). Les deux repos utilisent `Box<dyn Error>` à cet endroit. | Décision D5 |

## 6. Conversions et types

- IDs en newtypes : `struct CustomerId(pub Uuid)` (M `domain/customer/mod.rs:15`), `RoleId(pub Uuid)` (A).
- Value objects validés avec un champ privé et `TryFrom<String>` : A `libs/autharie-domain/src/cells/realm_name.rs:14,32`.
- Lignes DB : struct `*Row` privée côté infrastructure, convertie vers le domaine.
  - M : `TryFrom<XRow> for X` ou `From` (22 `TryFrom`, 28 `From` sous `infrastructure/`).
  - A : méthode inhérente `row.into_role()`.
  - Le prompt demande `TryFrom` : on suit M (voir la divergence D7).
- UUID v7 pour les identifiants ordonnés (M `libs/common/src/lib.rs`, compteur monotone).

## 7. Base de données

| Règle | Source |
|---|---|
| sqlx 0.8.6, Postgres, `runtime-tokio`, `tls-rustls`. | M + A |
| `query!` / `query_as!` par défaut (M environ 600 sites, A environ 144). Requêtes runtime seulement pour le SQL dynamique (filtres, tris). | M + A |
| Mode offline : `.sqlx/` versionné, `SQLX_OFFLINE=true` en CI, `cargo sqlx prepare --workspace -- --all-targets` après chaque nouvelle requête. | M `CLAUDE.md`, A `AGENTS.md` |
| Migrations `YYYYMMDDHHMMSS_snake_name.{up,down}.sql`, réversibles, `down` testé. | M `migrations/` (174), A `libs/autharie-core/migrations/` (92) |
| Repository = struct `PgXRepository<'tx>` qui emprunte une `SharedTx<'tx>`, annotée `#[repository(domain = X, backend = Postgres)]`. | M `infrastructure/customer/postgres/repository.rs:14`, A `libs/autharie-persistence/src/lib.rs:38` |
| `#[transactional(...)]` ouvre la transaction, injecte les repositories nommés, commit. | M `libs/macros`, A `libs/autharie-macros/src/lib.rs:108` |
| Les événements de domaine sont persistés dans la transaction, avant le commit : un événement existe si et seulement si sa transaction a commité. | M `CLAUDE.md` (`with_tx_emitting`) |
| Multi-tenant : chaque requête est filtrée par l'organisation. | M `CLAUDE.md` |

## 8. HTTP, auth, authz

- axum 0.8, `axum-extra` `TypedPath` pour les chemins (M `libs/handlers-organization/src/lib.rs:3`, A `get_deployment.rs:33`).
- utoipa 5, un `#[utoipa::path]` par handler, Scalar et Swagger UI, `operation_id` en camelCase.
- Auth : JWT FerrisKey vérifié via JWKS `{issuer}/protocol/openid-connect/certs`, `jsonwebtoken` 10 avec `aws_lc_rs` (A `ferriskey_repository.rs:100`, M `libs/auth`). `Identity` injectée en `Extension`.
- **FerrisKey n'est pas Keycloak : ne jamais modéliser son API sur celle de Keycloak** (M `CLAUDE.md`).
- Authz :
  - M : port de forme AuthZEN (`libs/authz/src/domain/ports.rs:13`), moteur local `LocalPolicyEngine`, client `AuthzenPdp` dans `apps/mcp-gateway/src/pdp.rs:73`.
  - A : bits de permission (`AuthariePolicy::require`).
  - Pour Capitainerie, le prompt impose le PDP AuthZEN de FerrisKey : on suit la forme M.
- MCP : `rmcp` (`#[tool_router]`, `#[tool]`), transport streamable HTTP, challenge `WWW-Authenticate: Bearer` et `resource_metadata` RFC 9728 (M `apps/mcp-server/src/app.rs:53`, `apps/mcp-gateway/src/app.rs:107`).

## 9. Config et observabilité

- clap 4 `derive` + `env`, dans un crate `args` ; `.env` chargé par `dotenvy` (M `libs/args/src/server.rs:82`, A `libs/autharie-api/src/args.rs:14`).
- `tracing` + `tracing-subscriber` (env-filter, json). M exporte traces, métriques et logs en OTLP (`apps/api/src/logger.rs:19-90`), activé par `ACTIVE_OBSERVABILITY`. A n'a pas de SDK OpenTelemetry. **On suit M**, puisque le prompt exige OTel dès le premier jalon.

## 10. Tests

| Niveau | Règle | Source |
|---|---|---|
| Unitaire | `mod tests` inline, sur le domaine. Mocks `#[cfg_attr(any(test, feature = "mock"), mockall::automock)]` sur les ports. | M `ports.rs:6`, A `reachability_history.rs:152` |
| Intégration DB | Contre un vrai Postgres, sans testcontainers ni `sqlx::test`. Mécanisme d'activation divergent : voir D4. | M + A |
| BDD | cucumber 0.23 ; features `.feature` dans `tests/features*/`, runner `tests/<nom>_bdd/main.rs`. | M `libs/core/tests/role_bdd`, A `libs/autharie-e2e/tests/bdd.rs` |
| Propriétés | proptest (A `libs/autharie-e2e/tests/properties.rs`). | A |
| Bench | divan (codspeed). | M `libs/auth/benches` |

## 11. Lint et format

- Pas de `rustfmt.toml`, `clippy.toml` ni `[workspace.lints]` dans les deux repos : rustfmt et clippy par défaut.
- CI : `cargo fmt --all -- --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `unwrap` / `expect` ne sont interdits nulle part (A en compte plus de 1 600 hors tests). Le prompt les interdit : voir D8.

## 12. CI (GitHub Actions)

M `.github/workflows/docker.yaml`, le plus complet :
- toolchain épinglée (`dtolnay/rust-toolchain@1.95.0`), `Swatinem/rust-cache` ;
- jobs en matrice `fmt`, `clippy`, `test` ;
- job `integration` avec un service Postgres, `sqlx-cli 0.8.6`, `sqlx migrate run`, `--test-threads=1` ;
- job `webapp` : `pnpm install --frozen-lockfile`, `pnpm check`, `pnpm test` ;
- build d'images multi-arch vers `ghcr.io/ferrislabs/<app>`.

A ajoute un job `generated-client-is-current` : il régénère le client OpenAPI du front et échoue s'il y a un diff. **À reprendre.**

## 13. Dev local et conteneurs

- `docker-compose` à la racine, `.env` / `.env.example`.
- FerrisKey en local : images `ghcr.io/ferriskey/ferriskey-api` (+ job de migrations) et `ferriskey-webapp` (A `docker-compose.yaml:136-177`).
- Dockerfile racine multi-stage : `rust:<version>` + `cargo-chef` + `sqlx-cli`, runtime `debian:bookworm-slim` en utilisateur non-root ; front buildé en `node:24-alpine`, servi par `nginx:1.28-alpine`.

## 14. Front

| Sujet | Règle commune | Source |
|---|---|---|
| Base | React 19, TypeScript strict (`noUnusedLocals`, `noUnusedParameters`, `verbatimModuleSyntax`), Vite, pnpm 9.15.0. | M + A |
| Données | TanStack Query. Client **généré** par `typed-openapi` depuis l'OpenAPI de l'API (`src/api/api.client.ts`, `api.tanstack.ts`), jamais édité à la main. | M `package.json:13`, A `scripts/generate-api-client.sh` |
| Fetch | `src/api/api.fetch.ts` : injecte `Authorization: Bearer`, lève une erreur typée (`status`, `code`, `message`) si la réponse n'est pas 2xx. A retente seulement sur 408, 429 et 5xx. | M + A |
| Config runtime | `/config.json` servi par nginx, rempli au démarrage du conteneur (`envsubst` ou `sed`) : une image pour tous les environnements. | M `docker-entrypoint.sh`, A idem |
| OIDC | `oidc-client-ts` + `react-oidc-context`. `automaticSilentRenew: false`, renouvellement maison sous `navigator.locks` : un seul onglet renouvelle, car FerrisKey fait tourner les refresh tokens et révoque toute la famille en cas de rejeu. `WebStorageStateStore(localStorage)`. Garde d'auth sur le layout authentifié. | M `src/lib/oidc.ts`, A `src/lib/auth/session.ts` |
| UI | Tailwind v4 en mode CSS-first (pas de `tailwind.config`), shadcn/ui (Radix, CVA), `lucide-react`, `cn()` = clsx + tailwind-merge, dark mode par classe (`@custom-variant dark`), tokens dans `:root` et `.dark`. | M `src/styles.css`, A `src/index.css` |
| Découpage | Chaque page = `feature/` (données, état, effets) + `ui/` (présentationnel : pas de hooks, pas de fetch, tout passe par les props). | M `CLAUDE.md`, A `AGENTS.md` |
| Nommage | Fichiers en kebab-case, composants en PascalCase, `interface Props`. | M + A |
| Tests | Vitest, tests colocalisés `*.test.ts(x)`. | M + A |
| CI front | `install --frozen-lockfile`, lint, test, build. | M + A |

Les points de divergence front (routing, store, lint, dossiers, tests de composants) sont listés en D9 à D12.

## 15. Git

- Les deux repos utilisent `type(scope): sujet (#PR)` (par exemple `feat(maintenance): ...`).
- Capitainerie impose `<type>(<subject>): <verbe><message>`.
- Claude n'apparaît jamais comme auteur, pas de ligne `Co-Authored-By`.

---

## À trancher

Chaque point indique ma recommandation et sa raison. Rien n'est appliqué tant que tu n'as pas répondu.

| # | Sujet | mestier | autharie | Recommandation |
|---|---|---|---|---|
| D1 | Couche use-case | Oui : `MestierUseCase`, `#[transactional]` | Oui : `AutharieService` | ✅ **Tranché** : comme mestier. |
| D2 | Découpage du domaine | Un crate `core` avec `domain/application/infrastructure` | Crates séparés `-domain`, `-core`, `-postgres`, `-persistence` | M pour le domaine et Postgres. Les adapters lourds (Rig, LanceDB, Iggy, DataFusion) vont dans des crates dédiés : Rig reste confiné et le temps de compilation baisse. |
| D3 | Dépendances | `[workspace.dependencies]` | Versions par crate | M. |
| D4 | Tests DB | `#[ignore]`, lancés avec `-- --ignored` | Skip si `DATABASE_URL` est absent, échec si `REQUIRE_DATABASE_URL=1` (CI) ; `cargo nextest` | A : un test ignoré par erreur reste visible en CI. |
| D5 | `anyhow` | Absent (`Box<dyn Error>`) | Absent | ✅ **Tranché** : `anyhow` dans les binaires, `thiserror` partout dans les libs. |
| D6 | Préfixe des crates | Sans préfixe (`core`, `auth`), sauf `mestier-core` | `autharie-*` | Préfixe `capitainerie-*` sur les noms de package, chemins de dossier courts (`libs/core`). |
| D7 | Row → domaine | `TryFrom` / `From` | `into_x()` | `TryFrom` (conforme au prompt). |
| D8 | `unwrap` / `expect` | Non interdits | Non interdits | `[workspace.lints.clippy]` : `unwrap_used = "deny"`, `expect_used = "deny"`, avec `allow-unwrap-in-tests` dans `clippy.toml`. |
| D9 | Routing front | TanStack Router basé sur les fichiers (`routes/`, notation à points), TanStack Start en mode SPA | TanStack Router basé sur le code (`router.tsx`) | M : routes typées générées, fichiers de route minces. |
| D10 | État global front | `@tanstack/store` | `zustand` | M : une dépendance de moins, même famille que le reste. |
| D11 | Lint et format front | Biome (tabs, quotes simples, sans `;`) | ESLint flat + Prettier (2 espaces) | M : un seul outil, plus rapide. |
| D12 | Dossiers front | `pages/<domaine>/{feature,ui}`, `hooks/use-*.ts` | `domain/<domaine>/{services,hooks,types,pages/{feature,ui}}` | A : tout un domaine dans un seul dossier, mieux adapté aux domaines Capitainerie (runs, cartes, agents). |
| D13 | Tests de composants | jsdom + Testing Library | Aucun | M. |
| D14 | Toolchain Rust | Épinglée `1.95.0` | `stable` | ✅ Épinglée à `1.97.1` dans `rust-toolchain.toml` (version installée en local). |
