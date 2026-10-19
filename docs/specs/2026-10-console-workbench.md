# Spec : Kanban vivant, agents en cours et dépôt enrichi

Statut : décisions validées le 2026-10-19 (tableau ci-dessous). Taille L, livrée en 4 PR.

## Objectif

Rendre la console utilisable au quotidien sur trois points :

1. voir en un coup d'œil les agents qui tournent, et suivre un run sur une frise chronologique ;
2. déplacer les cartes du Kanban au glisser-déposer, et ouvrir une carte ;
3. déposer une réunion autrement qu'avec un titre, un texte et un `.txt` : fichiers de sous-titres, métadonnées, audio transcrit.

La direction visuelle reste celle de `apps/console/DESIGN.md`. La frise s'inspire des principes de l'observabilité de Mistral Studio (frise de steps sur un axe de temps, liste des steps et inspecteur en bas). Rien n'en est copié.

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| W1 | Agents qui tournent | **Page « En cours »** dans Supervision, en direct. Le détail d'un run devient une frise avec un inspecteur | Filtre « en cours » ajouté à la page Runs |
| W2 | Glisser-déposer | **dnd-kit** (`@dnd-kit/core`, `@dnd-kit/sortable`, `@dnd-kit/utilities`) : clavier et lecteurs d'écran gérés | pragmatic-drag-and-drop, sans accessibilité clavier prête |
| W3 | Ouvrir une carte | **Panneau latéral** à droite, avec l'URL `/kanban?card=<id>` | Page dédiée `/kanban/<id>` |
| W4 | Dépôt | **Zone de dépôt et formats** (`.txt`, `.md`, `.vtt`, `.srt`), **métadonnées de réunion**, **audio transcrit** | Choix de l'agent d'extraction |
| W5 | Audio | **Transcrit puis jeté** : le texte revient dans le formulaire pour relecture, rien n'est stocké | Audio conservé en stockage objet |
| W6 | Livraison | **Cette spec, puis 4 PR** | Une seule PR |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Run actif | Run `queued` ou `running`. | « job » |
| Frise | Vue du détail d'un run : un bloc par step sur un axe de temps. | « waterfall », « gantt » |
| Inspecteur | Panneau sous la frise qui montre le step sélectionné. | « drawer » |
| Panneau de carte | Panneau latéral qui montre une carte ouverte. | « modale » |
| Métadonnées de réunion | Date, participants et dépôt par défaut, joints à un transcript. | « tags » |
| Transcripteur | Rôle de modèle qui transforme un audio en texte. | « STT » |

## Invariants

1. Un glisser-déposer envoie le même `POST /api/v1/cards/{id}/move` qu'avant, avec `expected_version`. Un 409 remet la carte à sa place et recharge le board. [test du hook]
2. Le Kanban reste utilisable sans souris : on prend une carte avec Espace, on la déplace avec les flèches, on la pose avec Espace, et chaque étape est annoncée en français. [test]
3. Le panneau de carte ne montre que les cartes de l'organisation de l'appelant ; sinon l'API répond 404, pas 403. [test]
4. Un commentaire humain porte le `subject` de l'appelant et aucun `actor`. Un agent ne peut pas passer par cette route. [test PDP : `card.comment` sans acteur agent]
5. Les métadonnées sont facultatives. Un transcript sans métadonnées se comporte exactement comme aujourd'hui. [test]
6. Le dépôt par défaut d'un transcript n'est qu'une préselection : la confiance à l'Exécutant reste un clic humain qui choisit le dépôt. [test : le dialogue préremplit, la requête porte le dépôt choisi]
7. Un audio n'est jamais écrit sur disque, en base ni dans les logs ou les traces. Seuls sa taille, sa durée et le modèle sont journalisés. [test : le handler ne touche aucun dépôt ; revue]
8. Le Transcripteur n'est assignable qu'à un fournisseur `mistral` ou `openai_compatible`. Un PUT des assignations qui le met sur `anthropic` répond 422. [type + test]

