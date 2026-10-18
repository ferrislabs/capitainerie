# Spec : fournisseurs de modèles, clés d'API et nouvelle console

Statut : validée le 2026-10-18 (décisions ci-dessous). Taille L.

## Objectif

Une organisation branche ses propres fournisseurs de modèles (Claude, Mistral, ou un endpoint compatible OpenAI comme Ollama ou vLLM), choisit quel modèle sert chaque rôle d'agent, et pilote tout cela depuis une console à la nouvelle identité visuelle.

## Décisions

| # | Décision |
|---|---|
| P1 | Les clés sont rattachées à l'**organisation** : une clé par fournisseur et par organisation, utilisée par tous ses runs. |
| P2 | Seuls les **admins de l'organisation** gèrent les clés et les choix de modèles. Un admin porte le rôle FerrisKey `capitainerie-admin` dans le realm de l'organisation. Les autres membres voient quels fournisseurs sont configurés et quels modèles sont choisis, jamais les clés. |
| P3 | La page Modèles affiche le **catalogue** des modèles disponibles avec les clés configurées, et permet de choisir un modèle **par rôle** : `extractor`, `judge`, `writer`, `embeddings`. |
| P4 | Le choix de modèles est **figé dans le run** au moment du dépôt : un changement de configuration n'affecte pas un run déjà en file. |
| P5 | Nouvelle identité visuelle, inspirée des principes de la console Qovery (navigation latérale à deux niveaux, pages denses, en-têtes de page, états vides soignés). La console Qovery est sous GPL-3.0 : **rien n'est copié** (ni code, ni composant, ni ressource) ; tout est réimplémenté avec shadcn et Tailwind. |
| P6 | Une clé n'est jamais renvoyée par l'API après son enregistrement : seuls le fournisseur, un aperçu (`sk-…a1b2`, les 4 derniers caractères), la date et l'auteur sont visibles. Elle est chiffrée au repos (AES-256-GCM, clé `SECRETS_KEY` distincte de `DELEGATION_KEY`, données associées `organization_id/provider`). |

## Contrats

### Fournisseurs

```rust
// core/domain/provider
pub enum ProviderKind { Anthropic, Mistral, OpenAiCompatible }      // as_str: "anthropic" | "mistral" | "openai_compatible"
pub struct ProviderCredential {                                       // jamais sérialisé vers l'API avec le secret
    pub organization_id: OrganizationId,
    pub kind: ProviderKind,
    pub secret: SecretToken,            // la clé d'API (vide autorisé pour un endpoint local sans clé)
    pub base_url: Option<String>,       // obligatoire pour OpenAiCompatible, ignoré sinon
    pub preview: String,                // 4 derniers caractères
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
}
pub enum AgentRole { Extractor, Judge, Writer, Embeddings }          // as_str: "extractor" | "judge" | "writer" | "embeddings"
pub struct ModelRef { pub provider: ProviderKind, pub model: String }
pub struct ModelAssignments { pub extractor: ModelRef, pub judge: ModelRef, pub writer: ModelRef, pub embeddings: ModelRef }
```

- Par défaut, sans configuration d'organisation, les rôles utilisent le fournisseur `openai_compatible` défini par l'environnement du worker (`LLM_BASE_URL`, `LLM_MODEL`, `EMBEDDING_MODEL`) : le comportement actuel reste celui d'une stack locale neuve.
- `Run` porte un champ `models: ModelAssignments`, écrit au dépôt (P4). Les runs existants, sans ce champ, prennent la configuration par défaut.

### Catalogue

`GET /api/v1/models` liste, pour chaque fournisseur configuré, les modèles que son API annonce (`GET /v1/models` chez Anthropic, Mistral et les endpoints compatibles OpenAI), avec : `provider`, `id`, `display_name`, `capabilities` (`chat`, `embeddings`), `context_window` si connu, `price` (par million de tokens, entrée et sortie) si connu. Les prix viennent d'une table versionnée dans le code (`libs/runtime-rig`), pas du fournisseur. Réponse mise en cache 10 min par organisation. Un fournisseur injoignable apparaît avec une erreur lisible, sans faire échouer la liste.

### API

| Méthode | Chemin | Rôle requis |
|---|---|---|
| `GET` | `/api/v1/providers` | membre |
| `PUT` | `/api/v1/providers/{kind}` (clé, `base_url`) | admin |
| `DELETE` | `/api/v1/providers/{kind}` | admin |
| `POST` | `/api/v1/providers/{kind}/test` (appel `GET /v1/models` avec la clé) | admin |
| `GET` | `/api/v1/models` | membre |
| `GET` | `/api/v1/model-assignments` | membre |
| `PUT` | `/api/v1/model-assignments` | admin |

Actions du PDP : `provider.read`, `provider.manage`, `models.assign`. Les agents n'ont accès à aucune.

### Identité

- `Identity` porte `roles: Vec<String>` (claim `realm_access.roles` des tokens FerrisKey) ; `Subject` porte `is_admin: bool`.
- `deploy/ferriskey/terraform` déclare le rôle `capitainerie-admin` ; `scripts/bootstrap-ferriskey.sh` l'attribue à un utilisateur nommé (`ADMIN_USERNAME`, `demo` par défaut) s'il existe.

### Exécution

- `AgentRuntime` reçoit le `ModelRef` du rôle et les identifiants du fournisseur de l'organisation pour chaque appel (extraction, juge, rédaction) ; l'`Embedder` aussi.
- `runtime-rig` construit le client du fournisseur à la demande, en cache par `(organisation, fournisseur, empreinte de la clé)`. Fournisseurs : Anthropic natif, Mistral natif, compatible OpenAI. Aucun type Rig ne sort du crate.
- Un changement du modèle d'embeddings rend l'index LanceDB de l'organisation incompatible : l'index est rangé par modèle d'embeddings (table par modèle) et l'indexeur réindexe les cartes de l'organisation dans la nouvelle table.

## Critères d'acceptation

1. Un admin enregistre une clé Anthropic : `GET /api/v1/providers` montre `anthropic` avec l'aperçu, jamais la clé ; la clé est chiffrée en base.
2. Un membre non admin reçoit 403 sur `PUT /api/v1/providers/anthropic`, et la décision est auditée.
3. `GET /api/v1/models` liste les modèles Claude annoncés par l'API Anthropic quand une clé valide est configurée, et ceux d'Ollama via le fournisseur compatible OpenAI.
4. Un admin assigne `claude-haiku-5-5` au rôle `extractor` : le run déposé ensuite l'utilise (visible dans la timeline : modèle de l'étape Extraction), un run déjà en file garde l'ancien modèle.
5. Sans aucune configuration d'organisation, un run fonctionne comme aujourd'hui (Ollama).
6. La console présente la nouvelle identité sur toutes les pages existantes (Runs, détail d'un run, Kanban, Agents) et deux pages nouvelles : Paramètres › Fournisseurs, Paramètres › Modèles.
7. CI verte, et aucune clé dans les logs, les traces ou les événements.

## Découpage

- **Vague A, en parallèle** :
  - A1 backend configuration (core, auth, API, Terraform/bootstrap) ;
  - A2 exécution multi-fournisseurs (runtime-rig, worker, vector-lancedb) ;
  - A3 nouvelle identité de la console (design system, mise en page, pages existantes).
- **Vague B** : pages Fournisseurs et Modèles dans la nouvelle console, sur l'API de A1.
- **Vérification** : run réel avec une clé Anthropic de l'utilisateur, jeu d'évaluation comparé (7B local contre Claude), avec son accord sur le coût.
