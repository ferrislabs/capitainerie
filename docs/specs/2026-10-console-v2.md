# Spec : console v2, clients et projets, cartes détaillées, chat

Statut : décisions validées le 2026-10-19 (tableau ci-dessous). Taille L, livrée en 4 étapes, une PR chacune. L'étape 4 (chat) aura sa propre spec détaillée avant le code.

## Objectif

1. Une interface plus simple et plus accessible : des cartes là où elles aident, une liste des agents lisible.
2. Des fiches clients et projets, pour qu'une tâche soit rattachée à un client et à un projet, au dépôt ou automatiquement.
3. Des cartes de Kanban plus détaillées : critères d'acceptation, priorité, échéance, responsable, étiquettes, estimation.
4. Une page de chat pour lancer des missions en dialoguant avec un agent, ouverte plus tard aux outils et connecteurs.

La direction visuelle reste celle de `apps/console/DESIGN.md`, dont la v2 s'inspire des principes de Mistral Studio et du Chat de Mistral (accueil centré sur un champ de saisie, liste des agents en tableau avec recherche et filtres). Rien n'en est copié.

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| V1 | Rôle du chat | **Agent conversationnel « Capitaine »** : réponses en streaming, questions de clarification, cartes proposées dans le fil et validées par l'humain, outils | Saisie rapide traitée comme un mini-transcript |
| V2 | Lien client et projet | **Choix au dépôt ou dans le chat, sinon détection** par l'extracteur parmi les fiches existantes ; le lien reste modifiable | Toujours manuel |
| V3 | Détails des cartes | **Critères d'acceptation, priorité et échéance, responsable et étiquettes, estimation** | — |
| V4 | Ordre | **Design, puis clients et projets, puis cartes détaillées, puis chat** | Chat d'abord |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Client | Organisation ou personne pour qui l'équipe travaille. | « compte », « customer » |
| Projet | Travail suivi pour un client (ou interne), avec un dépôt GitHub par défaut facultatif. | « repo » |
| Membre | Humain de l'organisation qui s'est déjà connecté à la console. | « utilisateur » |
| Critère d'acceptation | Condition vérifiable qui doit être vraie quand la carte est terminée. | « DoD », « checklist » |
| Capitaine | Agent conversationnel de la page de chat. | « assistant », « bot » |

## Invariants

