# Spec : espaces de travail, chantiers et liste des dépôts GitHub

Statut : réalisée le 2026-11-03, d'un seul tenant. Taille L, une seule PR (4 commits logiques : dépôts, projet courant, chantiers par prompt, chantiers par issue).

## Objectif

1. Travailler sur plusieurs projets sans tout mélanger : la page Chantiers garde un projet courant, retenu d'une visite à l'autre.
2. Lancer plusieurs chantiers à la suite ou en parallèle sur un projet, depuis un prompt libre ou depuis des issues GitHub.
3. Voir les dépôts auxquels l'installation GitHub donne accès, dans Paramètres > Intégrations.

## État actuel (vérifié dans le code)

- Aucun projet courant : `project` n'est qu'un filtre d'URL sur `/kanban` et un paramètre de `/projects/$projectId`.
- Une exécution part d'une carte : `POST /api/v1/cards/{id}/executions` avec `{repository, expected_version}`. Un clic humain choisit le dépôt (invariant 1 de `2026-10-claude-code-executor.md`).
- Limites de parallélisme : une exécution active par carte (index unique), et `EXECUTOR_CONCURRENCY` (2 par défaut) par worker. Rien par projet ni par organisation.
- `GET /api/v1/integrations/github/repositories` existe déjà et renvoie `{owner, name, full_name}` en direct depuis GitHub. La page Intégrations ne l'affiche pas.
- Le client GitHub n'a aucun appel pour les issues, et la GitHub App n'a pas la permission `issues` (manifeste : `contents`, `pull_requests`, `metadata`, `members`).
- `projects.repository` est un texte libre `owner/name`, sans lien avec l'installation.

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| E1 | Espace de travail | **Un projet courant**, choisi en haut de la page Chantiers et mémorisé en local (`capitainerie.current-project`). Pas de nouvelle table. Le Kanban garde son filtre `project` | Sélecteur dans le rail ; table `workspaces` côté serveur |
| E2 | Chantier | **Une carte plus son exécution**. Un prompt libre crée une carte dans le projet, une issue crée une carte liée à l'issue par la table `worksite_issues` | Colonne `source_issue` sur la carte ; nouvelle entité distincte |
| E3 | Lancement multiple | **Un dialogue « Nouveau chantier »**, un seul clic de confiance qui couvre le dépôt choisi et la liste affichée | Un clic par carte |
| E4 | Parallélisme | **Inchangé** : au-delà de `EXECUTOR_CONCURRENCY`, les runs restent `queued`, affichés « en file » | Limite par projet |
| E5 | Issues | **Nouvel appel `list_issues`** et permission `issues: read` dans le manifeste | Coller l'URL d'une issue à la main |
| E6 | Liste des dépôts | **Nom, visibilité, lien**, recherche côté client, lecture seule | Activer ou désactiver des dépôts |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Projet courant | Projet choisi dans le rail, qui filtre Kanban, Runs, En cours et Conversations. | « workspace », « contexte » |
| Chantier | Carte confiée à l'Exécutant, avec le run qui la traite. | « job », « tâche » |
| Chantier d'origine | Prompt libre ou issue GitHub dont la carte est née. | « source » |

## Invariants

1. Seul un humain lance un chantier, et seulement depuis « À faire ». Le dialogue ne s'ouvre que par un clic. [test]
2. Le dépôt d'un chantier doit être dans `github.repositories(...)`, sinon 403 `repository_not_allowed`, comme aujourd'hui. [test]
3. Un lot de N issues crée N cartes et N runs dans **une** transaction. Si l'un échoue, aucun ne part. [test]
4. Une issue ne devient une carte qu'une fois : 409 `issue_already_imported` (index unique sur l'organisation, le dépôt et le numéro). Relancer une issue passe par sa carte. [test]
5. Le projet courant ne donne jamais accès à un projet d'une autre organisation : 404, pas 403. [test]
6. Sans projet courant, la page Chantiers demande d'en choisir un ; le reste de la console est inchangé. [test]
7. La liste des dépôts n'expose ni jeton ni identifiant d'installation. [test]
8. Les issues sont lues avec le jeton d'installation limité au dépôt choisi. [test]

## PR 1 : liste des dépôts dans Intégrations (S)

- API : `GET /api/v1/integrations/github/repositories` ajoute `private: bool` et `html_url`.
- Client : `list_repositories` lit ces deux champs, déjà présents dans la réponse GitHub.
- Console : `github-panel.tsx` affiche la liste (nom, badge privé ou public, lien), un champ de recherche, un état vide « Aucun dépôt : ajoutez-en dans l'installation GitHub ».
- Tests : réponse API, hook, panneau (liste, recherche, vide, erreur).

## PR 2 : projet courant (M)

- Console seulement : sélecteur de projet dans le rail, paramètre `project` commun à `/kanban`, `/runs`, `/live`, `/chat`, mémorisé sous `capitainerie.current-project`.
- Le filtre `project` de Kanban devient le même mécanisme.
- Runs et En cours acceptent `project_id` côté API (à confirmer : le filtre existe-t-il déjà ?).
- Tests : sélecteur, persistance, projet inconnu ou archivé retombe sur « Tous les projets ».

## PR 3 : nouveau chantier depuis un prompt (M)

- API : `POST /api/v1/projects/{id}/yards` (nom à fixer) avec `{prompt, repository, titles?}` crée la carte puis la met en file, dans une transaction.
- Console : bouton « Nouveau chantier » dans le projet courant, dialogue prompt et dépôt, liste des chantiers du projet avec leur état (en file, en cours, terminé, en échec).
- Tests : création, 403 dépôt, 409 déjà actif, état « en file » au-delà de la concurrence.

## PR 4 : chantiers depuis des issues GitHub (M)

- Manifeste : `issues: read`. **Une App déjà créée doit faire accepter la nouvelle permission par son propriétaire GitHub** ; sans cela `list_issues` répond 403 et la console l'explique.
- Client : `list_issues(installation_id, repository)`, ouvertes seulement, sans les pull requests, paginé.
- Cartes : colonne nullable `source_issue` (numéro et URL) avec index unique partiel pour l'invariant 4.
- Console : le dialogue liste les issues ouvertes du dépôt, sélection multiple, un seul lancement.
- Tests : appel GitHub simulé, lot atomique, doublon, permission manquante.

## Questions ouvertes

1. Le nom de la route et du terme « chantier » côté API : `yards`, `workstreams` ou autre ?
2. Le lien issue : `source_issue` sur la carte convient-il, ou vaut-il mieux le déduire de `repository_hint` et d'un champ libre ?
3. La permission `issues: read` oblige à faire réapprouver l'App existante. Acceptable en local, et en production ?

## Écarts avec le brouillon

- Runs, En cours et Conversations ne sont **pas** filtrés par le projet courant : un run de réunion et une conversation ne portent pas de projet. Seuls les runs d'exécution passent par leur carte, et la page Chantiers les montre.
- Les routes s'appellent `GET` et `POST /api/v1/projects/{project_id}/worksites` et `GET /api/v1/integrations/github/issues?repository=owner/name`.
- Une issue absente des issues ouvertes (fermée, ou pull request) refuse tout le lot en 422.
- Au plus 20 chantiers par appel (`MAX_WORKSITES_PER_LAUNCH`).
- Une App GitHub créée avant ce changement n'a pas `issues: read` : l'API répond 422 `github_app_issues_permission_missing` et la console explique quoi cocher sur GitHub. Les prompts continuent de fonctionner.
