# API

`api.client.ts` et `api.tanstack.ts` sont générés par `pnpm gen:api` depuis l'OpenAPI de l'API
(`API_URL`, par défaut `http://localhost:4100/api-docs/openapi.json`). Ne jamais les éditer à la main.

En attendant, `api.fetch.ts` expose `fetcher` (compatible avec le `Fetcher` généré) et `apiRequest`.
