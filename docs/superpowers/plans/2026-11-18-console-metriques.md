# Console Métriques Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Filtres `agent_id` et `card_id` sur les rapports de coûts, puis une page console « Métriques » qui les affiche.

**Architecture:** Le repository reçoit un `CostFilter` et le pose dans le WHERE des quatre requêtes ; le service vérifie que les ids existent (404). La console ajoute un domaine `metrics` (hooks, modèle, UI, page) et une route `/metrics/{-$tab}`.

**Tech Stack:** Rust, sqlx, axum, utoipa ; React 19, TanStack Router/Query, Vitest, Biome.

**Spec:** `docs/specs/2026-11-console-metriques.md`

## Global Constraints

- Branche PR 1 : `feat/cost-filters`, base `feat/cost-observability`. Branche PR 2 : `feat/console-metrics`, base `feat/cost-filters`.
- Jamais `git add Cargo.lock`. Aucun co-auteur ni mention d'IA dans commits et PR.
- Pas de commentaires inutiles ; composants avec les tokens de `styles.css`, jamais une couleur brute.
- `.sqlx` régénéré avec `cargo sqlx prepare --workspace` (base de test, `DATABASE_URL`).
- Tests console : `pnpm --dir apps/console test` ; lint : `pnpm --dir apps/console check`.

## Review Focus

