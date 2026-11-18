# Spec : page Métriques de la console

Statut : proposée, en attente d'approbation. Deux PR empilées sur la #68 (taille M chacune).

Goal:
  Afficher dans la console les coûts de toute l'organisation sur une période, par projet, agent, tâche et utilisateur, avec un filtre par agent et par tâche.

## État actuel (vérifié dans le code)

- La #68 ajoute `GET /api/v1/analytics/costs/{projects,agents,tasks,users}?from&to`. Seule la vue Tâches accepte `project_id`. Aucun filtre agent ni tâche.
- La console a l'onglet Audit › Coûts (mois en cours, par agent, route Parquet `/analytics/costs`). Il reste inchangé.
- `src/api/api.client.ts` est généré par `pnpm gen:api` depuis l'OpenAPI de l'API qui tourne.
- La barre latérale est `NAV_SECTIONS` (`components/layout/nav-items.ts`) ; les onglets d'une page sont des liens, une URL par onglet (`/audit/{-$tab}`).

## Décisions (mes choix, à corriger)

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| M1 | Filtres | Côté API : `agent_id` et `card_id` sur les quatre routes | Filtrer dans le navigateur : inexact sur Utilisateurs, inutilisable sur Projets pour la tâche |
| M2 | Sens du filtre agent | Étapes des runs d'une tâche d'agent dont l'agent est `agent_id`. Les rôles de pipeline (`extractor`…) n'ont pas d'id : non filtrables | Filtrer sur le nom : non unique |
| M3 | Sens du filtre tâche | Étapes des runs dont `runs.card_id` est la carte | — |
| M4 | Id inconnu ou d'une autre organisation | 404, comme `project_id` | Liste vide : cache une erreur |
| M5 | Entrée de menu | « Métriques », groupe Organisation, route `/metrics/{-$tab}` | Étendre Audit › Coûts |
| M6 | Onglets | Projets, Agents, Tâches, Utilisateurs (un lien chacun) | Un seul tableau |
| M7 | Période | Choix : 7 jours, 30 jours (défaut), mois en cours, dates libres. Envoyée en `from`/`to` UTC | Mois seul |
| M8 | État des filtres et de la période | État local de la page, pas dans l'URL | Query string : suite possible |
| M9 | Accès | `audit.read` : un 403 affiche « pas accès », comme Budgets | — |

## Invariants

1. Un filtre ne change que les étapes retenues ; chaque détail somme encore au total de sa ligne. [test]
2. Les filtres s'appliquent aux quatre routes avec le même sens. [test]
3. Aucune donnée d'une autre organisation ; `agent_id` ou `card_id` étranger renvoie 404. [test]
4. Aucun nombre calculé dans le navigateur à part l'affichage (formats) : les totaux viennent de l'API. [revue]

## Parties

**PR 1, backend** (`core`, `handlers-audit`, `apps/api`) :
- `CostFilter { agent: Option<AgentId>, card: Option<CardId> }` passé aux quatre méthodes du port `CostRepository`.
- Clauses `($n IS NULL OR a.id = $n)` et `($m IS NULL OR r.card_id = $m)` dans les quatre requêtes, `.sqlx` régénéré.
- `agent_exists`, `card_exists` ; 404 dans `CostService`.
- Query `agent_id`, `card_id` sur les quatre routes ; OpenAPI ; tests Postgres et API ; `docs/ARCHITECTURE.md`.

**PR 2, console** (`apps/console`) :
- `pnpm gen:api` contre l'API de la PR 1.
- `domain/metrics/` : hooks de requête, modèle (période, formats), UI (sélecteur de période, filtres agent et tâche, tableau avec lignes dépliables, tuiles de total), page.
- Route `/metrics/{-$tab}`, entrée de menu, libellé de fil d'Ariane, palette de commandes.
- Filtre agent : liste des agents de l'équipe (`useTeamAgents`). Filtre tâche : liste des cartes de l'organisation, avec recherche.
- Tests Vitest sur le modèle et la page.

## Out of scope

- Chat avec le Captain, délégation entre agents (projet séparé).
- Filtre par projet dans la console (l'API le permet déjà sur Tâches ; suite possible).
- Filtre par utilisateur, export, pagination, graphiques.
- Audit › Coûts reste tel quel.

## Acceptance

1. `agent_id` restreint les quatre rapports aux runs de cet agent ; les détails somment aux totaux.
2. `card_id` restreint les quatre rapports aux runs de cette carte.
3. `agent_id` ou `card_id` d'une autre organisation renvoie 404 ; les deux filtres ensemble se cumulent.
4. La console affiche « Métriques » sous Organisation ; chaque onglet charge son rapport sur la période choisie.
5. Changer d'agent, de tâche ou de période recharge le rapport ; les tuiles montrent le total de l'API.
6. Une ligne se déplie : agents d'un projet ou d'une tâche, modèles d'un utilisateur.
7. Un 403 affiche un message d'accès ; un rapport vide affiche un état vide.

## Verify (exit)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p capitainerie-core cost
cargo test -p capitainerie-handlers-audit
pnpm --dir apps/console check
pnpm --dir apps/console test
pnpm --dir apps/console build
```

## Questions ouvertes

Aucune.
