# Capitainerie · Console

Console d'orchestration d'agents IA. React 19, Vite, TanStack Start (mode SPA) et Router, TanStack Query, Tailwind v4, shadcn/ui.

## Commandes

```sh
pnpm install     # installe les dépendances
pnpm dev         # serveur de dev sur le port 4000
pnpm check       # Biome (lint + format)
pnpm test        # Vitest
pnpm build       # build de production (dist/client)
pnpm gen:api     # génère src/api/api.client.ts et api.tanstack.ts
```

## Configuration

- Dev : copier `.env.example` vers `.env` (`VITE_API_URL`, `VITE_OIDC_ISSUER_URL`, `VITE_OIDC_CLIENT_ID`).
- Prod : `/config.json` est rempli au démarrage du conteneur depuis `API_URL`, `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`.

## Structure

`src/domain/<domaine>/pages/{feature,ui}` : `feature` porte données, état et effets ; `ui` est présentationnel (props uniquement).
