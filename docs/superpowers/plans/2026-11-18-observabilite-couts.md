# Observabilité des coûts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quatre routes en lecture seule qui donnent le coût des runs sur une période `from`/`to`, par projet, agent, tâche et utilisateur, avec le détail agent (projet, tâche) ou modèle (utilisateur).

**Architecture:** Le motif de `budgets` : un service de domaine (`CostService`) avec un port `CostRepository` et l'`Authorizer`, un adapter Postgres sur `run_steps`, des méthodes `#[transactional]` sur `CapitainerieUseCase`, des handlers dans `handlers-audit`. Le dépôt renvoie des lignes de détail ; le service les replie en groupes et calcule les totaux, donc un détail somme toujours à son total.

**Tech Stack:** Rust, sqlx (requêtes vérifiées, cache `.sqlx`), axum, utoipa, Postgres.

**Spec:** `docs/specs/2026-11-observabilite-couts.md`

## Global Constraints

- Période : étapes avec `started_at` dans `[from, to[`, défaut 30 jours, `from < to` sinon `CoreError::Invalid` (422).
- Autorisation : `Action::AuditRead` / `ResourceKind::Audit`, comme `BudgetService::overview`. Un agent reçoit 403.
- Une étape sans coût compte 0. Tri : coût décroissant, puis nom.
- Isolation : toute requête filtre sur `organization_id` de l'appelant.
- Aucun commentaire sauf les « pourquoi » non évidents ; nommage et style de `domain/budget` et `infrastructure/audit/budget.rs`.
- Hors périmètre : chat du Captain, Parquet et `/analytics/costs`, console, pagination, délégation entre agents.

## Review Focus

- Organisation sans aucune étape : `rows` vide et `total` à 0, pas d'erreur.
- Run de carte sans projet, et run de pipeline sans projet : une ligne « sans projet » (`project_id: null`), jamais ignoré.
- Étape sans `model` ou sans agent identifiable : regroupée sous `"unknown"`, jamais perdue.
- Agent archivé : son coût passé reste visible sous son nom.
- Étape exactement à `to` exclue, exactement à `from` incluse.

---

### Task 1: Domaine `cost` : période, lignes, service

**Files:**
- Create: `libs/core/src/domain/cost/mod.rs`, `libs/core/src/domain/cost/ports.rs`, `libs/core/src/domain/cost/service.rs`
- Modify: `libs/core/src/domain/mod.rs` (ajouter `pub mod cost;`)

**Interfaces:**
- Consumes: `Subject`, `Authorizer`, `AccessRequest`, `Action::AuditRead`, `ResourceKind::Audit`, `require` (voir `domain/budget/service.rs`) ; `AgentId`, `CardId`, `ProjectId`, `OrganizationId`, `CoreError` de `capitainerie_common`.
- Produces (`mod.rs`) :
  - `pub const DEFAULT_WINDOW_DAYS: i64 = 30;`
  - `pub struct Period` avec `pub fn new(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Result<Self, CoreError>`, `pub fn from(&self)`, `pub fn to(&self)` ; champs privés.
  - `#[derive(Default, Clone, Copy)] pub struct Spent { pub input_tokens: i64, pub output_tokens: i64, pub cost_micros: i64 }` avec `pub fn add(&mut self, other: &Spent)`.
  - `pub struct AgentRef { pub id: Option<AgentId>, pub name: String }`
  - Lignes de détail renvoyées par le dépôt : `ProjectAgentRow { project_id: Option<ProjectId>, project_name: Option<String>, agent: AgentRef, runs: i64, spent: Spent }`, `TaskAgentRow { card_id: CardId, title: String, project_id: Option<ProjectId>, agent: AgentRef, runs: i64, spent: Spent }`, `UserModelRow { user: String, model: String, spent: Spent }`, `AgentRow { agent: AgentRef, runs: i64, spent: Spent }`.
  - Groupes : `AgentSpend { agent, runs, spent }`, `ProjectSpend { project_id, name, spent, agents: Vec<AgentSpend> }`, `TaskSpend { card_id, title, project_id, spent, agents: Vec<AgentSpend> }`, `ModelSpend { model, spent }`, `UserSpend { user, spent, models: Vec<ModelSpend> }`, `CostReport<T> { from, to, total: Spent, rows: Vec<T> }`.
