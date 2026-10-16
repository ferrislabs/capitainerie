# API

`api.client.ts` et `api.tanstack.ts` sont générés par `pnpm gen:api` contre une API en cours d'exécution
(`API_URL`, par défaut `http://localhost:4100/api-docs/openapi.json`). Ne jamais les éditer à la main.
Ils doivent être régénérés après toute modification de l'API.

`api.fetch.ts` expose `fetcher` (type `Fetcher['fetch']` généré) : Bearer depuis le store d'auth, erreurs `ApiRequestError`.
`index.ts` exporte les instances uniques `api` et `tanstackApi`.
