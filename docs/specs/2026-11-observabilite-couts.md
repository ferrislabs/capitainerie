# Spec : observabilité des coûts

Statut : à approuver. Taille M, une PR.

Goal:
  Lire le coût des runs sur une période `from`/`to`, par projet, par agent, par tâche et par utilisateur, avec le détail agent (projet, tâche) ou modèle (utilisateur).

## État actuel (vérifié dans le code)

- `GET /api/v1/analytics/costs?by=agent|user|model|day` lit du Parquet (DataFusion). Il n'a ni projet ni tâche, ni croisement de dimensions. Il reste inchangé.
- `run_steps` porte `agent` (texte), `model`, `input_tokens`, `output_tokens`, `cost_micros`, `started_at`.
- Run d'agent : `agent_tasks(run_id, card_id, agent_id)` ; le projet vient de `cards.project_id`. Son unique étape `agent_task` est créée sans agent, puis reçoit le slug de l'agent à la fin.
- Run de pipeline (transcript) : `transcripts.project_id`, étapes `extract`/`deduplicate`/`write` avec `agent` = rôle.
- `runs.initiated_by` donne l'utilisateur.
- `budgets` calcule déjà la dépense en SQL sur `run_steps` : précédent pour une lecture Postgres.

## Décisions (mes choix, à corriger)

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| O1 | Source | SQL Postgres sur `run_steps` joint à `runs`, `agent_tasks`, `cards`, `transcripts` | Étendre le Parquet : l'historique n'aurait pas projet ni carte |
| O2 | Période | `started_at` de l'étape dans `[from, to[`. Défaut : 30 jours, `from < to` sinon 422 | `runs.created_at` : un run long déborderait |
| O3 | Tâche | Une carte (`runs.card_id`). Coût = somme de tous ses runs : tâches d'agent, exécutions, interventions | Un run par ligne |
| O4 | Agent d'une étape | Agent de `agent_tasks` si le run est une tâche d'agent ; sinon `run_steps.agent` (rôle : `extractor`, `executor`…). Un agent = une clé `run_steps.agent` ou un agent de la table `agents` | Parser le texte seul |
| O5 | Projet d'un run | `cards.project_id` via `runs.card_id`, sinon `transcripts.project_id`. Aucun projet : ligne « sans projet » | Ignorer ces runs |
| O6 | Accès | Humains seulement, comme `/analytics/costs` (`human_only`) | Ouvrir aux agents |
| O7 | Endroit | Port de lecture dans `core/domain`, adapter Postgres dans `core/infrastructure`, routes dans `handlers-analytics` | Nouveau crate |

## Invariants

1. Un coût est la somme des `cost_micros` des étapes de l'organisation de l'appelant ; jamais d'une autre. [test]
2. `from < to`, sinon 422 ; une étape à `to` exact est exclue. [type : `Period::new` renvoie `Result`]
3. Chaque détail somme au total de sa ligne (les agents d'un projet font le coût du projet). [test]
4. Une étape sans coût compte 0, jamais NULL. [type : colonne `NOT NULL DEFAULT 0`]
5. Un identifiant d'un autre organisation répond 404. [test]

## Routes (lecture seule)

| Route | Contenu |
|---|---|
| `GET /api/v1/analytics/costs/projects?from&to` | Par projet : total, tokens, runs, et la liste des agents intervenus avec leur coût |
| `GET /api/v1/analytics/costs/agents?from&to` | Par agent : total, tokens, runs |
| `GET /api/v1/analytics/costs/tasks?from&to&project_id?` | Par carte : titre, projet, total, et la liste des agents intervenus avec leur coût |
| `GET /api/v1/analytics/costs/users?from&to` | Par utilisateur (`initiated_by`) : total, et la liste des modèles avec leur coût |

Chaque réponse : `{rows, total, from, to}`. Tri par coût décroissant.

## Out of scope

- Le chat avec le Captain (coût cumulé, pas daté).
- Le Parquet et `/analytics/costs` existant.
- La console (page de lecture) : suite possible, hors de cette PR.
- Pagination et export.

## Acceptance

1. Projets : deux projets et un run sans projet donnent trois lignes ; chaque ligne liste ses agents, dont la somme égale le total.
2. Agents : un run d'agent et un run de pipeline apparaissent sous leur agent ou rôle respectif.
3. Tâches : une carte avec deux runs d'agents différents affiche un total égal à la somme et deux agents.
4. Utilisateurs : deux utilisateurs avec deux modèles affichent le détail par modèle.
5. Période : une étape avant `from`, ou à `to` exact, est exclue ; une étape à `from` est incluse.
6. `from >= to` renvoie 422 ; sans paramètres, la fenêtre est de 30 jours.
7. Les données d'une autre organisation n'apparaissent jamais ; `project_id` d'une autre organisation renvoie 404.
8. Un appelant agent reçoit 403.

## Modules

`core` (domain + infrastructure), `handlers-analytics`, `apps/api` (câblage, OpenAPI).

## Files

- `libs/core/src/domain/cost/{mod,ports}.rs` (`Period`, lignes, port `CostReports`)
- `libs/core/src/infrastructure/cost/postgres.rs` (requêtes sqlx, `.sqlx` régénéré)
- `libs/core/tests/cost/` (Postgres)
- `libs/handlers-analytics/src/{projects,tasks,users}.rs` + `costs.rs`/`agents.rs` pour les routes agents
- `apps/api` câblage

## Verify (exit)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p capitainerie-core cost
cargo test -p capitainerie-handlers-analytics
```

## Questions ouvertes

Aucune. Amendement 2026-11-18 : « agents intervenus » = tous les agents des étapes des runs de la ligne (plusieurs runs ou plusieurs étapes). Le code n'a aucune délégation d'un agent à un autre dans un même run (`reports_to` n'est que l'organigramme) : une délégation future produira ses propres étapes avec leur `agent` et leur coût, et apparaîtra ici sans changement de requête.