1. Une carte, un transcript, un projet ne peuvent référencer qu'un client ou un projet **de la même organisation**. [contrainte en base + test]
2. Un projet rattaché à un client garde ce client : une carte liée à un projet a le client du projet. [type + test]
3. Un lien proposé par l'extracteur ne vise qu'une fiche existante et non archivée ; une référence inconnue est ignorée, jamais créée. [test]
4. Un choix humain (au dépôt ou sur la carte) l'emporte toujours sur la détection. [test]
5. Toute modification d'une carte passe par `expected_version` ; une version périmée répond 409. [test]
6. Le responsable d'une carte est un membre de l'organisation. [test : 422 sinon]
7. Les critères d'acceptation d'une carte confiée à Claude Code figurent dans son invite. [test sur l'invite]

---

## Étape 1 : design (console seule, aucune route d'API nouvelle)

- **Accueil `/`** (remplace la redirection vers `/runs`) :
  - titre « Bonsoir, <prénom> » (bonjour avant 18 h), sous-titre court ;
  - une rangée de cartes : « À valider » (cartes proposées, jaune, lien Kanban), « Agents en cours » (runs actifs, lien En cours), « Coût du mois » (analytics), « Déposer une réunion » (lien vers le formulaire) ;
  - « Activité récente » : les 5 derniers runs en liste compacte ;
  - l'emplacement du champ de chat est réservé en tête de page pour l'étape 4.
- **Agents `/agents`**, sur le modèle d'un tableau Studio :
  - en-tête de page (titre, description) ;
  - barre d'outils : filtres en puces « Source » et « Modèle » (bordure en pointillés tant qu'aucune valeur n'est choisie), champ de recherche plein ;
  - colonnes : Agent (nom lisible), Description, Source (badge : Pipeline, Exécutant, MCP), Modèle, Runs, Dernière activité, Coût, Identifiant (mono, copiable) ;
  - le nom, la description et la source viennent d'un catalogue côté console (`domain/agents/catalog.ts`) indexé par l'identifiant client ; le modèle vient des assignations ; un agent inconnu s'affiche avec son identifiant ;
  - un clic ouvre un panneau latéral : rôle, modèle, statistiques, 10 derniers runs.
- **Paramètres `/settings`** : une grille de cartes (Fournisseurs, Modèles, Intégrations) avec l'état de chacune (« 2 fournisseurs », « Transcripteur non choisi », « GitHub connecté ») au lieu de la redirection.
- **Kanban** : la carte est réorganisée en zones (titre, méta, pied) pour accueillir les étapes 2 et 3 ; densité inchangée.
- **États vides** : en carte, avec une action.
- `DESIGN.md` gagne la section « Cartes » (surface `card`, 8 px, bordure, aucune ombre au repos, survol `muted`) et la barre d'outils de filtres.

## Étape 2 : clients et projets

### Domaine et base

- Migration `20261021000001_clients_projects` :
  - `clients (id, organization_id, name, aliases TEXT[], notes, created_by, created_at, updated_at, archived_at)`, nom unique par organisation sans casse ;
  - `projects (id, organization_id, client_id NULL, name, aliases TEXT[], repository NULL, description, created_by, created_at, updated_at, archived_at)`, nom unique par organisation et client sans casse ;
  - `cards`, `transcripts` gagnent `client_id`, `project_id` nullables ;
  - clés étrangères composites `(organization_id, id)` pour l'invariant 1.
- Limites : nom ≤ 120 caractères, 10 alias ≤ 120 caractères (mêmes règles que les participants : ni contrôle, ni virgule), notes et description ≤ 5 000.

### API

| Méthode | Chemin | Rôle |
|---|---|---|
| `GET` | `/api/v1/clients` (`?q=`, `?archived=`) | membre |
| `POST` | `/api/v1/clients` | membre humain |
| `GET` | `/api/v1/clients/{id}` (avec ses projets et le nombre de cartes par colonne) | membre |
| `PATCH` | `/api/v1/clients/{id}` (champs, `archived`) | membre humain |
| idem | `/api/v1/projects…` (`?client_id=`) | idem |
| `PATCH` | `/api/v1/cards/{id}` `{ expected_version, client_id?, project_id? }` | membre humain |

- `POST /api/v1/transcripts` accepte `client_id` et `project_id` facultatifs. Le `repository_hint` vient du transcript, sinon du projet.
- Actions PDP : `client.read`, `client.manage`, `card.update`. Les agents n'en ont aucune.

### Détection

- L'extracteur reçoit, après l'en-tête de réunion, la liste des clients et projets non archivés (id court, nom, alias ; 200 au plus, les plus récemment utilisés d'abord).
- Chaque action extraite peut porter `client_ref` et `project_ref` (id court). Le cœur les résout, ignore les inconnus (invariant 3) et applique l'invariant 2.
- Un choix au dépôt s'applique à toutes les cartes du run et désactive la détection.

### Console

- Section « Travail » : « Clients » et « Projets ».
- Liste en cartes (nom, alias, projets, cartes ouvertes) avec recherche ; fiche détail avec ses projets, ses cartes par colonne et ses derniers runs ; création et édition en dialogue.
- Formulaire de dépôt : client et projet dans « Détails de la réunion » (le projet filtre par client, choisir un projet remplit le client et le dépôt).
- Carte et panneau : puce client et projet, modifiables dans le panneau. Kanban : filtres Client et Projet dans la barre d'outils.

## Étape 3 : cartes détaillées

### Domaine et base

- Migration `20261022000001_card_details` :
  - `cards` gagne `acceptance_criteria JSONB NOT NULL DEFAULT '[]'` (`[{ text, done }]`, 20 au plus, 300 caractères chacun), `priority TEXT NOT NULL DEFAULT 'normal'` (`low | normal | high | urgent`), `due_date DATE`, `assignee_subject TEXT`, `labels TEXT[] NOT NULL DEFAULT '{}'` (10 au plus, 30 caractères, sans casse en double), `estimate TEXT` (`xs | s | m | l | xl`) ;
  - `members (organization_id, subject, name, email, last_seen_at)`, mis à jour à chaque `GET /api/v1/me`.
- Le rédacteur propose critères, priorité et estimation, et une échéance seulement si le transcript en cite une.
- L'invite de Claude Code liste les critères d'acceptation (invariant 7).

### API

- `PATCH /api/v1/cards/{id}` accepte aussi `title`, `description`, `acceptance_criteria`, `priority`, `due_date`, `assignee_subject`, `labels`, `estimate` (absent = inchangé, `null` = effacé). Événement `CardUpdated`.
- `GET /api/v1/members` : membres de l'organisation (membre).
- `GET /api/v1/cards` accepte les filtres `client_id`, `project_id`, `priority`, `assignee`, `label`.

### Console

- Carte du Kanban : badge de priorité (urgente et haute seulement), échéance (rouge si dépassée, avec le mot « en retard »), initiales du responsable, étiquettes (2 visibles + compteur), progression des critères `n/m`, puces client et projet.
- Panneau : tous les champs modifiables sur place, critères cochables, enregistrement par champ.
- Barre d'outils du Kanban : filtres Priorité, Responsable, Étiquette en plus de Client et Projet ; filtres dans l'URL.

## Étape 4 : chat (spec détaillée à écrire avant le code)

Décisions déjà prises (V1) :
- l'accueil `/` porte le champ « Que lance-t-on ? » ; envoyer ouvre une conversation `/chat/<id>` ;
- le Capitaine répond en streaming, pose des questions quand la demande est floue, propose des cartes dans le fil (avec client, projet, critères) que l'humain valide ou modifie, et peut les confier à Claude Code ;
- il agit uniquement par des outils déclarés, avec le PDP : lister et chercher clients, projets et cartes ; proposer une carte ; confier une carte validée ;
- les conversations sont gardées et listées ;
- les connecteurs (outils externes, MCP) viendront s'ajouter à cette liste d'outils.

À trancher dans sa spec : rôle de modèle `captain`, transport du streaming, stockage des messages, budget de tokens par conversation, champ audio du chat.

## Hors périmètre

- Import de clients depuis un CRM, facturation, contacts.
- Sous-tâches et dépendances entre cartes.
- Notifications (e-mail, Slack) au responsable.
