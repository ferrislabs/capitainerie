# Spec : l'Exécutant, Claude Code sur les cartes validées

Statut : brouillon, décisions validées le 2026-10-18 (tableau ci-dessous). Taille L.

## Objectif

Sur une carte validée par un humain, un membre clique « Confier à Claude Code » : Claude Code traite la carte sur un dépôt GitHub de l'organisation, dans un conteneur jetable, pousse une branche, et le worker ouvre une pull request en brouillon liée à la carte.

## Contexte

- Le pipeline actuel s'arrête à la proposition de cartes. `docs/ARCHITECTURE.md` §2 prévoit déjà des « agents d'itération » : un run dont le pipeline n'a qu'une étape, déclenché par un événement de carte. L'Exécutant en est le premier.
- Claude Code est un agent de code (outils fichiers, shell, sous-agents). Il apporte ce que la boucle Rig n'a pas : agir sur un dépôt.
- **L'Exécutant n'écrit que par le code du worker.** Claude Code ne voit ni jeton utilisateur ni jeton GitHub ni clé : il lit et modifie un clone, et le worker, hors du conteneur, pousse la branche (via le proxy), ouvre la PR et commente la carte (décision X8). Aucun client FerrisKey n'existe pour lui.
- **Authentification : clé API uniquement.** Anthropic interdit aux produits tiers, sauf accord préalable, de proposer la connexion claude.ai ou les limites d'un abonnement (doc de l'Agent SDK : « Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products »). L'Exécutant utilise la clé Anthropic de l'organisation (Paramètres › Fournisseurs).

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| X1 | Rôle de Claude Code | **Exécutant des cartes validées** | Moteur du Rédacteur |
| X2 | Où il tourne | **Un Job Kubernetes par tâche**, conteneur jetable | Sous-processus du worker |
| X3 | Accès au code | **GitHub App par organisation, créée depuis la console (manifeste GitHub)**, jeton d'installation court limité à un dépôt | App unique configurée par l'opérateur (variables d'environnement) ; jeton personnel d'un admin |
| X4 | Secrets dans le conteneur | **Proxy d'identifiants** : le conteneur ne reçoit qu'un jeton de tâche sans valeur ailleurs ; le proxy le remplace par la vraie clé Anthropic ou le jeton GitHub, et filtre les requêtes | Secret Kubernetes lisible par l'agent |
| X5 | Suivi et résultat | **Lecture des logs du pod** (`--output-format stream-json`) | Rappel HTTP depuis le conteneur |
| X6 | Exécuteur en local | **Kubernetes partout** : un cluster kind à côté du compose | Docker en local, Kubernetes en prod |
| X7 | Pull request | **Ouverte par le worker, en brouillon**, avec la GitHub App | Branche seulement |
| X8 | Écriture sur la carte | **Le worker écrit lui-même le commentaire final**, au nom de l'humain qui a confié la carte avec l'Exécutant comme acteur (`actor`), par un use case du cœur ; aucun jeton délégué, aucun passage par le serveur MCP | Jeton de l'utilisateur échangé pour un client `capitainerie-agent-executor`, appel du MCP |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Exécutant | Agent qui traite une carte validée sur un dépôt (Claude Code). Il n'a ni client FerrisKey ni jeton : `capitainerie-agent-executor` n'est que l'`actor` des décisions PDP et des commentaires que le worker écrit pour lui. | « agent de code », « worker » |
| Exécution | Run d'un seul step `Execute`, rattaché à une carte et à un dépôt. | « tâche », « job » (le Job est l'objet Kubernetes) |
| Proxy d'identifiants | Service `apps/egress-proxy`, seul point de sortie réseau du conteneur. | « gateway » |
| Jeton de tâche | Jeton aléatoire propre à une exécution, que seul le proxy accepte. | « clé » |

## Invariants