## PR 1 : Kanban au glisser-déposer et panneau de carte

### API

| Méthode | Chemin | Rôle | Réponse |
|---|---|---|---|
| `GET` | `/api/v1/cards/{card_id}` | membre | `CardDetailResponse` |
| `POST` | `/api/v1/cards/{card_id}/comments` | membre humain | 201 `CommentResponse`. Corps `{ body }`, non vide après `trim`, 10 000 caractères au plus, sinon 422 |

```rust
pub struct CardDetailResponse {
    pub card: CardResponse,
    pub comments: Vec<CommentResponse>,   // du plus ancien au plus récent
    pub runs: Vec<RunResponse>,           // le run qui a proposé la carte, puis ses exécutions, de la plus récente à la plus ancienne
}
pub struct CommentResponse {
    pub id: Uuid,
    pub body: String,
    pub author_subject: String,
    pub author_actor: Option<String>,     // client de l'agent, si un agent l'a écrit
    pub created_at: DateTime<Utc>,
}
```

- Action du PDP : `card.comment`. Les humains l'ont, les agents passent par le serveur MCP comme aujourd'hui.
- Un commentaire publie l'événement `CardCommented` sur le bus. Le temps réel recharge alors le panneau ouvert.

### Console

- **Board :**
  - `DndContext` avec les capteurs pointeur (distance d'activation de 4 px), tactile et clavier, et une colonne `useDroppable` par colonne ;
  - pendant le glisser, la carte suit le pointeur dans un `DragOverlay` et sa place d'origine reste en pointillés ;
  - une colonne interdite ne s'éclaire pas. Il n'y en a aucune pour un humain aujourd'hui, mais la règle est en un seul endroit (`canDrop(from, to)`) ;
  - au dépôt, mise à jour optimiste du cache `CARDS_QUERY_KEY`, puis appel de `move` ; en cas d'erreur, retour arrière et toast.
- **Ordre :** dans une colonne, les cartes gardent l'ordre de l'API. Il n'y a pas de réordonnancement manuel (hors périmètre).
- **Carte :**
  - un clic sur la carte, ou Entrée, ouvre le panneau ;
  - la liste « Déplacer vers… » quitte la carte et passe dans le panneau ;
  - les boutons « Valider » et « Rejeter » des cartes proposées restent sur la carte.
- **Panneau** (composant `Sheet` à créer sur le `Dialog` de Radix, à droite, 480 px, plein écran sous 640 px) :
  - en-tête : titre, badge de colonne, menu « Déplacer vers… » ;
  - description complète ;
  - extrait du transcript, avec un lien vers le run d'origine ;
  - « Confier à Claude Code » si la carte est « À faire » ;
  - runs liés : statut, date, lien vers la PR pour une exécution ;
  - fil des commentaires et champ pour en écrire un ;
  - l'URL porte `?card=<id>` : le panneau s'ouvre au chargement, Échap le ferme et retire le paramètre.

## PR 2 : page « En cours » et frise d'un run

### API

| Méthode | Chemin | Rôle | Réponse |
|---|---|---|---|
| `GET` | `/api/v1/runs/active` | membre | `ActiveRunListResponse` |

```rust
pub struct ActiveRunListResponse { pub items: Vec<ActiveRunResponse> }   // runs actifs, le plus ancien d'abord, 100 au plus
pub struct ActiveRunResponse {
    pub run: RunResponse,
    pub current_step: Option<ActiveStepResponse>,  // le dernier step `running`, sinon None (run en file)
    pub steps_done: u32,
    pub steps_total: u32,                          // longueur du pipeline
    pub input_tokens: u64,                         // cumul des steps, terminés ou non
    pub output_tokens: u64,
    pub cost_micros: u64,
}
pub struct ActiveStepResponse {
    pub kind: String,             // extract | deduplicate | write | execute
    pub agent: Option<String>,
    pub model: Option<String>,
    pub started_at: DateTime<Utc>,
}
```

### Console

- **Navigation :** une entrée « En cours » dans Supervision, au-dessus de Runs, avec le nombre de runs actifs. Le compteur est en bleu « running », pas en jaune : personne n'a à agir.
- **Page `/live` :**
  - une ligne par run actif : point bleu animé (statique si `prefers-reduced-motion`), agent du step en cours, titre ou dépôt, step `n/total` et son nom, durée qui avance chaque seconde côté client, tokens, coût ;
  - un clic ouvre le détail du run ;
  - rafraîchie par le temps réel (`run.*`), avec un repli sur un polling de 5 s ;
  - état vide : « Aucun agent ne tourne. »
- **Détail d'un run :** la liste actuelle des steps devient une frise.
  - Axe de temps : de `started_at` du run jusqu'à `finished_at`, ou maintenant si le run est actif. Graduations lisibles (`0 s`, `5 s`, `1 min`…).
  - Un bloc par step, placé selon `started_at` et `finished_at`. Couleur selon l'état (DESIGN.md, signaux) et une icône en plus de la couleur. Un step en cours s'étire jusqu'à maintenant.
  - Sous la frise, deux volets :
    - à gauche, la liste des steps ;
    - à droite, l'inspecteur du step sélectionné, avec les onglets « Sortie » (`detail`, `execution` ou `error`, en JSON indenté, sans couleur d'état, avec un bouton copier), « Outils » (`tool_calls`) et « Métadonnées » (agent, modèle, tokens, coût, latence, horodatages).
  - Les décisions du PDP restent sous la frise, comme aujourd'hui.
  - Le clavier sélectionne un step (flèches haut et bas dans la liste) ; la frise suit la sélection.

