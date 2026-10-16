# Chart Helm Capitainerie

Déploie les quatre composants de Capitainerie : `api` (4100), `mcp-kanban` (4101), `worker` (sans Service) et `console` (nginx, 8080), plus un Job de migrations.

Postgres, Iggy, FerrisKey et le endpoint LLM sont des dépendances externes : le chart ne fait que les référencer par valeurs.

## Images

`ghcr.io/ferrislabs/capitainerie-<composant>` pour `api`, `worker`, `mcp-kanban`, `migrations` et `console`. Le tag par défaut est `.Chart.AppVersion` (publié sur les tags `v*`). En déploiement continu, fixer `<composant>.image.tag` sur `sha-<commit>`, jamais `latest`.

## Secret à créer hors du chart

Le chart ne crée aucune valeur secrète. Créer le Secret `secrets.existingSecret` (par défaut `capitainerie`) par External Secrets, OpenBao ou sealed secret. Chaque clé porte le nom de la variable d'environnement :

| Clé | Utilisée par |
|---|---|
| `DATABASE_URL` | api, worker, mcp-kanban, migrations |
| `DELEGATION_KEY` (`openssl rand -base64 32`, identique pour api et worker) | api, worker |
| `EXCHANGE_CLIENT_ID`, `EXCHANGE_CLIENT_SECRET` | api, worker |
| `AGENT_CREDENTIALS` (`client=secret,...`) | api, worker |
| `IGGY_USERNAME`, `IGGY_PASSWORD` | api, worker |
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
| `console.config.*` | `API_URL`, `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`, `OIDC_RESOURCE`, `TRACES_URL` |
| `gateway.*` | HTTPRoutes Gateway API (`/api`, `/mcp` + `/.well-known/oauth-protected-resource`, `/`) |
| `networkPolicy.*` | NetworkPolicy par composant, dépendances externes dans `networkPolicy.external` |

`ALLOWED_ORIGINS`, `AUTH_AUDIENCE`, `MCP_PUBLIC_URL` et les URI de redirection OIDC suivent l'hôte : les changer ensemble.

Le worker n'expose aucun port HTTP, donc aucune sonde. `terminationGracePeriodSeconds` vaut 90 s pour laisser finir le step en cours ; un run interrompu reprend au premier step non terminé.

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