- Filtre agent + filtre carte ensemble : cumul (ET), détail = total. Test dans Task 1.
- `agent_id` d'une autre organisation : 404, pas liste vide. Test dans Task 2.
- Utilisateurs filtré par agent : seuls les runs de cet agent comptent (la requête n'avait pas la jointure agent). Test dans Task 1.
- Période où le filtre ne laisse rien : état vide, pas d'erreur. Test dans Task 4.
- 403 : message d'accès. Test dans Task 4.

---

### Task 1: Filtre dans le domaine et le repository

**Files:**
- Modify: `libs/core/src/domain/cost/mod.rs`, `ports.rs`, `service.rs`, `libs/core/src/infrastructure/cost/postgres.rs`, `libs/core/src/application/audit.rs`
- Test: `libs/core/tests/cost.rs`

**Interfaces:**
- Produces: `pub struct CostFilter { pub agent: Option<AgentId>, pub card: Option<CardId> }` (`Clone, Copy, Debug, Default, PartialEq, Eq`), exportée par `domain::cost`.
- Produces: port `CostRepository` : `project_rows(org, &Period, &CostFilter)`, `agent_rows(org, &Period, &CostFilter)`, `task_rows(org, &Period, project: Option<ProjectId>, &CostFilter)`, `user_rows(org, &Period, &CostFilter)`, `agent_exists(org, AgentId) -> bool`, `card_exists(org, CardId) -> bool`.
- Produces: `CostService` et `CapitainerieUseCase::{project_costs, agent_costs, task_costs, user_costs}` prennent un `CostFilter` en dernier argument ; `NotFound` si `agent` ou `card` n'existe pas dans l'organisation.

- [ ] **Step 1: Tests Postgres (rouge).** Dans `libs/core/tests/cost.rs`, trois tests : `agent_filter_narrows_every_report` (deux agents, un run chacun, `agent_costs`, `project_costs`, `task_costs`, `user_costs` filtrés sur l'un ne comptent que son run, détails = totaux), `card_filter_narrows_every_report` (deux cartes), `agent_and_card_filters_combine` (agent A sur carte 1 seulement).
- [ ] **Step 2:** `cargo test -p capitainerie-core --test cost` : ne compile pas (signatures).
- [ ] **Step 3: Implémenter** `CostFilter`, le port, le service et les quatre requêtes : `AND ($n::uuid IS NULL OR a.id = $n) AND ($m::uuid IS NULL OR r.card_id = $m)`. `user_rows` gagne les `LEFT JOIN agent_tasks` et `agents` des autres requêtes. `agent_exists` et `card_exists` sur le modèle de `project_exists`. Le service appelle ces deux vérifications avant les lignes quand le filtre est posé.
- [ ] **Step 4:** `cargo sqlx prepare --workspace`, puis `cargo test -p capitainerie-core --test cost` : vert.
- [ ] **Step 5: Test 404 :** `unknown_agent_or_card_is_not_found` (ids d'une autre organisation et ids inconnus) ; vert.
- [ ] **Step 6: Commit** `feat(core): agent and task filters on cost reports` (sans `Cargo.lock`).

### Task 2: Routes, OpenAPI, docs

**Files:**
- Modify: `libs/handlers-audit/src/costs.rs`, `apps/api/src/openapi.rs`, `docs/ARCHITECTURE.md`
- Test: `libs/handlers-audit/tests/api.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces: `agent_id: Option<AgentId>` et `card_id: Option<CardId>` dans `CostsQuery` et `TaskCostsQuery`.

- [ ] **Step 1: Tests API (rouge) :** `unknown_agent_filter_is_not_found` et `unknown_card_filter_is_not_found` (404, quatre routes via une boucle), `filters_are_accepted` (200 avec les deux paramètres sur une base vide).
- [ ] **Step 2:** Ajouter les deux champs aux deux structs de query et les passer à l'use-case ; documenter les paramètres.
- [ ] **Step 3:** `cargo test -p capitainerie-handlers-audit` et `cargo test -p capitainerie-api openapi` : vert. Mettre à jour la ligne des routes dans `docs/ARCHITECTURE.md`.
- [ ] **Step 4:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (avec `DATABASE_URL`) : vert.
- [ ] **Step 5: Commit** `feat(api): agent and task filters on cost routes`, puis push et PR 1 (base `feat/cost-observability`).

### Task 3: Client API et modèle console

**Files:**
- Modify: `apps/console/src/api/api.client.ts`, `api.tanstack.ts` (générés)
- Create: `apps/console/src/domain/metrics/model.ts`, `model.test.ts`, `hooks/use-metrics.ts`, `tabs.ts`, `tabs.test.ts`

**Interfaces:**
- Produces: `METRICS_TABS` (`projets`, `agents`, `taches`, `utilisateurs`), `parseMetricsTab(value?: string): MetricsTab` (défaut `projets`).
- Produces: `PERIODS` (`7j`, `30j`, `mois`, `libre`) et `periodWindow(period: Period, now: Date, custom?: {from: string; to: string}): {from: string; to: string}` en ISO UTC ; `to` exclusif. `7j`/`30j` : `to = now`, `from = now - N jours`. `mois` : réutilise `monthWindow` de `domain/audit/model`. `libre` : dates `AAAA-MM-JJ` locales vers UTC, `to` = jour suivant minuit.
- Produces: `useProjectCosts`, `useAgentCostReport`, `useTaskCosts`, `useUserCosts` `(window, filters: {agentId?: string; cardId?: string})` via `tanstackApi.get(...).queryOptions`, `enabled` sur le jeton.

- [ ] **Step 1: Tests (rouge)** : `tabs.test.ts` (slug inconnu → `projets`) ; `model.test.ts` : `periodWindow('7j', 2026-11-18T12:00Z)` donne `from` = 2026-11-11T12:00Z, `periodWindow('libre', …, {from:'2026-11-01', to:'2026-11-03'})` donne `to` = 2026-11-04 minuit, `from >= to` détecté par `isValidWindow`.
- [ ] **Step 2:** API lancée sur la branche `feat/cost-filters`, `pnpm --dir apps/console gen:api`. Vérifier que le diff ne contient que les nouvelles routes.
- [ ] **Step 3:** Implémenter `tabs.ts`, `model.ts` (+ `isValidWindow`), hooks. `pnpm --dir apps/console test` : vert.
- [ ] **Step 4: Commit** `feat(console): metrics api client and model`.

### Task 4: Page, route, menu

**Files:**
- Create: `apps/console/src/domain/metrics/feature/metrics-page.tsx`, `metrics-page.test.tsx`, `ui/period-picker.tsx`, `ui/metrics-filters.tsx`, `ui/cost-table.tsx`, `apps/console/src/routes/_app.metrics.{-$tab}.tsx`
- Modify: `components/layout/nav-items.ts` (entrée `Métriques`, icône `ChartColumn`, groupe Organisation, `to: '/metrics/{-$tab}'`), `components/layout/breadcrumbs.ts` (`metrics: 'Métriques'`), `routeTree.gen.ts` (généré)

**Interfaces:**
- Consumes: Task 3.
- Produces: `MetricsPage({ tab })`.

- [ ] **Step 1: Tests (rouge)** avec le mock d'API de `test-utils/api-mock.tsx` : l'onglet Projets affiche une ligne et se déplie sur ses agents ; choisir un agent recharge avec `agent_id` ; un 403 affiche « Tu n'as pas accès aux métriques de l'organisation. » ; un rapport vide affiche l'état vide ; les tuiles montrent le `total` de l'API.
- [ ] **Step 2:** Implémenter : en-tête, `TabNav` (un lien par onglet), `PeriodPicker` (select + deux `DateInput` pour `libre`, 422 évité par `isValidWindow`), filtres (agents de `useTeamAgents`, tâches de la liste des cartes avec recherche via le combobox existant), tuiles (coût, jetons entrée, jetons sortie), `CostTable` à lignes dépliables, états chargement, erreur, vide.
- [ ] **Step 3:** Route `/metrics/{-$tab}` sur le modèle d'`audit`, entrée de menu, fil d'Ariane.
- [ ] **Step 4:** `pnpm --dir apps/console check`, `test`, `build` : verts. Vérifier dans le navigateur la page contre l'API locale.
- [ ] **Step 5: Commit** `feat(console): metrics page`, push, PR 2 (base `feat/cost-filters`).

## Self-review

- Couverture : M1–M4 Tasks 1-2 ; M5–M9 Tasks 3-4 ; Acceptance 1-3 Tasks 1-2, 4-7 Tasks 3-4.
- Types : `CostFilter` défini Task 1, utilisé Task 2 ; `periodWindow` et hooks définis Task 3, utilisés Task 4.
- Proportion : le plan donne signatures, tests et valeurs ; les corps sont laissés à l'implémenteur.