- Produces (`ports.rs`) : `pub trait CostRepository: Send` (avec `#[cfg_attr(any(test, feature = "mock"), mockall::automock)]`), méthodes toutes `-> impl Future<Output = Result<_, CoreError>> + Send` :
  - `project_rows(&mut self, OrganizationId, &Period) -> Vec<ProjectAgentRow>`
  - `agent_rows(&mut self, OrganizationId, &Period) -> Vec<AgentRow>`
  - `task_rows(&mut self, OrganizationId, &Period, Option<ProjectId>) -> Vec<TaskAgentRow>`
  - `user_rows(&mut self, OrganizationId, &Period) -> Vec<UserModelRow>`
  - `project_exists(&mut self, OrganizationId, ProjectId) -> bool`
- Produces (`service.rs`) : `CostService<'a, R, A>::new(costs: R, authorizer: &'a A)` avec `async fn projects / agents / tasks(project: Option<ProjectId>) / users(self, subject: Subject, period: Period) -> Result<CostReport<_>, CoreError>`. `tasks` avec un `project` inconnu renvoie `CoreError::NotFound`.

- [ ] **Step 1: Tests de `Period` (dans `mod.rs`)**
  - `defaults_to_the_last_thirty_days`: `Period::new(None, None, now)` donne `to == now`, `from == now - 30 jours`.
  - `rejects_an_empty_or_reversed_window`: `from == to` et `from > to` renvoient `CoreError::Invalid`.
  - `keeps_explicit_bounds`: `from` et `to` fournis sont conservés tels quels.
- [ ] **Step 2: Lancer** `cargo test -p capitainerie-core cost::` ; attendu : échec de compilation (module absent).
- [ ] **Step 3: Implémenter** `Period::new` (même logique que `handlers-analytics/src/costs.rs::window`, mais renvoie `CoreError::Invalid`) et les types ci-dessus.
- [ ] **Step 4: Tests du service** avec `MockCostRepository` et l'`Authorizer` de test utilisé dans `domain/budget/service.rs` (ou son mock) :
  - `projects_group_agents_and_sum_to_the_total` : lignes `(P1, a1, 100)`, `(P1, a2, 50)`, `(None, a1, 10)` donnent `P1` à 150 avec deux agents, une ligne `project_id: None` à 10, `total == 160`, et pour chaque groupe `spent == somme de ses agents`.
  - `groups_are_ordered_by_cost_then_name`.
  - `tasks_with_an_unknown_project_are_not_found` : `project_exists` renvoie `false` donc `NotFound`, et `task_rows` n'est pas appelé.
  - `users_group_models_and_sum_to_the_total`.
  - `an_empty_organization_reports_zero` : aucune ligne donne `rows` vide et `total == Spent::default()`.
  - `a_refused_subject_reads_nothing` : l'`Authorizer` refuse, `Err(CoreError::Forbidden{..})`, aucune méthode du dépôt appelée.
- [ ] **Step 5: Implémenter `CostService`** : `require(AuditRead)` d'abord, puis repli des lignes (`BTreeMap`/`HashMap` par clé de groupe), totaux calculés en sommant les détails.
- [ ] **Step 6: Lancer** `cargo test -p capitainerie-core cost::` ; attendu : tout passe.
- [ ] **Step 7: Commit** `feat(core): cost reports domain`

---

### Task 2: Adapter Postgres et cas d'usage

**Files:**
- Create: `libs/core/src/infrastructure/cost/mod.rs`, `libs/core/src/infrastructure/cost/postgres.rs`, `libs/core/tests/cost.rs`
- Modify: `libs/core/src/infrastructure/mod.rs` (`pub mod cost;`), `libs/core/src/infrastructure/registry.rs` (`pub struct Cost;` dans `domain`), `libs/core/src/application/audit.rs` (4 méthodes), `.sqlx/` (régénéré)

**Interfaces:**
- Consumes: `CostRepository`, `Period`, lignes de la Task 1 ; `SharedTx`, `map_sqlx_error`, `#[repository(domain = Cost, backend = Postgres)]` (voir `infrastructure/audit/budget.rs`).
- Produces: `PgCostRepository<'tx>::new(&SharedTx<'tx>)` et, sur `CapitainerieUseCase`, des méthodes `#[transactional(cost, authz)]` :
  - `project_costs(&self, subject: Subject, period: Period) -> Result<CostReport<ProjectSpend>, CoreError>`
  - `agent_costs(...) -> CostReport<AgentSpend>`
  - `task_costs(&self, subject, period, project: Option<ProjectId>) -> CostReport<TaskSpend>`
  - `user_costs(...) -> CostReport<UserSpend>`

