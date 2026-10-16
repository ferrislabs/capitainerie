# Besoins de Capitainerie côté FerrisKey

Briefing autonome à donner à un agent qui travaille sur le repo `ferriskey`.
Auteur du besoin : projet Capitainerie (`/opt/ferrislabs/capitainerie`), plateforme d'orchestration d'agents IA.
Constat établi le 2026-10-16 sur `ferriskey/ferriskey` `main` (a4848755) et `origin/main` (ab8a2b85). Les 7 commits d'écart sont des montées de version de dépendances et ne changent rien aux points ci-dessous.

## Contexte

Capitainerie fait travailler des agents IA **pour le compte d'un utilisateur**. Exemple : un utilisateur dépose le transcript d'une réunion, puis une chaîne d'agents crée des cartes Kanban via un serveur MCP.

- Un realm FerrisKey par organisation. Les humains se connectent en OIDC (Authorization Code + PKCE).
- Chaque agent est un client FerrisKey confidentiel (`client_credentials`).
- Un agent qui agit pour un utilisateur obtient par token exchange (RFC 8693) un jeton délégué : `sub` = l'utilisateur, `act` = l'agent.
- Le serveur MCP Kanban est une ressource protégée. Avant chaque écriture, il demande une décision au **PDP AuthZEN de FerrisKey**.
- Un run est **asynchrone** : il dure de quelques secondes à plusieurs dizaines de minutes, et peut reprendre après un redémarrage.

## Déjà présent dans FerrisKey (vérifié, rien à faire)

| Capacité | Où |
|---|---|
| Token exchange RFC 8693 avec `actor_token` et `act` imbriqué | `core/src/domain/authentication/token_exchange/services.rs` (`act_claim`) |
| Policies d'échange (`allow_delegation`, `allow_impersonation`, `target_audience`, `allowed_scopes`) + API admin `POST /clients/{id}/token-exchange-policies` | `core/src/entity/token_exchange_policies.rs`, `api/tests/token_exchange_policy_test.rs` |
| Realm comme serveur d'autorisation d'un serveur MCP | #1611, `api/tests/mcp_resource_owner_test.rs` |
| Règles d'autorisation propres à un realm | #1575, `libs/ferriskey-authz` |
| Audit `TokenExchanged` | `services.rs` (`audit`) |

## Besoin 1 : endpoint HTTP AuthZEN (PDP)

**Constat.** Le endpoint est spécifié dans `docs/design/authz/07-http-authzen.md`, mais `api/` ne l'expose pas : aucune occurrence de `authzen` sous `api/`, sur `main` comme sur `origin/main`. `libs/ferriskey-authz` contient les ports, les entités et le moteur, sans route HTTP.

**Attendu.**
1. `POST /realms/{realm}/authzen/access/v1/evaluation`, conforme à AuthZEN Authorization API 1.0 et à `07-http-authzen.md`.
2. `POST /realms/{realm}/authzen/access/v1/evaluations` (batch) : un run vérifie plusieurs écritures d'un coup.
3. Authentification de l'appelant (le PEP) par un jeton `client_credentials` du realm.
4. Prise en compte de la délégation. Le PEP envoie l'utilisateur en `subject` et la chaîne `act` en `context`, par exemple `context.actor = { "type": "client", "id": "agent-writer" }`. Une règle doit pouvoir dire : « l'agent X peut créer une carte pour l'utilisateur U seulement si U peut créer une carte ». Il faut aussi pouvoir restreindre un agent à certaines actions, quels que soient les droits de U.
5. Réponse : `{ "decision": bool, "context": { "reason": ... } }`, avec une raison exploitable dans les logs et dans une timeline d'audit.
6. Chaque décision est auditée : sujet, acteur, action, ressource, décision.

**Ressources et actions Capitainerie (exemples pour les tests)** :
- `transcript` : `create`, `read`.
- `run` : `read`, `cancel`.
- `card` : `create`, `update`, `comment`, `move`.
- Règle métier à exprimer : seul un humain fait sortir une carte de la colonne `proposed`. Ressource `card` avec la propriété `column`, action `move`, refus si `context.actor` est présent et `resource.properties.column == "proposed"`.

**Critères d'acceptation.**
- Un test HTTP : décision `true` pour un utilisateur autorisé, `false` pour un utilisateur non autorisé.
- Un test où le même utilisateur obtient `false` quand un acteur agent est présent et que la règle de l'agent l'interdit.
- Un test batch.
- La spec OpenAPI est mise à jour.

## Besoin 2 : délégation durable pour un travail asynchrone