1. Seule une carte en « À faire » peut être confiée, et seulement par un humain. [type : `ExecutableCard::try_from(Card)` ; test PDP : `card.execute` refusé à un acteur agent]
2. Une carte a au plus une exécution active. [index unique partiel en base + test]
3. Le conteneur ne reçoit jamais la clé Anthropic de l'organisation, un jeton GitHub ni un jeton de l'utilisateur. [test : la spec du Job ne contient que `CAPITAINERIE_TASK_TOKEN` comme secret ; aucune ligne `run_delegations` pour une exécution]
4. Le proxy n'accepte un jeton de tâche que pour une exécution `running` et non expirée ; il est révoqué à la fin de l'exécution, quel que soit le résultat. [test]
5. Côté Anthropic, le proxy ne laisse passer que `POST /v1/messages` et `POST /v1/messages/count_tokens`, avec le modèle de l'exécution (`task_grants.model`), `max_tokens` ≤ 128 000, un corps JSON ≤ 10 MiB, sans `mcp_servers` ni `container`, avec des outils côté client seulement (aucun `web_search_*`, `web_fetch_*`, `code_execution_*`, `bash_*`, `computer_*`, `text_editor_*`, `mcp_toolset`), les seuls en-têtes `anthropic-beta` de la liste fermée (`prompt-caching-*`, `interleaved-thinking-*`, `fine-grained-tool-streaming-*`, `context-management-*`, `token-efficient-tools-*`) et la seule requête `beta=true`. Les octets d'origine sont transmis tels quels après validation. [tests]
6. Côté GitHub, le proxy ne laisse passer que la lecture du dépôt de l'exécution et un push sur sa branche `capitainerie/<12 derniers hex de l'id de l'exécution>` (la queue aléatoire d'un UUIDv7 : ses 8 premiers hex sont un horodatage, deux exécutions rapprochées partageraient la branche) : ni autre dépôt, ni autre branche, ni suppression de ref. [test sur les commandes `git-receive-pack`]
7. L'agent ne déplace jamais la carte. C'est le clic humain qui la passe en « En cours », dans la même transaction que la création de l'exécution. [test]
8. Le modèle de l'Exécutant est un modèle Anthropic. [type : `ExecutorModel(String)`, dont le constructeur refuse tout nom qui ne commence pas par `claude-` (la désérialisation aussi) ; le PUT des assignations répond 422]
9. Une exécution est bornée : durée (`activeDeadlineSeconds`, 30 min par défaut) et nombre de tours (`--max-turns`, 50 par défaut). [test sur la spec du Job]
10. Aucune clé ni aucun jeton dans les logs, les traces, les événements ou la timeline. [test : les sorties `stream-json` sont filtrées avant stockage]
11. Une installation GitHub n'est liée à une organisation que si l'admin qui la connecte y a accès (OAuth GitHub) **et en est propriétaire** : admin actif de l'organisation sur laquelle l'App est installée (`GET /user/memberships/orgs/{login}`), ou utilisateur dont le login est celui du compte (`GET /user`) ; une installation n'est liée qu'à une seule organisation. [tests]
12. Les secrets de la GitHub App d'une organisation (secret OAuth, clé privée, secret de webhook) ne sont jamais renvoyés par l'API ni journalisés ; en base ils sont scellés avec `SECRETS_KEY` (AAD `<id de l'organisation>/github_app/<champ>`), et un `state` signé pour le manifeste n'est pas accepté pour une installation, ni l'inverse. [tests : colonnes scellées, schéma OpenAPI sans secret, `state` de mauvais usage → 400]

## Hors périmètre

- La connexion par abonnement claude.ai (interdite, voir Contexte).
- GitLab, Bitbucket ; plusieurs dépôts pour une carte.
- La fusion de la PR ; la relance automatique sur commentaire de revue.
- Le pilotage en cours d'exécution (« steer ») ; le mode interactif.
- Un plafond de coût en tokens appliqué par le proxy (les bornes durée et tours suffisent pour le MVP).
- Bedrock et Vertex comme fournisseurs de l'Exécutant.
- L'export OTel de Claude Code lui-même : le worker trace l'exécution de l'extérieur.

## Conception

### Domaine (`libs/core`)

- `Run` porte un `subject: RunSubject` au lieu de `transcript_id` :
  ```rust
  pub enum RunSubject { Transcript(TranscriptId), Card { card_id: CardId, repository: RepositoryRef } }
  ```
  Migration : `runs.transcript_id` devient nullable, ajout de `card_id`, `repository` (`owner/name`), `base_branch`, avec `CHECK` « exactement un sujet ».
