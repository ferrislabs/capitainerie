# Product

<!-- impeccable:product-schema 1 -->

Scope: the Capitainerie console (`apps/console`). Facts below come from the repository (README, docs/ARCHITECTURE.md, docs/specs/2026-10-providers-and-console.md) and the redesign brief. No interview round was possible in this session; lines marked *(inferred)* were not confirmed by the user.

## Platform

web

## Users

- Members of an organization who drop meeting transcripts and review what the agents propose: they validate or reject proposed Kanban cards, follow a run's steps, and check what it cost.
- Organization admins (FerrisKey role `capitainerie-admin`) who, from wave B, manage model provider keys and pick a model per agent role.
- *(inferred)* Used at a desk, during or right after a work session, in an office or at home; sessions are short and frequent (check proposed cards, look at a failed run).

## Product Purpose

Capitainerie is an open-source platform that orchestrates AI agents. First use case: a meeting transcript goes in, a chain of agents (extraction, deduplication, writing) extracts the actions and proposes them as Kanban cards; a human validates each card before it enters the board. The console is where humans submit, supervise, decide, and audit.

## Positioning

Every agent write is decided by an authorization policy and audited; agents act with delegated tokens; a card always cites the exact transcript passage it comes from; only a human can move a card out of "Proposé". The console makes that human-in-the-loop control and its audit trail visible.

## Operating Context

- Pages: Runs (submit a transcript, list runs), run detail (step timeline, allowed/denied decisions, tokens, cost, trace link), Kanban (five columns: Proposé, À faire, En cours, Terminé, Rejeté), Agents (activity and 30-day costs by agent, user, model, day), Paramètres › Fournisseurs and › Modèles (wave B).
- Realtime updates over WebSocket with connected / degraded / offline states.
- Sign-in through FerrisKey (OIDC).

## Capabilities and Constraints

- Stack: React 19, TanStack Start (SPA) + Router + Query + Form, Tailwind v4 CSS-first, shadcn/ui new-york, lucide-react, Biome, Vitest.
- UI language: French. Front conventions in docs/CONVENTIONS.md §14 (domain/<d>/{hooks,feature,ui}, presentational ui without hooks, kebab-case files).
- License Apache-2.0. Qovery's console (GPL-3.0) is a reference for principles only: nothing copied.
- Fournisseurs and Modèles endpoints are not live yet; their pages are placeholders until wave B.

## Brand Commitments

- Name: Capitainerie (a harbour master's office), orchestrating AI agents.
- Spec decision P5: new visual identity inspired by Qovery's principles: two-level side navigation, dense pages, page headers, careful empty states.

## Evidence on Hand

No customers, testimonials, benchmarks or pricing exist; none may be invented.

## Product Principles

1. The human decides: proposed work is always visibly pending until a person acts.
2. Every action leaves a trace: decisions, costs and steps are shown, never hidden.
3. Density over decoration: operators scan many runs and cards quickly.
4. Calm under failure: errors and degraded states are explicit and give the way out.

## Accessibility & Inclusion

WCAG AA contrast in light and dark themes, full keyboard navigation of rail and menus, visible focus, `prefers-reduced-motion` respected.