Requête commune (un seul gabarit, adapté par groupe) : `run_steps s JOIN runs r ON r.id = s.run_id`, `LEFT JOIN agent_tasks t ON t.run_id = s.run_id`, `LEFT JOIN agents a ON a.id = t.agent_id`, `LEFT JOIN cards c ON c.id = r.card_id`, `LEFT JOIN transcripts tr ON tr.id = r.transcript_id`, `LEFT JOIN projects p ON p.id = COALESCE(c.project_id, tr.project_id)` ; filtre `s.organization_id = $1 AND s.started_at >= $2 AND s.started_at < $3` ; agent = `a.id` et `COALESCE(a.name, s.agent, 'unknown')` ; modèle = `COALESCE(s.model, 'unknown')` ; sommes avec `COALESCE(sum(...), 0)::bigint` ; `runs` = `count(DISTINCT s.run_id)`. Les tâches ne gardent que `r.card_id IS NOT NULL` ; l'utilisateur est `r.initiated_by`.

- [ ] **Step 1: Écrire les tests d'intégration** dans `libs/core/tests/cost.rs` (même squelette que `tests/postgres.rs` : `database_url`, `pool`, `organization`, skip sans `DATABASE_URL`). Un helper insère un run, ses étapes, et au besoin `transcripts`/`cards`/`projects`/`agent_tasks`. Tests, un par Acceptance :
  - `projects_list_their_agents_and_a_projectless_row` (Acceptance 1) : deux projets plus un run sans projet donnent 3 lignes ; la somme des agents de chaque ligne égale son total.
  - `agents_cover_agent_tasks_and_pipeline_roles` (2) : un run d'agent (agent de `agents`) et un run de pipeline (`run_steps.agent = 'extractor'`) apparaissent chacun sous leur nom.
  - `a_task_adds_every_run_of_its_card` (3) : une carte avec deux runs d'agents différents a un total égal à la somme et deux agents.
  - `users_list_their_models` (4) : deux utilisateurs, deux modèles, détail par modèle correct.
  - `the_period_is_half_open` (5) : étape avant `from` exclue, à `from` incluse, à `to` exclue.
  - `another_organization_never_shows` (7) : des étapes d'une autre organisation n'apparaissent dans aucune des quatre vues ; `task_costs` avec le `project_id` de l'autre organisation renvoie `NotFound`.
  - `missing_model_and_agent_fall_under_unknown` : étape sans `model` ni agent donne la clé `"unknown"`.
  - `an_archived_agent_keeps_its_past_cost`.
- [ ] **Step 2: Lancer** `cargo test -p capitainerie-core --test cost` avec `DATABASE_URL` défini ; attendu : échec (adapter absent).
- [ ] **Step 3: Implémenter `PgCostRepository`** (4 requêtes `sqlx::query!` plus `project_exists`), l'enregistrer dans le registre, ajouter les 4 méthodes `#[transactional(cost, authz)]` qui appellent `CostService::new(cost_repository, authz)` avec `Period` déjà construit par l'appelant.
- [ ] **Step 4: Régénérer le cache** `cargo sqlx prepare --workspace` (comme pour `budgets`) ; vérifier que `.sqlx/` contient les nouvelles requêtes.
- [ ] **Step 5: Lancer** `cargo test -p capitainerie-core --test cost` ; attendu : tout passe. Si `runs.card_id` est NULL pour les runs d'agent (Acceptance 3 échoue), rattacher la carte via `agent_tasks.card_id` dans la jointure `cards` et le noter dans la spec.
- [ ] **Step 6: Commit** `feat(core): postgres cost reports`

---

### Task 3: Routes, OpenAPI et tests d'API

**Files:**
- Create: `libs/handlers-audit/src/costs.rs`
- Modify: `libs/handlers-audit/src/lib.rs` (4 routes), `libs/handlers-audit/tests/api.rs`, `apps/api/src/openapi.rs` (chemins, schémas, test de présence)

**Interfaces:**
- Consumes: `CapitainerieUseCase::{project_costs, agent_costs, task_costs, user_costs}`, `Period::new`, types `CostReport` etc.
- Produces: routes `GET /api/v1/analytics/costs/{projects,agents,tasks,users}` ; query `from`, `to` (`DateTime<Utc>` optionnels), `project_id` (tâches). Réponse `{from, to, total: {input_tokens, output_tokens, cost_micros}, rows: [...]}` ; `rows` de projet : `{project_id|null, name|null, input_tokens, output_tokens, cost_micros, agents: [{agent_id|null, name, runs, input_tokens, output_tokens, cost_micros}]}` ; tâche : `{card_id, title, project_id|null, ..., agents}` ; utilisateur : `{user, ..., models: [{model, input_tokens, output_tokens, cost_micros}]}` ; agent : `{agent_id|null, name, runs, ...}`. Handlers `projects_handler`, `agents_handler`, `tasks_handler`, `users_handler`.