- `StepKind::Execute`.
- `ExecutableCard` : une `Card` en `Todo`, construite par `TryFrom<Card>` (invariant 1).
- `ExecutorModel(String)` dans `ModelAssignments`, champ `executor`, défaut `claude-sonnet-5-5` (invariant 8). Les assignations stockées sans ce champ prennent le défaut.
- `GithubInstallation { organization_id, installation_id, account_login, created_by, created_at }`.
- `GithubAppCredentials { organization_id, app_id, slug, client_id, client_secret, private_key_pem, webhook_secret, html_url, owner_login, owner_type, created_by, created_at }` (les trois secrets sont des `SecretToken`) et `GithubAppSummary` (sans secret). Une App par organisation (table `github_apps`, clé primaire `organization_id`, `app_id` unique).
- Actions PDP : `CardExecute` (humain, membre, carte en « À faire »), `GithubManage` (admin). L'acteur `capitainerie-agent-executor` garde les droits des agents sur les cartes (commenter), rien de plus.
- Use cases :
  - `check_card_executable(subject, card_id)` : lecture seule (PDP `card.execute`, carte, état, exécution active), pour qu'un handler refuse avant tout appel à GitHub.
  - `execute_card(subject, card_id, repository, base_branch, expected_version)` : revérifie tout (invariant 1, PDP), puis dans une transaction : carte → « En cours », `Run(Queued, [Execute], Card{..})`, événement `RunQueued`. Aucun échange de jeton, aucune délégation.
  - `comment_card_as_executor(run_id, card_id, body)` (méthode système du worker) : auteur `{ subject: run.initiated_by, actor: Some(EXECUTOR_AGENT) }`, décision PDP `card.comment` pour ce sujet et cet acteur, auditée avec l'id du run (les agents peuvent commenter). Idempotent par run : l'id du commentaire est écrit dans la sortie du step Execute dans la même transaction, et un second appel le renvoie sans rien poster.
  - `authorize(subject, action, kind, id)` : décision PDP auditée, pour qu'un handler refuse avant tout autre travail.
  - `connect_github(subject, installation_id, account_login)` et `disconnect_github(subject)`.
  - `save_github_app(subject, credentials)` (admin, PDP `github.manage`, audité ; 409 `github_app_exists`), `github_app(subject)` (lecture membre, sans secret), `delete_github_app(subject)` (admin ; supprime aussi l'installation) et `github_app_for(organization_id)` / `github_app_summary_for(organization_id)` (méthodes système, sans décision PDP, pour l'API, le worker et le proxy).

### Ports

| Port | Module | Dispatch | Rôle |
|---|---|---|---|
| `TaskRunner` | `core/domain/execution/ports.rs` | générique | lancer un Job (`start`, run neuf), savoir s'il existe encore (`attach`, run repris), suivre son flux de sortie ligne à ligne, connaître son issue, le supprimer |
| `CodeHost` | `core/domain/execution/ports.rs` | générique | lister les dépôts d'une installation (chaque méthode prend l'organisation : l'App est celle de l'organisation), lire la tête d'une branche (`branch_head`), ouvrir une PR en brouillon (idempotent : si une PR ouverte existe pour la branche, son URL est renvoyée) |
| `TaskGrantRepository` | `core/domain/execution/ports.rs` | générique | créer, révoquer, retrouver un jeton de tâche (haché) |
| `GithubInstallationRepository` | `core/domain/execution/ports.rs` | générique | une installation par organisation |
| `GithubAppRepository` | `core/domain/execution/ports.rs` | générique | une App par organisation, secrets scellés (`get`, `summary`, `insert`, `delete`) |

### Adapters

