# Spec : l'Exécutant, Claude Code sur les cartes validées

Statut : brouillon, décisions validées le 2026-10-18 (tableau ci-dessous). Taille L.

## Objectif

Sur une carte validée par un humain, un membre clique « Confier à Claude Code » : Claude Code traite la carte sur un dépôt GitHub de l'organisation, dans un conteneur jetable, pousse une branche, et le worker ouvre une pull request en brouillon liée à la carte.

## Contexte

- Le pipeline actuel s'arrête à la proposition de cartes. `docs/ARCHITECTURE.md` §2 prévoit déjà des « agents d'itération » : un run dont le pipeline n'a qu'une étape, déclenché par un événement de carte. L'Exécutant en est le premier.
- Claude Code est un agent de code (outils fichiers, shell, sous-agents). Il apporte ce que la boucle Rig n'a pas : agir sur un dépôt.
- **Authentification : clé API uniquement.** Anthropic interdit aux produits tiers, sauf accord préalable, de proposer la connexion claude.ai ou les limites d'un abonnement (doc de l'Agent SDK : « Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products »). L'Exécutant utilise la clé Anthropic de l'organisation (Paramètres › Fournisseurs).

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| X1 | Rôle de Claude Code | **Exécutant des cartes validées** | Moteur du Rédacteur |
| X2 | Où il tourne | **Un Job Kubernetes par tâche**, conteneur jetable | Sous-processus du worker |
| X3 | Accès au code | **GitHub App par organisation**, jeton d'installation court limité à un dépôt | Jeton personnel d'un admin |
| X4 | Secrets dans le conteneur | **Proxy d'identifiants** : le conteneur ne reçoit qu'un jeton de tâche sans valeur ailleurs ; le proxy le remplace par la vraie clé Anthropic ou le jeton GitHub, et filtre les requêtes | Secret Kubernetes lisible par l'agent |
| X5 | Suivi et résultat | **Lecture des logs du pod** (`--output-format stream-json`) | Rappel HTTP depuis le conteneur |
| X6 | Exécuteur en local | **Kubernetes partout** : un cluster kind à côté du compose | Docker en local, Kubernetes en prod |
| X7 | Pull request | **Ouverte par le worker, en brouillon**, avec la GitHub App | Branche seulement |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Exécutant | Agent qui traite une carte validée sur un dépôt (Claude Code). Client FerrisKey `capitainerie-agent-executor`. | « agent de code », « worker » |
| Exécution | Run d'un seul step `Execute`, rattaché à une carte et à un dépôt. | « tâche », « job » (le Job est l'objet Kubernetes) |
| Proxy d'identifiants | Service `apps/egress-proxy`, seul point de sortie réseau du conteneur. | « gateway » |
| Jeton de tâche | Jeton aléatoire propre à une exécution, que seul le proxy accepte. | « clé » |

## Invariants