- [ ] **Step 1: Tests d'API** dans `tests/api.rs` (helpers `Fixture`/`Who` existants) :
  - `costs_require_a_human_member`: `Who::Agent` reçoit 403 sur les quatre routes (Acceptance 8).
  - `costs_reject_an_empty_window`: `from=2026-11-02T00:00:00Z&to=2026-11-01T00:00:00Z` renvoie 422 (6).
  - `costs_default_to_thirty_days`: sans paramètres, `to - from == 30 jours` dans la réponse (6).
  - `project_costs_return_agents_and_total`: après insertion de données, forme JSON ci-dessus, `total.cost_micros` = somme des lignes.
  - `tasks_with_a_foreign_project_are_not_found`: `project_id` d'une autre organisation renvoie 404 (7).
- [ ] **Step 2: Lancer** `cargo test -p capitainerie-handlers-audit costs` ; attendu : échec (routes absentes).
- [ ] **Step 3: Implémenter `costs.rs`** : un `TypedPath` par route, DTO de réponse (`Serialize` + `ToSchema`) avec `From<CostReport<_>>`, `#[utoipa::path]` par handler (200, 401, 403, 422, plus 404 pour les tâches), tag `super::TAG`. Convertir `CoreError::Invalid`/`NotFound` comme les autres handlers (`ApiError::from`). Enregistrer les quatre routes dans `router()`.
- [ ] **Step 4: OpenAPI** : ajouter les 4 handlers à `paths(...)` de `apps/api/src/openapi.rs` et, dans son test existant, assert que les quatre chemins sont des objets et que `ProjectCostsResponse` expose `rows`.
- [ ] **Step 5: Lancer** `cargo test -p capitainerie-handlers-audit` et `cargo test -p capitainerie-api openapi` ; attendu : tout passe.
- [ ] **Step 6: Commit** `feat(api): cost reports by project, agent, task and user`

---

### Task 4: Documentation et vérification de sortie

**Files:**
- Modify: `docs/ARCHITECTURE.md` (section 5 « Coût » et section 6 « Observabilité » : ajouter les quatre routes et préciser qu'elles lisent Postgres, pas Parquet), `docs/specs/2026-11-observabilite-couts.md` (O6 : `AuditRead` au lieu de `human_only` ; O7 : routes dans `handlers-audit` ; `runs` retiré des projets et tâches ; statut « réalisée »)

- [ ] **Step 1: Mettre à jour les deux documents** comme ci-dessus.
- [ ] **Step 2: Vérification de sortie** (une seule fois) :
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p capitainerie-core cost` et `cargo test -p capitainerie-core --test cost` (avec `DATABASE_URL`)
  - `cargo test -p capitainerie-handlers-audit`
  - `cargo test -p capitainerie-api openapi`
  Attendu : tout vert. Sinon, nommer la commande et l'erreur.
- [ ] **Step 3: Commit** `docs: cost reports`

---

## Self-review

- **Couverture de la spec :** Acceptance 1 à 8 sont dans les tests des Tasks 2 et 3 (1 à 5 et 7 en SQL, 6 à 8 en API, plus 6 et 7 dans le service). Invariant 1 : `another_organization_never_shows`. Invariant 3 : repli du service, test `projects_group_agents_and_sum_to_the_total`. Invariant 5 : 404 en Tasks 2 et 3.
- **Écart avec la spec :** autorisation par `Action::AuditRead` (PDP) au lieu de `human_only`, et routes dans `handlers-audit` plutôt que `handlers-analytics`, parce que ce crate n'a pas accès à Postgres ni au PDP. Corrigé dans la spec en Task 4.
- **Écart avec la spec :** `runs` n'est pas porté au niveau projet ou tâche (un run de pipeline compte pour chacun de ses agents, la somme serait fausse) ; il reste par agent.
- **Cohérence des types :** `Period`, `Spent`, `AgentRef`, lignes et groupes sont définis en Task 1 et réutilisés tels quels en Tasks 2 et 3.