- `libs/runner-kube` : `TaskRunner` avec `kube` + `k8s-openapi` (création du Job, suivi des logs du pod, statut). Namespace dédié `capitainerie-tasks`, `ttlSecondsAfterFinished: 1800` (au-delà des 15 min après lesquelles un run abandonné est repris), `backoffLimit: 0`, pod non root, système de fichiers racine en lecture seule sauf `/workspace` (4 Gi), `/tmp` (1 Gi) et `/home/executor` (1 Gi) en emptyDir bornés, limite `ephemeral-storage` de 6 Gi, aucun ServiceAccount monté. Le pod ne résout aucun nom : le worker résout l'hôte du proxy à la création du Job (`hostAliases`), `dnsPolicy: None` avec `127.0.0.1` pour seul serveur. Les lignes de log dépassant 1 MiB sont ignorées (avertissement) et un step garde au plus 500 appels d'outils (le nombre d'appels écartés est conservé dans `dropped_tool_calls`).
- `libs/github` : `CodeHost` en HTTP (`reqwest`), liens `Link` suivis seulement vers le même schéma, hôte et port que l'API ; JWT de la GitHub App en RS256 (`jsonwebtoken`, déjà présent), résolveur `GithubApps` (organisation → `GithubClient`, caches de jetons conservés par App, entrée valable 5 min : une App supprimée est constatée au prochain échec de cache), jetons d'installation limités à un dépôt (`repositories: [name]`, permissions `contents: write`, `pull_requests: write`).
- `libs/core/infrastructure` : repositories Postgres (`github_installations`, `task_grants` avec `token_sha256`, `run_id`, `model`, `expires_at`, `revoked_at`).
- `apps/egress-proxy` : axum.
  - `/anthropic/*` : vérifie `x-api-key` = jeton de tâche, valide le corps (invariant 5), remplace le jeton par la clé de l'organisation (`provider_access`, déchiffrée avec `SECRETS_KEY`), transmet à `https://api.anthropic.com` et relaie la réponse en streaming.
  - `/git/<owner>/<repo>.git/*` : vérifie le mot de passe Basic = jeton de tâche et le dépôt de l'exécution, lit les commandes `pkt-line` d'un `git-receive-pack` pour l'invariant 6, puis transmet à `github.com` avec un jeton d'installation.
  - Tout le reste : 403.
  - Une grant lue est gardée 10 s en cache : une révocation peut mettre jusqu'à 10 s à s'appliquer. Un jeton inconnu ou révoqué est refusé 10 s sans relire la base. Au plus 16 requêtes simultanées par exécution (429 au-delà).
  - Moindre privilège : le proxy ne lit que `task_grants`, `provider_credentials`, `github_installations` et `github_apps` (méthodes système du cœur, aucune décision auditée, aucune écriture). `deploy/postgres/egress-proxy-role.sql` crée le rôle `capitainerie_egress_proxy` en `SELECT` seul ; le chart accepte sa `DATABASE_URL` via `egressProxy.databaseUrlSecretKey`.
- `NetworkPolicy` du namespace des tâches : sortie autorisée vers le proxy seulement (plus de DNS : voir `hostAliases`). L'IP du Service du proxy doit rester stable (Service `ClusterIP`).

### Image `capitainerie-executor`

- Base `node:22-slim`, `git`, Claude Code en version figée (paquet npm `@anthropic-ai/claude-code@<version>`), script d'entrée `deploy/executor/entrypoint.sh`.
- Le script : clone par le proxy, crée la branche, lance `claude -p "$PROMPT" --output-format stream-json --verbose --model "$MODEL" --max-turns "$MAX_TURNS" --allowedTools "Read,Edit,Write,Glob,Grep,Bash"`, puis commite et pousse s'il y a des changements. Il émet des lignes `{"type":"capitainerie",...}` (cloné, poussé avec le SHA, aucun changement) que le worker distingue des lignes de Claude Code.
- Variables : `ANTHROPIC_BASE_URL` (le proxy), `ANTHROPIC_API_KEY` et le mot de passe git = jeton de tâche, `PROMPT`, `MODEL`, `MAX_TURNS`, `REPOSITORY`, `BRANCH`.

### Worker

- `ExecutePipeline` (à côté de `MeetingPipeline`) : réclame le run, crée le jeton de tâche, lance le Job, lit le flux :
  - chaque `tool_use` / `tool_result` devient un `ToolCall` de l'étape (sortie tronquée et filtrée, invariant 10) ;
  - le message `result` final donne l'usage (tokens), le coût calculé avec la table de prix, le résumé.
- Coût : avec un prix connu pour le modèle, les tokens du flux sont facturés par nature (lecture de cache à 10 % du prix d'entrée, écriture de cache à 125 %, le reste au prix d'entrée, sortie au prix de sortie) ; sans prix connu, c'est le `total_cost_usd` du flux, converti en micros. Les colonnes de tokens gardent le total d'entrée et la sortie.
- Les tokens et le coût viennent du flux de Claude Code, qui tourne dans le conteneur : ils sont indicatifs, pas une preuve de facturation (le proxy ne les mesure pas).
- À la fin, quel que soit le résultat : révocation du jeton de tâche (la suppression de la délégation du run ne concerne plus que le jeton de tâche, stocké chiffré). En cas d'échec après le lancement du Job, le Job est supprimé après la révocation ; en cas de succès il reste pour ses logs (le TTL le nettoie).
- Si une branche a été poussée : le worker vérifie auprès de GitHub que `refs/heads/<branche>` existe et pointe vers le SHA annoncé (sinon l'exécution échoue « branche absente ou différente »), puis ouvre la PR en brouillon (titre = titre de la carte, corps = lien vers la carte, résumé, coût). L'URL de la PR est écrite dans le step comme erreur dure et reportée sur chaque tentative de reprise : une reprise n'ouvre jamais une seconde PR. Puis le worker poste lui-même le commentaire final (`comment_card_as_executor`) : lien de la PR et résumé. Sinon : commentaire « aucun changement » ou raison de l'échec. Un marqueur « aucun changement » alors que le flux dit que l'agent s'est arrêté (`error_max_turns`, erreur) fait échouer l'exécution (« arrêté : nombre maximal de tours atteint »). Le texte écrit par l'agent (corps de la PR, commentaire) a ses `@` suivis d'un espace de largeur nulle pour ne notifier personne.
- Reprise : un run `Execute` repris se rattache au Job existant (`TaskRunner::attach`, nom du Job = id du run) ; s'il n'existe plus, l'exécution échoue « tâche introuvable à la reprise » sans lancer de second Job. Seul un run dont le step avait noté `job_started: false` (planté avant le lancement) peut le lancer. Une erreur d'infrastructure (`Database`, `Internal`) pendant la préparation ou l'écriture du step ne fait pas échouer le run : il reste en cours et le balayage le reprend.
- Concurrence : `EXECUTOR_CONCURRENCY` (2 par défaut) borne les exécutions, à part de `WORKER_CONCURRENCY` ; un id de run déjà en cours dans ce worker n'est jamais relancé.

### API et console

| Méthode | Chemin | Rôle requis |
|---|---|---|
| `POST` | `/api/v1/cards/{id}/executions` (`repository`, `expected_version`) | membre |
| `GET` | `/api/v1/integrations/github` (`{ app, installation }`, chacun `null` tant qu'il n'existe pas) | membre |
| `POST` | `/api/v1/integrations/github/app/manifest` (`owner`: `user` ou `organization` + `login`) → `{ action_url, manifest, state }` | admin |
| `POST` | `/api/v1/integrations/github/app` (`code`, `state`) : échange le code du manifeste contre l'App et la stocke scellée | admin |
| `DELETE` | `/api/v1/integrations/github/app` (oublie l'App et l'installation ; l'App reste sur GitHub, `html_url` renvoyé) | admin |
| `GET` | `/api/v1/integrations/github/install` (URL d'installation de l'App de l'organisation, `state` signé) | admin |
| `POST` | `/api/v1/integrations/github/callback` (`installation_id`, `code`, `state`) | admin |
| `DELETE` | `/api/v1/integrations/github` (l'installation seulement) | admin |
| `GET` | `/api/v1/integrations/github/repositories` | membre |

- Création de l'App (manifeste GitHub) : l'API prépare le manifeste (nom `Capitainerie <organisation> <6 hex>` borné à 34 caractères, permissions `contents: write`, `pull_requests: write`, `metadata: read`, aucun événement ni webhook (GitHub refuse une adresse de webhook injoignable depuis Internet, même inactive), URLs sur `CONSOLE_URL`) et un `state` signé (HMAC dérivé de `SECRETS_KEY`, organisation, sujet, 10 min, usage « manifest »). La console POSTe le formulaire à `action_url` (GitHub exige un POST du navigateur) ; GitHub redirige vers `/settings/integrations/github/app-callback?code=…&state=…` ; la console envoie `{ code, state }`, l'API vérifie le `state` puis appelle `POST <api>/app-manifests/{code}/conversions`. Sans `CONSOLE_URL` l'API répond 503 `console_url_not_configured`. Le code ne servant qu'une fois, l'existence d'une App est vérifiée avant l'échange.
- Console : bouton « Confier à Claude Code » sur une carte « À faire » (dialogue : choix du dépôt, coût estimé non affiché, avertissement « l'agent pousse une branche et ouvre une PR en brouillon ») ; page Paramètres › Intégrations › GitHub ; la timeline d'un run `Execute` montre les appels d'outils et le lien de la PR.

### Erreurs

- `CoreError` existant, codes stables dans `Conflict` (voir Contrats figés).
- `runner-kube` et `github` : `thiserror`, convertis en `CoreError::Internal` (transitoire, réessayé) ou `CoreError::Invalid`.

## Acceptance (Gherkin)

```gherkin
@spec-exec-1  Scenario: un membre confie une carte validée
  Given une organisation avec une clé Anthropic et une installation GitHub sur "acme/web"
  And une carte en « À faire »
  When un membre la confie à Claude Code sur "acme/web"
  Then la carte passe en « En cours »
  And un run Execute est en file, rattaché à la carte et au dépôt

@spec-exec-2  Scenario: une carte proposée ne peut pas être confiée
  Given une carte en « Proposé »
  When un membre la confie
  Then la réponse est 409 card_not_executable et rien n'est créé

@spec-exec-3  Scenario: une seule exécution active par carte
  Given une carte avec une exécution en cours
  When un membre la confie de nouveau
  Then la réponse est 409 execution_already_active

@spec-exec-4  Scenario: le conteneur ne voit aucun vrai secret
  When le worker lance le Job d'une exécution
  Then la spec du pod ne contient ni la clé Anthropic ni un jeton GitHub
  And sa seule sortie réseau autorisée est le proxy

@spec-exec-5  Scenario: le proxy borne l'accès GitHub
  Given le jeton d'une exécution sur "acme/web", branche "capitainerie/0a1b2c3d4e5f"
  When le conteneur pousse sur "main", ou lit "acme/autre"
  Then le proxy répond 403
  And un push sur "capitainerie/0a1b2c3d4e5f" passe

@spec-exec-6  Scenario: le jeton est révoqué à la fin
  Given une exécution terminée, réussie ou non
  When le jeton de tâche est présenté au proxy
  Then la réponse est 401

@spec-exec-7  Scenario: exécution réussie de bout en bout (cluster kind)
  Given une carte « Ajouter un fichier CHANGELOG.md » confiée sur un dépôt de test
  When l'exécution se termine
  Then une branche capitainerie/… existe avec un commit
  And une PR en brouillon pointe vers la carte
  And la carte a un commentaire de l'Exécutant avec le lien de la PR
  And la timeline montre les appels d'outils, les tokens et le coût

@spec-exec-8  Scenario: exécution sans changement
  When Claude Code termine sans modifier le dépôt
  Then aucune branche ni PR n'est créée et la carte reçoit « aucun changement »

@spec-exec-9  Scenario: exécution trop longue
  When l'exécution dépasse sa durée maximale
  Then le Job est arrêté, le run échoue avec « durée maximale dépassée » et le jeton est révoqué

@spec-exec-10 Scenario: seul un admin connecte GitHub
  When un membre non admin appelle POST /api/v1/integrations/github/callback
  Then la réponse est 403 et la décision est auditée

@spec-exec-12 Scenario: reprise sans Job
  Given une exécution reprise dont le Job n'existe plus
  Then elle échoue « tâche introuvable à la reprise » et aucun Job n'est lancé

@spec-exec-13 Scenario: le proxy borne le modèle et les outils
  When le conteneur demande un autre modèle, plus de 128 000 tokens, ou un outil serveur (web_search, code_execution…)
  Then le proxy répond 403 et rien n'est transmis à Anthropic

@spec-exec-14 Scenario: une installation d'une organisation que l'admin n'administre pas est refusée
  When un admin dont le rôle GitHub dans l'organisation propriétaire n'est pas « admin actif » appelle le callback
  Then la réponse est 403 `installation_not_owned` et rien n'est stocké

@spec-exec-11 Scenario: une installation GitHub inaccessible ou déjà liée est refusée
  Given un admin dont le code OAuth GitHub ne donne pas accès à l'installation
  When il appelle POST /api/v1/integrations/github/callback avec { installation_id, code, state }
  Then la réponse est 403 `installation_not_accessible` et rien n'est stocké
  And une installation déjà liée à une autre organisation répond 409 `installation_already_connected`

@spec-exec-15 Scenario: une organisation crée sa GitHub App depuis la console
  Given un admin sans App pour son organisation
  When il demande le manifeste, valide la création sur GitHub, et que la console envoie { code, state } à POST /api/v1/integrations/github/app
  Then l'App est stockée, ses secrets scellés, et la réponse ne contient aucun secret
  And l'URL d'installation utilise le slug de cette App
  And un `state` de manifeste refusé pour une installation (et l'inverse) répond 400 `invalid_state`
  And une seconde App répond 409 `github_app_exists` sans consommer le code

@spec-exec-16 Scenario: supprimer l'App supprime l'installation
  Given une organisation avec une App et une installation
  When l'admin appelle DELETE /api/v1/integrations/github/app
  Then l'App et l'installation sont oubliées et la réponse porte le `html_url` de l'App
  And une exécution lancée ensuite échoue « application GitHub absente »
```

## Simulation

Non : l'exécution est une suite séquentielle supervisée par un seul worker, réclamée en base comme un run actuel. La reprise (rattachement au Job existant) est couverte par un test d'intégration.

## Workstreams

| # | Mission | Dépend de | Fichiers |
|---|---|---|---|
| W1 | Domaine, migrations, PDP, use cases, repositories | — | `libs/core/**/execution/**`, `run/**`, `authorization/**`, `migrations/2026102*` |
| W2 | GitHub App : `CodeHost`, endpoints d'intégration | W1 (contrats) | `libs/github/**`, `libs/handlers-integrations/**` |
| W3 | Proxy d'identifiants | W1 (contrats) | `apps/egress-proxy/**` |
| W4 | `runner-kube`, image, chart, kind local | W1 (contrats) | `libs/runner-kube/**`, `deploy/executor/**`, `deploy/helm/**`, `deploy/kind/**`, `docker-compose.yml` |
| W5 | `ExecutePipeline` et lecture du flux `stream-json` | W1, W4 | `apps/worker/src/execute/**`, `libs/core/src/application/execute*.rs` |
| W6 | Console : bouton, dialogue, Intégrations › GitHub, timeline | W2 (API) | `apps/console/src/domain/{kanban,integrations,runs}/**` |

Ordre : W1 d'abord (contrats figés), puis W2, W3, W4 en parallèle, puis W5, puis W6.

## Contrats figés

Dans le code : `libs/core/src/domain/execution/mod.rs` (types) et `ports.rs` (ports `TaskRunner`, `CodeHost`, `TaskGrantRepository`, `GithubInstallationRepository`, `GithubAppRepository`). Ces fichiers font foi.

- `TaskRunner::follow(run_id, mpsc::Sender<TaskLine>) -> TaskOutcome` : le flux passe par un canal `tokio::mpsc`, car `core` ne dépend pas de `futures`.
- Erreur transitoire (réessayée) : `CoreError::Internal`, comme pour les étapes actuelles.
- Les erreurs de domaine sont des `CoreError::Conflict` avec un code stable : `card_not_executable`, `execution_already_active`, `github_not_connected` ; `RepositoryNotAllowed` est un `CoreError::Forbidden`.

## Dépendances ajoutées

| Crate | Pourquoi | Alternative écartée |
|---|---|---|
| `kube`, `k8s-openapi` | client Kubernetes (Jobs, logs, statut) ; référence en Rust | appels HTTP à la main sur l'API Kubernetes : authentification et watch à réécrire |
| `futures-util` (déjà présent) | flux des lignes de logs | — |

Aucune autre : `reqwest`, `jsonwebtoken`, `axum`, `aws-lc-rs` sont déjà dans le workspace.

## Risques et questions ouvertes

| Risque | Porteur |
|---|---|
| kind n'applique pas les `NetworkPolicy` avec son CNI par défaut : en local, l'isolation réseau n'est pas vérifiée. À vérifier en CI sur un cluster avec Calico, ou documenter la limite. | W4 |
| Le format `stream-json` de Claude Code peut évoluer : la version est figée dans l'image, et un test de parsing tourne sur un enregistrement réel. | W5 |
| Le prompt de l'Exécutant (titre, description, citation de la carte) est une entrée utilisateur : injection possible. Atténuation : isolation, proxy (modèle, outils et en-têtes bornés), aucun secret, PR en brouillon relue par un humain. | W5 |
| GitHub Actions : une branche `capitainerie/*` poussée par l'Exécutant peut déclencher des workflows qui disposent de secrets. Risque résiduel, non traité par Capitainerie : restreindre les workflows à secrets aux branches protégées ou aux environnements à relecteurs. L'avertissement de la console le mentionne (W6) ; voir le README « Exécutant ». | exploitant du dépôt |
| La révocation d'un jeton de tâche peut mettre jusqu'à 10 s à s'appliquer (cache de grant du proxy). | W3 |
| Coût d'une exécution difficile à prévoir : affiché après coup dans la timeline et la page Agents ; un plafond par organisation viendra si le besoin apparaît. | produit |
| ~~Le jeton délégué de l'utilisateur est borné par sa session~~ : sans objet depuis X8. L'Exécutant ne tient aucun jeton ; il n'écrit que par le code du worker, qui commente sous l'identité de l'humain avec l'acteur `capitainerie-agent-executor`, et la règle PDP « les agents peuvent commenter » reste la seule permission qu'il a. | W5 |
| Une App créée sur GitHub puis oubliée côté Capitainerie (suppression, ou échec du stockage après l'échange du code) reste sur GitHub : l'interface renvoie vers sa page pour la supprimer. | W6 |
| La suppression de l'App oublie aussi l'installation ; les exécutions en cours gardent leur jeton de tâche, mais le proxy n'obtient plus de jeton GitHub (cache d'au plus 5 min). | W3 |

## Vérification (sortie)

```bash
cargo fmt --all --check
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://capitainerie:capitainerie@localhost:5442/capitainerie_test \
  REQUIRE_DATABASE_URL=1 cargo test --workspace -- --test-threads=1
kind export kubeconfig --name capitainerie
REQUIRE_KUBE=1 cargo test -p capitainerie-runner-kube -p capitainerie-worker --test execute -- --test-threads=1
cd apps/console && corepack pnpm check && corepack pnpm test && corepack pnpm build
```

Plus un run réel (`@spec-exec-7`) sur un dépôt de test de l'organisation, avec la clé Anthropic de l'utilisateur et son accord sur le coût.

Amended: 2026-10-18 contrats figés dans le code ; suivi par canal mpsc ; erreurs sur CoreError existant.
Amended: 2026-10-19 l'installation GitHub est liée par OAuth (invariant 11), unique par organisation ; MCP_ACCEPTED_AGENTS inclut l'Exécutant du chart.
Amended: 2026-10-20 la branche est `capitainerie/<12 derniers hex>` (queue aléatoire de l'UUIDv7) ; `TaskRunner::attach` : une reprise sans Job échoue au lieu d'en relancer un ; vérification de la branche avant la PR ; coût par nature de token (cache) ; `ExecutorModel` refuse un modèle non `claude-`.
Amended: 2026-10-20 X8 : le worker écrit le commentaire final lui-même (`comment_card_as_executor`, idempotent) ; plus de délégation ni de client `capitainerie-agent-executor` (MCP, FerrisKey, chart) ; `execute_card` ne prend plus de jeton ; contrôle `check_card_executable` avant tout appel GitHub.
Amended: 2026-10-20 le proxy valide le corps Anthropic (modèle de la grant, `max_tokens`, outils, en-têtes `anthropic-beta`, requête), cache négatif de 10 s, 16 requêtes simultanées par grant, rôle Postgres en lecture seule ; le pod ne résout aucun nom (`hostAliases`, `dnsPolicy: None`) ; la propriété de l'installation GitHub est vérifiée (invariant 11) ; décalage de révocation de 10 s documenté ; risque GitHub Actions ajouté.
Amended: 2026-10-21 X3 : chaque organisation crée sa GitHub App depuis la console (manifeste GitHub) ; plus de variable `GITHUB_APP_*` côté opérateur, seulement `GITHUB_API_BASE`, `GITHUB_WEB_BASE` et `CONSOLE_URL` ; table `github_apps` aux secrets scellés (invariant 12) ; `CodeHost` prend l'organisation ; le proxy lit aussi `github_apps`.
Amended: 2026-10-19 le manifest ne déclare plus de webhook : GitHub refusait `localhost`.
Amended: 2026-10-19 l'App demande `members: read` (organisation) : sans elle, GitHub refuse de dire si l'admin qui connecte administre l'organisation (403 `github_app_members_permission_missing`).
Amended: 2026-10-19 plafond `max_tokens` du proxy porté à 128 000 : Claude Code en demande davantage que 32 000 ; le run reste borné par ses tours et sa durée.
