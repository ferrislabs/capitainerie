# handlers-realtime

Passerelle WebSocket qui pousse à la console les changements de runs et de cartes, pour remplacer l'essentiel du polling.

## Route

`GET /api/v1/realtime` : mise à niveau WebSocket, **sans** middleware d'authentification (un navigateur ne peut pas envoyer `Authorization` sur un WebSocket, et le jeton ne doit jamais figurer dans l'URL). La route n'est pas dans l'OpenAPI.

## Protocole

Tous les messages sont du JSON texte avec un champ `op`.

Client vers serveur :

| Message | Rôle |
|---|---|
| `{"op":"identify","token":"<access token>"}` | Premier message obligatoire, dans les 10 s. Renvoyable à tout moment pour renouveler le jeton. |

Serveur vers client :

| Message | Sens |
|---|---|
| `{"op":"ready"}` | Identification acceptée (aussi renvoyé après un renouvellement, et quand le bus redevient joignable). |
| `{"op":"degraded"}` | Le bus Iggy est injoignable : aucun événement ne viendra, la console doit continuer à interroger l'API. |
| `{"op":"resync"}` | Des événements ont pu être perdus (socket trop lent, bus de retour) : tout recharger. |
| `{"op":"event","kind":"run.started","run_id":"…","card_id":"…","occurred_at":"…"}` | Un changement. `run_id` et `card_id` sont omis s'ils sont absents. Rien d'autre du contenu de l'événement ne transite. |

`kind` vaut `run.queued`, `run.started`, `run.step_finished`, `run.finished`, `card.proposed`, `card.updated`, `card.moved` ou `card.commented`.

### Authentification

Même chemin que le middleware HTTP : émetteur lu sans vérification, `organization_by_issuer`, puis `verifier.verify` avec l'audience configurée. Les événements ne sont envoyés qu'aux sockets de la **même organisation** que le jeton.

### Fermetures

| Code | Raison |
|---|---|
| 4001 | Pas d'`identify` valide dans les 10 s, jeton refusé, ou jeton expiré (`exp` dépassé sans nouvel `identify`). |
| 4003 | Un nouvel `identify` porte un autre sujet (ou une autre organisation). |
| 1011 | Pas de réponse au ping (le serveur pingue toutes les 30 s ; tout message reçu compte comme preuve de vie). |

## Côté serveur

Une tâche par processus API (`consumer::spawn`) consomme les topics `runs` et `cards` d'Iggy avec un groupe `api-realtime-<uuid v7>` propre au processus, et diffuse sur un `tokio::sync::broadcast` borné (256). Un groupe neuf rejoue le topic depuis son début : les enveloppes dont `occurred_at` précède le démarrage du processus sont ignorées, donc seuls les événements postérieurs sont poussés. Un socket en retard reçoit `resync` au lieu d'être déconnecté.

Si Iggy est injoignable au démarrage (ou le devient), l'API sert toujours le HTTP, la tâche réessaie (1 s à 30 s) et les sockets reçoivent `degraded` ; au retour du bus ils reçoivent `ready` puis `resync`.

Chaque connexion a un span `realtime.connection` portant `organization_id` (jamais le jeton).