1. Seule une carte en « À faire » peut être confiée, et seulement par un humain. [type : `ExecutableCard::try_from(Card)` ; test PDP : `card.execute` refusé à un acteur agent]
2. Une carte a au plus une exécution active. [index unique partiel en base + test]
3. Le conteneur ne reçoit jamais la clé Anthropic de l'organisation ni un jeton GitHub. [test : la spec du Job ne contient que `CAPITAINERIE_TASK_TOKEN` comme secret]
4. Le proxy n'accepte un jeton de tâche que pour une exécution `running` et non expirée ; il est révoqué à la fin de l'exécution, quel que soit le résultat. [test]
5. Côté Anthropic, le proxy ne laisse passer que `POST /v1/messages` et `POST /v1/messages/count_tokens`. [test]
6. Côté GitHub, le proxy ne laisse passer que la lecture du dépôt de l'exécution et un push sur sa branche `capitainerie/<id court de l'exécution>` : ni autre dépôt, ni autre branche, ni suppression de ref. [test sur les commandes `git-receive-pack`]
7. L'agent ne déplace jamais la carte. C'est le clic humain qui la passe en « En cours », dans la même transaction que la création de l'exécution. [test]
8. Le modèle de l'Exécutant est un modèle Anthropic. [type : `ExecutorModel(String)`, pas un `ModelRef` multi-fournisseur]
9. Une exécution est bornée : durée (`activeDeadlineSeconds`, 30 min par défaut) et nombre de tours (`--max-turns`, 50 par défaut). [test sur la spec du Job]
10. Aucune clé ni aucun jeton dans les logs, les traces, les événements ou la timeline. [test : les sorties `stream-json` sont filtrées avant stockage]
11. Une installation GitHub n'est liée à une organisation que si l'admin qui la connecte y a accès (OAuth GitHub) ; une installation n'est liée qu'à une seule organisation. [test]

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
- Actions PDP : `CardExecute` (humain, membre, carte en « À faire »), `GithubManage` (admin). L'acteur `capitainerie-agent-executor` garde les droits des agents sur les cartes (commenter), rien de plus.
- Use cases :
  - `execute_card(subject, card_id, repository, expected_version)` : vérifie l'invariant 1 et le PDP, échange le jeton de l'utilisateur pour l'Exécutant (comme `submit_transcript_delegated`), puis dans une transaction : carte → « En cours », `Run(Queued, [Execute], Card{..})`, délégation chiffrée, événement `RunQueued`.
  - `connect_github(subject, installation_id, state)` et `disconnect_github(subject)`.

### Ports

| Port | Module | Dispatch | Rôle |
|---|---|---|---|
| `TaskRunner` | `core/domain/execution/ports.rs` | générique | lancer un Job, suivre son flux de sortie ligne à ligne, connaître son issue, le supprimer |
| `CodeHost` | `core/domain/execution/ports.rs` | générique | lister les dépôts d'une installation, ouvrir une PR en brouillon |
| `TaskGrantRepository` | `core/domain/execution/ports.rs` | générique | créer, révoquer, retrouver un jeton de tâche (haché) |
| `GithubInstallationRepository` | `core/domain/execution/ports.rs` | générique | une installation par organisation |

### Adapters

- `libs/runner-kube` : `TaskRunner` avec `kube` + `k8s-openapi` (création du Job, suivi des logs du pod, statut). Namespace dédié `capitainerie-tasks`, `ttlSecondsAfterFinished: 600`, `backoffLimit: 0`, pod non root, système de fichiers racine en lecture seule sauf `/workspace` (emptyDir), aucun ServiceAccount monté.
- `libs/github` : `CodeHost` en HTTP (`reqwest`) ; JWT de la GitHub App en RS256 (`jsonwebtoken`, déjà présent), jetons d'installation limités à un dépôt (`repositories: [name]`, permissions `contents: write`, `pull_requests: write`).
- `libs/core/infrastructure` : repositories Postgres (`github_installations`, `task_grants` avec `token_sha256`, `run_id`, `expires_at`, `revoked_at`).
- `apps/egress-proxy` : axum.
  - `/anthropic/*` : vérifie `x-api-key` = jeton de tâche, le remplace par la clé de l'organisation (`provider_access`, déchiffrée avec `SECRETS_KEY`), transmet en streaming à `https://api.anthropic.com`.
  - `/git/<owner>/<repo>.git/*` : vérifie le mot de passe Basic = jeton de tâche et le dépôt de l'exécution, lit les commandes `pkt-line` d'un `git-receive-pack` pour l'invariant 6, puis transmet à `github.com` avec un jeton d'installation.
  - Tout le reste : 403.
- `NetworkPolicy` du namespace des tâches : sortie autorisée vers le proxy et le DNS seulement.

### Image `capitainerie-executor`

- Base `node:22-slim`, `git`, Claude Code en version figée (paquet npm `@anthropic-ai/claude-code@<version>`), script d'entrée `deploy/executor/entrypoint.sh`.
- Le script : clone par le proxy, crée la branche, lance `claude -p "$PROMPT" --output-format stream-json --verbose --model "$MODEL" --max-turns "$MAX_TURNS" --allowedTools "Read,Edit,Write,Glob,Grep,Bash"`, puis commite et pousse s'il y a des changements. Il émet des lignes `{"type":"capitainerie",...}` (cloné, poussé avec le SHA, aucun changement) que le worker distingue des lignes de Claude Code.
- Variables : `ANTHROPIC_BASE_URL` (le proxy), `ANTHROPIC_API_KEY` et le mot de passe git = jeton de tâche, `PROMPT`, `MODEL`, `MAX_TURNS`, `REPOSITORY`, `BRANCH`.

### Worker

- `ExecutePipeline` (à côté de `MeetingPipeline`) : réclame le run, crée le jeton de tâche, lance le Job, lit le flux :
  - chaque `tool_use` / `tool_result` devient un `ToolCall` de l'étape (sortie tronquée et filtrée, invariant 10) ;
  - le message `result` final donne l'usage (tokens), le coût calculé avec la table de prix, le résumé.
- À la fin, quel que soit le résultat : révocation du jeton de tâche, suppression de la délégation.
- Si une branche a été poussée : PR en brouillon (titre = titre de la carte, corps = lien vers la carte, résumé, coût), puis commentaire sur la carte via le MCP avec le jeton délégué : lien de la PR et résumé. Sinon : commentaire « aucun changement » ou raison de l'échec.
- Reprise : un run `Execute` repris dont le Job existe encore se rattache à lui (nom du Job = id du run) ; sinon il échoue avec une raison claire, sans relancer un second Job.

### API et console

| Méthode | Chemin | Rôle requis |
|---|---|---|
| `POST` | `/api/v1/cards/{id}/executions` (`repository`, `expected_version`) | membre |
| `GET` | `/api/v1/integrations/github` | membre |
| `GET` | `/api/v1/integrations/github/install` (URL d'installation avec `state` signé lié à l'organisation) | admin |
| `POST` | `/api/v1/integrations/github/callback` (`installation_id`, `state`) | admin |
| `DELETE` | `/api/v1/integrations/github` | admin |
| `GET` | `/api/v1/integrations/github/repositories` | membre |

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
  Given le jeton d'une exécution sur "acme/web", branche "capitainerie/0a1b2c3d"
  When le conteneur pousse sur "main", ou lit "acme/autre"
  Then le proxy répond 403
  And un push sur "capitainerie/0a1b2c3d" passe

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

@spec-exec-11 Scenario: une installation GitHub inaccessible ou déjà liée est refusée
  Given un admin dont le code OAuth GitHub ne donne pas accès à l'installation
  When il appelle POST /api/v1/integrations/github/callback avec { installation_id, code, state }
  Then la réponse est 403 `installation_not_accessible` et rien n'est stocké
  And une installation déjà liée à une autre organisation répond 409 `installation_already_connected`
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

Dans le code : `libs/core/src/domain/execution/mod.rs` (types) et `ports.rs` (ports `TaskRunner`, `CodeHost`, `TaskGrantRepository`, `GithubInstallationRepository`). Ces fichiers font foi.

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
| Le prompt de l'Exécutant (titre, description, citation de la carte) est une entrée utilisateur : injection possible. Atténuation : isolation, proxy, aucun secret, PR en brouillon relue par un humain. | W5 |
| Coût d'une exécution difficile à prévoir : affiché après coup dans la timeline et la page Agents ; un plafond par organisation viendra si le besoin apparaît. | produit |
| Le jeton délégué de l'utilisateur est borné par sa session FerrisKey (besoin 5) : une exécution longue peut ne plus pouvoir commenter la carte. Le commentaire final se fait alors au nom du compte de service de l'Exécutant, marqué comme tel. | W5 |

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
