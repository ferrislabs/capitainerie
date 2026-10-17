# Chart Helm Capitainerie

Déploie les composants de Capitainerie : `api` (4100), `mcp-kanban` (4101), `worker` et `exporter` (sans Service) et `console` (nginx, 8080), plus un Job de migrations.

Postgres, Iggy, FerrisKey et le endpoint LLM sont des dépendances externes : le chart ne fait que les référencer par valeurs.

## Images

`ghcr.io/ferrislabs/capitainerie-<composant>` pour `api`, `worker`, `exporter`, `mcp-kanban`, `migrations` et `console`. Le tag par défaut est `.Chart.AppVersion` (publié sur les tags `v*`). En déploiement continu, fixer `<composant>.image.tag` sur `sha-<commit>`, jamais `latest`.

## Secret à créer hors du chart

Le chart ne crée aucune valeur secrète. Créer le Secret `secrets.existingSecret` (par défaut `capitainerie`) par External Secrets, OpenBao ou sealed secret. Chaque clé porte le nom de la variable d'environnement :

| Clé | Utilisée par |
|---|---|
| `DATABASE_URL` | api, worker, mcp-kanban, migrations |
| `DELEGATION_KEY` (`openssl rand -base64 32`, identique pour api et worker) | api, worker |
| `EXCHANGE_CLIENT_ID`, `EXCHANGE_CLIENT_SECRET` | api, worker |
| `AGENT_CREDENTIALS` (`client=secret,...`) | api, worker |
| `IGGY_USERNAME`, `IGGY_PASSWORD` | api, worker, exporter |
| `LLM_API_KEY` (optionnelle) | worker |

Les clés listées dans `secrets.optionalKeys` sont marquées `optional`. La liste lue par chaque composant se règle dans `<composant>.secretEnv`.

## Valeurs principales

| Valeur | Rôle |
|---|---|
| `image.prefix`, `<composant>.image.{repository,tag}` | image des pods |
| `api.allowedOrigins`, `api.acceptedClients`, `api.audience` | `ALLOWED_ORIGINS`, `AUTH_ACCEPTED_CLIENTS`, `AUTH_AUDIENCE` |
| `mcpKanban.publicUrl`, `config.mcpAudience`, `config.pipelineAgents` | `MCP_PUBLIC_URL`, `MCP_AUDIENCE`, `PIPELINE_AGENTS` / `MCP_ACCEPTED_AGENTS` |
| `config.iggy.{address,stream,partitions}` | `IGGY_ADDRESS`, `IGGY_STREAM`, `IGGY_PARTITIONS` |
| `worker.llm.{baseUrl,model,timeoutSecs}`, `worker.concurrency` | `LLM_*`, `WORKER_CONCURRENCY` |
| `config.{logFilter,logJson,otlpEndpoint}` | `LOG_FILTER`, `LOG_JSON`, `OTLP_ENDPOINT` |
| `exporter.{flushEvents,flushSecs}`, `analytics.*` | `ANALYTICS_FLUSH_EVENTS`, `ANALYTICS_FLUSH_SECS`, `ANALYTICS_URI`, volume du dataset Parquet |
| `console.config.*` | `API_URL`, `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`, `OIDC_RESOURCE`, `TRACES_URL` |
| `gateway.*` | HTTPRoutes Gateway API (`/api`, `/mcp` + `/.well-known/oauth-protected-resource`, `/`) |
| `networkPolicy.*` | NetworkPolicy par composant, dépendances externes dans `networkPolicy.external` |

`ALLOWED_ORIGINS`, `AUTH_AUDIENCE`, `MCP_PUBLIC_URL` et les URI de redirection OIDC suivent l'hôte : les changer ensemble.

Le worker n'expose aucun port HTTP, donc aucune sonde. `terminationGracePeriodSeconds` vaut 90 s pour laisser finir le step en cours ; un run interrompu reprend au premier step non terminé.

## Analytics et exporter

L'`exporter` consomme le topic `runs` (groupe `analytics-exporter`) et écrit des fichiers Parquet sous `analytics.mountPath` (`events/organization_id=<uuid>/date=<jour>/part-<uuid>.parquet`). Un seul réplica, stratégie `Recreate` : deux exporters écriraient chacun leur part des événements, et un déploiement progressif en laisserait deux en parallèle. Un SIGTERM vide le tampon avant l'arrêt.

**Attention : par défaut (`analytics.persistence.enabled: false`) le dataset vit dans un `emptyDir`.** Il disparaît quand le pod est replanifié, et l'`api` (pod distinct) n'y a pas accès : les pages coûts et agents restent vides. Pour un usage réel, activer `analytics.persistence` : l'exporter monte le PVC en écriture, l'api le monte en lecture seule, ce qui demande `ReadWriteMany` (ou de placer les deux pods sur le même nœud). Seuls les répertoires locaux sont gérés ; un stockage S3 viendra plus tard.

Les événements sont livrés au moins une fois et les offsets sont validés à la lecture : la déduplication se fait à la requête, sur `event_id`. Un arrêt brutal (SIGKILL) perd au plus les événements du tampon, soit `flushEvents` ou `flushSecs` d'activité.

## Installation

```bash
helm lint deploy/helm/capitainerie -f deploy/helm/capitainerie/values-example.yaml
helm template capitainerie deploy/helm/capitainerie -f values-example.yaml | kubeconform -strict -summary -ignore-missing-schemas
helm install capitainerie oci://ghcr.io/ferrislabs/charts/capitainerie --version <version> -n capitainerie -f values-prod.yaml
```

En production, l'installation passe par GitOps (ArgoCD), pas par `helm install` à la main.

## Migrations et ArgoCD

Le Job `migrations` (et son ServiceAccount) porte `argocd.argoproj.io/hook: PreSync` et `hook-delete-policy: BeforeHookCreation` : il s'exécute avant les nouveaux pods, et le Job précédent est supprimé avant d'être recréé. Le Secret doit exister avant la synchronisation, puisque le hook passe avant le reste de l'Application. Hors ArgoCD, c'est un Job ordinaire à lancer avant le premier démarrage.

Les migrations doivent rester idempotentes et réversibles (`.down.sql`). Une migration qui casse les requêtes de la version précédente se livre en deux versions (expand puis contract).

## Version du chart

Incrémenter `version` dans `Chart.yaml` à chaque changement de template ou de valeur par défaut : le workflow de publication ignore une version déjà publiée.

## Dédoublonnage (worker)

Le worker indexe les cartes dans LanceDB (`worker.vectorStore.uri`) avec le modèle d'embeddings `worker.embedding.model` (par défaut `bge-m3`, à télécharger sur l'endpoint LLM à côté du modèle de chat). Sans persistance, l'index vit dans un `emptyDir` et le worker le reconstruit à chaque démarrage à partir des cartes. Avec `worker.vectorStore.persistence.enabled=true`, le volume est en `ReadWriteOnce` et le déploiement passe en `Recreate` : un seul worker écrit l'index, donc `worker.replicaCount` reste à 1 tant que l'index n'est pas sur un stockage objet.

Une action est un doublon d'une carte dès que leur similarité cosinus atteint `worker.dedupThreshold` (0,75) et une nouvelle carte sous `worker.dedupGrayFloor` (0,65, jamais au-dessus du seuil : le worker refuse de démarrer sinon) ; entre les deux, le modèle de chat juge si l'action et la carte la plus proche sont la même tâche (un appel par action concernée, un échec compte comme « pas un doublon »).