## PR 3 : zone de dépôt, formats et métadonnées

### API

`POST /api/v1/transcripts` accepte trois champs facultatifs de plus :

```rust
pub struct SubmitTranscriptRequest {
    pub title: String,
    pub body: String,
    pub meeting_date: Option<NaiveDate>,     // AAAA-MM-JJ
    pub participants: Option<Vec<String>>,   // 50 au plus, chacun non vide après trim, 100 caractères au plus
    pub repository: Option<String>,          // owner/name, validé par RepositoryRef
}
```

- **Migration `20261020000001_transcript_metadata` :** `transcripts` gagne `meeting_date DATE`, `participants TEXT[] NOT NULL DEFAULT '{}'` et `repository TEXT`.
- **Domaine :** `Transcript` gagne `metadata: MeetingMetadata { date, participants, repository }`. Les règles sont dans les constructeurs, testées.
- **Extracteur :** quand une métadonnée existe, l'invite reçoit un en-tête avant le transcript : `Date de la réunion : …`, `Participants : …`. Le dépôt n'est pas envoyé au modèle.
- **Réponses :** `RunResponse` gagne `repository_hint: Option<String>` pour les runs de transcript. `CardResponse` aussi, à partir du transcript d'origine, pour préremplir le dialogue « Confier à Claude Code ».

### Console

- **Formulaire :**
  - Titre.
  - Zone de dépôt. On y glisse un fichier, ou on clique pour en choisir un. Elle accepte `.txt`, `.md`, `.vtt` et `.srt`, 5 Mo au plus.
  - Transcript, qui reste éditable après l'import.
  - Volet « Détails de la réunion », replié par défaut, qui contient :
    - la date ;
    - les participants, saisis comme des étiquettes : on valide avec Entrée ou une virgule ;
    - le dépôt par défaut, avec la liste des dépôts de l'installation GitHub si elle existe.
- **Import :** l'analyse se fait côté client, dans `domain/runs/transcript-import.ts`, avec des tests.
  - `.vtt` : on retire l'en-tête `WEBVTT`, les numéros, les horodatages et les blocs `NOTE`. `<v Nom>texte` devient `Nom : texte`. Les répliques consécutives d'un même locuteur sont fusionnées.
  - `.srt` : on retire les numéros et les horodatages. Un préfixe `Nom:` est gardé.
  - `.md` et `.txt` : le texte est gardé tel quel.
  - Si le titre est vide, il prend le nom du fichier sans extension. Les participants détectés dans un `.vtt` préremplissent le champ.
