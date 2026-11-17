# Spec : refonte de l'interface de la console

Statut : décisions prises le 2026-11-17 à partir de la maquette « Capitainerie » (artifact Claude, version 9). Taille L, livrée en pile de PR (chaque PR repose sur la précédente).

## Objectif

Remplacer l'interface de `apps/console` par celle de la maquette : style compact, barre latérale de 204 px repliable à 48 px, fil d'Ariane, palette de commandes, onglets portés par l'URL. Les pages sans backend sont fournies avec des données de démonstration.

## Décisions

| # | Décision | Option retenue | Option écartée |
|---|---|---|---|
| U1 | Direction visuelle | Compacte de la maquette : Inter et JetBrains Mono, texte 13 px, contrôles 28 px, accent bleu, thème sombre ou clair selon le système | « Harbour Register » (papier chaud, Atkinson) |
| U2 | Fonctionnalités existantes | Gardées : runs, Kanban, clients, projets, agents (coûts), conversations, intégrations, fournisseurs, modèles. Seul l'habillage change | Réécrire les pages sur la maquette |
| U3 | Fonctionnalités absentes du backend | Pages de démonstration : données figées dans `domain/<d>/mock/`, bandeau « Données de démonstration », aucune requête réseau ni écriture | Appeler des endpoints inexistants |
| U4 | Onglets | Un onglet = une URL (segment de chemin), lisible et partageable | État local |
| U5 | Livraison | Pile de PR, une couche par PR, chaque couche passe `check`, `tsc`, `test`, `build` | Une PR unique |

## Glossaire

| Terme | Définition | Synonyme écarté |
|---|---|---|
| Source | Flux d'événements écouté par des agents (DNS, webhook, e-mail, RSS, Git, Odoo). | « trigger » |
| Routine | Travail récurrent qui crée une tâche à chaque passage. | « cron » |
| Connecteur | Canal, compte, outil MCP ou registre qu'un agent peut utiliser, refusé par défaut. | « outil », « canal » pris séparément |
| Démonstration | Donnée ou page sans backend, marquée comme telle. | « fake » |

## Invariants

1. Une page de démonstration n'émet aucune requête réseau et n'écrit rien en base. [test : aucun appel `fetch`]
2. Une page de démonstration affiche le bandeau « Données de démonstration ». [test]
3. Un agent n'a accès qu'aux connecteurs explicitement autorisés ; un sous-agent n'hérite de rien de son parent. [type : liste de noms par agent, test]
4. Contraste WCAG AA dans les deux thèmes, navigation clavier complète de la barre latérale et de la palette, `prefers-reduced-motion` respecté. [test pour la navigation clavier, revue pour le contraste]
5. Les noms de jetons de `styles.css` ne changent pas : les composants existants restent valides. [revue]

## Hors périmètre

- Aucun endpoint, aucune migration, aucun changement du client API généré.
- Pas de persistance des données de démonstration (perdues au rechargement).
- Pas de refonte des flux métier existants (dépôt de transcript, exécution d'une carte, intervenant).

## Pile de PR

| # | Branche | Contenu | Dépend de |
|---|---|---|---|
| 1 | `feat/console-ui-1-tokens` | Jetons, polices, `DESIGN.md`, cette spec | main |
| 2 | `feat/console-ui-2-shell` | Barre latérale, fil d'Ariane, palette ⌘K, kit compact (onglets, panneau, tuile) | 1 |
| 3 | `feat/console-ui-3-pages` | Habillage des pages existantes | 2 |
| 4 | `feat/console-ui-4-agents` | Agents : liste, organigramme, création, fiche à onglets, délégation, connecteurs (démo) | 3 |
| 5 | `feat/console-ui-5-sources` | Sources, pipeline de décision, export CRM (démo) | 4 |
| 6 | `feat/console-ui-6-routines` | Routines (démo) | 5 |
| 7 | `feat/console-ui-7-workspace` | Connecteurs, compétences, approbations, audit, boîte de réception, artifacts (démo) | 6 |
| 8 | `feat/console-ui-8-notice` | Composant de retour d'action partagé, fin des 4 copies locales (#46) | 7 |
| 9 | `feat/console-ui-9-tasks` | Tâches (liste, tableau, page, délégation, nouvelle tâche), liens vers les tâches, entrées de la barre latérale | 8 |
| 10 | `feat/console-ui-10-dashboard` | Tableau de bord, approbations actionnables, demande de validation depuis le pipeline | 9 |

## Acceptance

1. Les deux thèmes utilisent les couleurs de la maquette et la page d'accueil reste lisible (contraste AA).
2. La barre latérale a deux états : étendue (204 px) et rail (48 px, imposé sous 1024 px) ; le lien courant porte `aria-current="page"`.
3. ⌘K ouvre la palette ; flèches et Entrée naviguent ; Échap ferme.
4. Chaque onglet d'une page à onglets est une URL qui s'ouvre directement sur cet onglet.
5. Chaque page de démonstration affiche le bandeau et ne fait aucun appel réseau.
6. Les tests existants passent, hors les 3 échecs déjà présents sur `main` (settings, Intervenant).

Amended: 2026-11-18 pile étendue aux couches 8 à 10 (tâches, tableau de bord, approbations) pour finir la migration de la maquette. Hors périmètre maintenu : la page Projets réelle et le Kanban réel gardent leurs données ; le sélecteur d'organisation (une seule organisation).
Amended: 2026-11-17 pas de tiroir mobile, le rail suffit sous 1024 px (la console reste un outil de bureau).

## Verify (exit)

`cd apps/console && pnpm check && pnpm exec tsc --noEmit && pnpm test && pnpm build`

Les échecs déjà présents sur `main` : `model-assignments-form.test.tsx` (`optionValues` non défini) et `settings-pages.test.tsx` (Intervenant).