**Constat.** `exchanged_claims` (`token_exchange/services.rs`) dit : « The new token never outlives the subject token ». Avec des jetons utilisateur de 5 minutes, un run qui dure plus longtemps ne peut plus obtenir de jeton délégué valide. Ré-échanger en chaîne n'aide pas : chaque jeton est borné par l'`exp` du jeton d'origine. Stocker le refresh token de la console n'est pas acceptable : FerrisKey fait tourner les refresh tokens et révoque toute la famille en cas de rejeu, ce qui casserait la session du navigateur.

Deuxième contrainte. `verify_subject` exige que le client demandeur soit partie au jeton sujet (`azp`, `aud` ou audience possédée). Le client d'un agent n'est pas partie au jeton de la console. Il faut donc un flux documenté pour qu'un agent obtienne un jeton délégué à partir d'un consentement donné par l'utilisateur via la console ou l'API Capitainerie.

**Attendu (le choix du mécanisme revient à FerrisKey).** Un **grant de délégation** :
1. Créé pendant que l'utilisateur est présent, à partir de son jeton (par exemple par le client `capitainerie-api`, partie au jeton).
2. Lié à : utilisateur, client agent autorisé (ou liste de clients), audience cible (`capitainerie-mcp`), scopes, durée maximale (exemple : 1 h), et un identifiant de contexte libre (`run_id`).
3. Échangeable plusieurs fois par le client agent, via token exchange avec son `actor_token` `client_credentials`, contre un access token court : `sub` = utilisateur, `act` = agent, `aud` = cible.
4. Révocable par l'utilisateur, par un admin du realm, et par le client qui l'a créé (annulation du run).
5. Invalidé si l'utilisateur est désactivé ou retiré du realm.
6. Audité : création, chaque échange, révocation.

Pistes possibles, à évaluer par l'agent FerrisKey :
- **A.** Un nouveau `subject_token_type` (par exemple `urn:ferriskey:params:oauth:token-type:delegation-grant`) dont la valeur est un handle opaque.
- **B.** Un refresh token de délégation dédié, distinct de la famille de la session navigateur, lié au client agent et à l'audience.

**Critères d'acceptation.**
- Un agent obtient un jeton délégué valide 30 minutes après la création du grant, alors que le jeton utilisateur d'origine a expiré.
- Après révocation, l'échange est refusé.
- Un autre client que celui autorisé est refusé.
- Le claim `act` du jeton émis contient le client agent.
- La durée maximale est respectée.

## Besoin 3 : audience des access tokens — résolu sur `main`, pas encore publié

Les resource indicators (RFC 8707) existent sur `main` : la console demande `resource=http://localhost:4100` au login, et le token porte `aud = ["http://localhost:4100"]`. Avec la propriété de la ressource (#1611, `PUT /realms/{realm}/resource-owners`), `capitainerie-api` devient partie au token et peut l'échanger. Vérifié contre l'image `ghcr.io/ferriskey/ferriskey-api:sha-ab8a2b8`.

**Reste à faire côté FerrisKey :**
1. Publier une release qui contient `resource-owners` : la 0.9.0 ne l'a pas, ce qui oblige Capitainerie à figer un build de `main`.
2. Le password grant ignore `resource` (seuls le code d'autorisation et le refresh l'appliquent). Les tests automatisés ne peuvent donc pas obtenir un token avec audience sans passer par un navigateur.

## Besoin 4 : outillage de provisioning

Constatés en écrivant `deploy/ferriskey/terraform` et `scripts/bootstrap-ferriskey.sh` :

1. Le provider Terraform `ferriskey/ferriskey` 0.1.0 ne déclare ni les policies d'échange, ni `allowed_resources`, ni les propriétaires de ressources, ni `token_exchange_enabled` d'un client. Tout passe par l'API admin dans `scripts/bootstrap-ferriskey.sh`.
2. `ferriskey_client.secret` contient le masque `***` au lieu du secret. `autharie` contourne en lisant `/clients/{id}/client-secret`.
3. Il n'y a pas d'API admin pour définir le mot de passe d'un utilisateur. Un environnement local ne peut avoir d'utilisateur qu'en activant l'auto-inscription.
4. Après « Create Account », la page d'inscription renvoie vers le `redirect_uri` **sans** `code` : le flux OIDC ne se termine pas, et l'utilisateur doit se reconnecter. Un client qui ne s'y attend pas boucle.

## Hors périmètre

- Provisioning des realms et des clients : l'API admin existante suffit pour le MVP Capitainerie.
- Introspection des jetons : le serveur MCP vérifie les JWT localement via JWKS.