- **Aperçu :** après l'import, une ligne indique le nom du fichier, le nombre de caractères et le nombre de participants détectés, avec un bouton « Retirer ».

## PR 4 : audio transcrit (empilée sur la PR 3)

### Domaine et fournisseurs

- `AgentRole` gagne `Transcriber` (`"transcriber"`). `ModelAssignments` gagne `transcriber: Option<ModelRef>` : il n'y a pas de transcripteur par défaut.
- Le PUT des assignations refuse un transcripteur `anthropic`, avec un 422.
- Le catalogue ajoute la capacité `transcription` aux modèles dont l'identifiant contient `voxtral` ou `whisper`.
- Port du cœur :

```rust
pub trait Transcriber: Send + Sync {
    fn transcribe(&self, model: &ModelRef, audio: AudioClip)
        -> impl Future<Output = Result<TranscriptionText, CoreError>> + Send;
}
pub struct AudioClip { pub file_name: String, pub mime: AudioMime, pub bytes: bytes::Bytes }
pub enum AudioMime { Mpeg, Mp4, Wav, Webm, Ogg }   // mp3, m4a/mp4, wav, webm, ogg
```

- L'adaptateur, dans `libs/runtime-rig`, appelle `POST {base}/v1/audio/transcriptions` en `multipart/form-data` (`file`, `model`). C'est la même route chez Mistral (Voxtral) et chez les serveurs compatibles OpenAI (Whisper). Il utilise la clé de l'organisation.

### API

| Méthode | Chemin | Rôle | Réponse |
|---|---|---|---|
| `POST` | `/api/v1/transcriptions` | membre humain | 200 `{ text, model }` |

- Corps en `multipart/form-data`, avec une seule partie `file` de 25 Mio au plus. La limite de corps est relevée pour cette route seulement.
- Erreurs :
  - 413 si le fichier est trop gros ;
  - 415 si le format n'est pas pris en charge ;
  - 409 `transcriber_not_configured` si aucun transcripteur n'est assigné ;
  - 502 si le fournisseur échoue, avec son message lisible.
- La route est synchrone : elle ne crée ni transcript ni run. Le texte revient dans le formulaire, l'humain le relit, puis il dépose (PR 3).
- Action du PDP : `transcription.create`, réservée aux humains.

### Console

- La zone de dépôt accepte aussi `.mp3`, `.m4a`, `.wav`, `.webm` et `.ogg`.
- Pendant la transcription, la zone affiche « Transcription en cours… » avec le nom du fichier et un bouton « Annuler » qui interrompt la requête.
- Le texte remplit le champ Transcript.
- Paramètres › Modèles affiche le rôle « Transcripteur », facultatif. Sans transcripteur, la zone indique qu'il faut en choisir un et renvoie vers la page.

## Hors périmètre

- Le réordonnancement des cartes dans une colonne (aucune position n'existe en base).
- L'historique des déplacements d'une carte dans le panneau.
- La modification du titre ou de la description d'une carte.
- La diarisation de l'audio (qui parle) et l'enregistrement depuis le micro.
- Le stockage de l'audio (W5).
- Le choix de l'agent d'extraction au dépôt.

## Plan de livraison

| PR | Branche | Base | Dépendances ajoutées |
|---|---|---|---|
| 1 | `feat/kanban-dnd-card-panel` | `main` | `@dnd-kit/core`, `@dnd-kit/sortable`, `@dnd-kit/utilities` (W2) |
| 2 | `feat/live-runs-timeline` | `main` | aucune |
| 3 | `feat/transcript-intake` | `main` | aucune |
| 4 | `feat/audio-transcription` | PR 3 | feature `multipart` d'axum (ajoute `multer`, pour recevoir l'audio) et de reqwest (pour l'envoyer au fournisseur) |

Le client `src/api/api.client.ts` est généré dans chaque PR. En cas de conflit au rebase, on le régénère ; on ne le fusionne pas à la main.
