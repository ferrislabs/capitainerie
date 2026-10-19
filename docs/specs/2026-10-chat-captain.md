# Spec : le chat, l'agent « Capitaine »

Statut : brouillon du 2026-10-19. Les décisions C1 à C4 viennent de la spec `2026-10-console-v2.md` (V1). Les décisions C5 à C9 sont des choix par défaut pris pendant l'absence de l'utilisateur, **à confirmer** à la relecture. Taille L.

## Objectif

Depuis l'accueil, un membre écrit une mission (« Ajoute un README au dépôt test pour Acme »). Le Capitaine répond en streaming, pose une question si la demande est floue, puis propose des cartes dans le fil. Le membre les valide, les corrige ou les confie à Claude Code sans quitter la conversation. Les outils du Capitaine sont déclarés, passent par le PDP, et accueilleront plus tard des connecteurs externes.

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| C1 | Nature | Agent conversationnel, streaming, outils, conversations gardées | Saisie one-shot |
| C2 | Entrée | Le champ de l'accueil ; envoyer crée une conversation `/chat/<id>` | Page chat seule |
| C3 | Validation | Une carte proposée par le Capitaine est `proposed` ; seul un clic humain dans le fil la valide (`todo`) ou la confie à Claude Code | Le Capitaine valide lui-même |
| C4 | Connecteurs | Plus tard, comme de nouveaux outils dans la même liste | — |
| C5 (à confirmer) | Où tourne un tour de conversation | **Dans l'API**, pendant la requête : le jeton de l'utilisateur est disponible, échangé (RFC 8693) contre un jeton du client `capitainerie-agent-captain` avec `act`, comme le worker pour le Rédacteur | Dans le worker, avec les deltas relayés par Iggy et le WebSocket |
| C6 (à confirmer) | Transport | **SSE** sur la réponse de `POST /api/v1/conversations/{id}/messages` | WebSocket temps réel existant |
| C7 (à confirmer) | Modèle | Nouveau rôle **`captain`**, assignable à tout modèle `chat` qui sait appeler des outils ; par défaut le modèle du Rédacteur | Réutiliser le rôle `writer` |
| C8 (à confirmer) | Outils | Par le **serveur MCP du Kanban** (mêmes contrôles que les autres agents), plus des outils de lecture servis par l'API | Appels directs au cœur |
| C9 (à confirmer) | Bornes | 20 appels d'outils et 16 000 tokens de sortie par tour ; 400 000 tokens par conversation, au-delà le champ indique d'en ouvrir une nouvelle | Aucune borne |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Conversation | Fil de messages entre un membre et le Capitaine. | « thread », « session » |
| Tour | Un message du membre et tout ce que le Capitaine fait pour y répondre (texte, appels d'outils). | « échange » |
| Proposition | Carte créée par le Capitaine pendant un tour, affichée dans le fil avec ses actions. | « suggestion » |

## Invariants

1. Une conversation appartient à son auteur et à son organisation ; personne d'autre ne la lit. [test : 404]
2. Le Capitaine ne valide ni ne rejette une carte, et ne confie rien à Claude Code : ces actions sont des boutons humains. [test PDP + test du jeu d'outils]
3. Chaque appel d'outil passe par le PDP avec `actor = capitainerie-agent-captain` et est enregistré dans la conversation (nom, arguments résumés, succès, latence). [test]
4. Le jeton de l'utilisateur n'est ni stocké ni envoyé au modèle ; seul le jeton échangé sert, le temps du tour. [test]
5. Un tour interrompu (fermeture de l'onglet, erreur du fournisseur) laisse une conversation cohérente : le message du membre est gardé, la réponse partielle est marquée `interrupted`. [test]
6. Les bornes C9 sont appliquées côté serveur. [test]

## Contrats

### Données

- Migration `20261023000001_conversations` :
  - `conversations (id, organization_id, owner_subject, title, model JSONB, input_tokens, output_tokens, cost_micros, created_at, updated_at, archived_at)` ;
  - `conversation_messages (id, conversation_id, seq, role ('user' | 'assistant' | 'tool'), content JSONB, status ('complete' | 'interrupted' | 'failed'), created_at)`. `content` porte le texte, les appels d'outils et leurs résultats résumés, et les cartes proposées (`card_id`).
- Le titre est tiré du premier message (60 caractères), modifiable.

### API

| Méthode | Chemin | Réponse |
|---|---|---|
| `POST` | `/api/v1/conversations` `{ message }` | 201 `{ id }`, puis le client ouvre le flux du premier tour |
| `GET` | `/api/v1/conversations` | les conversations de l'appelant, la plus récente d'abord |
| `GET` | `/api/v1/conversations/{id}` | la conversation et ses messages |
| `POST` | `/api/v1/conversations/{id}/messages` `{ message }` | `text/event-stream` : événements `delta` (texte), `tool` (début et fin d'un appel), `card` (proposition), `done` (usage), `error` |
| `PATCH` | `/api/v1/conversations/{id}` `{ title?, archived? }` | 200 |

- Un seul tour à la fois par conversation (409 sinon).
- Actions PDP : `conversation.read`, `conversation.write` (humains).

### Outils du Capitaine (v1)

- Lecture : `search_cards` (texte, colonne, client, projet), `list_clients`, `list_projects`, `list_repositories` (installation GitHub).
- Écriture : `propose_card` (titre, description, client, projet, critères d'acceptation, priorité, estimation), `comment_card`.
- Les champs des étapes 2 et 3 (`client`, `project`, critères…) ne sont exposés que s'ils existent dans le cœur au moment de l'implémentation.

### Console

- Accueil : le champ « Que lance-t-on ? » prend l'emplacement réservé. Il propose le bouton micro (transcription audio existante) et, à droite, le modèle du Capitaine.
- `/chat/<id>` : fil de messages, texte en streaming, appels d'outils repliés (« A cherché 3 cartes »), propositions en cartes avec « Valider », « Rejeter », « Confier à Claude Code » et « Ouvrir ».
- Barre : entrée « Conversations », liste des conversations récentes.
- Clavier : Entrée envoie, Maj+Entrée va à la ligne, Échap interrompt le tour.

## Hors périmètre

- Les connecteurs externes (C4) et l'ajout d'outils par l'utilisateur.
- Le partage d'une conversation entre membres.
- Les pièces jointes autres que l'audio transcrit.
