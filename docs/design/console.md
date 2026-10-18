# Console : système de design

Statut : en place depuis la vague A3 (spec `docs/specs/2026-10-providers-and-console.md`, décision P5). Code : `apps/console`.

## Direction

La console se lit comme un **annuaire des marées** : des colonnes de chiffres alignés, des heures à gauche, des durées calées à droite, pour comparer d'un coup d'œil. La capitainerie fournit le reste : un bandeau marine pour la navigation, et les couleurs du balisage maritime pour les états.

- Ce que la console refuse : le tableau de bord gris avec un accent unique et des cartes à ombre douce.
- Ce que la console garde des principes de la console Qovery (aucun code, composant, icône ni texte repris) : navigation latérale à deux niveaux, pages denses, en-tête de page, états vides soignés.

## Couleurs

Tous les jetons sont des variables CSS en OKLCH dans `src/styles.css` (`:root` pour le clair, `.dark` pour le sombre), exposées à Tailwind par `@theme inline`.

| Rôle | Jeton | Usage |
|---|---|---|
| Fond | `--background` | brume de mer (clair) ou nuit de port (sombre) |
| Surface | `--card`, `--popover` | panneaux, tableaux, menus |
| Encre | `--foreground`, `--muted-foreground` | texte principal et secondaire |
| Action | `--primary` | marine en clair, blanc d’écume en sombre (hors des teintes d’état) |
| Lien | `--link` | liens texte |
| Bandeau | `--rail*` | barre latérale, marine dans les deux thèmes |
| Repère | `--rail-indicator` | blanc d’écume : page active dans le bandeau |
| Feu | `--signal-attention` | jaune de bouée cardinale : « Proposé », à décider (réservé à cet usage) |

États, d'après le balisage (IALA) :

| État | Jeton | Sens |
|---|---|---|
| En cours | `--signal-running` | bleu de flot |
| Terminé, autorisé | `--signal-success` | vert tribord |
| Échec, refusé | `--signal-danger` | rouge bâbord |
| À décider | `--signal-attention` (+ `-ink`, `-soft`, `-foreground`) | jaune cardinal |
| Neutre (en attente, annulé) | `--signal-neutral` | encre adoucie |

Chaque état a une version `-soft` pour les fonds de badge. Contrastes vérifiés (WCAG AA) dans les deux thèmes : texte principal ≥ 14:1, texte secondaire ≥ 5,8:1, texte d'état sur son fond `-soft` ≥ 5,3:1, bordure de champ ≥ 3:1.

## Typographie

- **Atkinson Hyperlegible Next** (variable 200–800) pour l'interface, **Atkinson Hyperlegible Mono** pour les identifiants, modèles, outils et heures. Paquets `@fontsource-variable/atkinson-hyperlegible-next@5.3.0` et `@fontsource-variable/atkinson-hyperlegible-mono@5.3.0`, versions figées, servis par le bundle (aucune requête vers un CDN).
- Pourquoi : la famille est dessinée par le Braille Institute pour distinguer les glyphes ambigus (`Il1`, `0O`, zéro barré). La console affiche surtout des identifiants, des coûts et des comptes de tokens : la lisibilité de chaque caractère compte plus que le style. Les chiffres tabulaires (`tnum`) sont disponibles, ce qui aligne les colonnes. Licence OFL.
- Échelle : titre de page 22 px semi-gras, titre de section 15 px, corps 14 px, tableaux 13 px, libellés 12 px, micro-texte 11 px.
- Utilitaire `figures` : chiffres tabulaires et alignés, à poser sur toute valeur numérique en colonne.

## Rythme et forme

- Grille de 4 px. Page : marges 24 px, sections espacées de 20 px. Panneau : en-tête 12 × 16 px, contenu 16 px.
- Densité : ligne de tableau 36 px, en-tête de tableau 32 px, bouton 32 px (28 px en `sm`).
- Rayons serrés : 6 px par défaut, 4 px pour les badges.
- Séparation par filets d'un pixel, pas par ombres. Seuls les menus et infobulles flottants portent une ombre (`shadow-float`).
- Mouvement : une seule courbe (`ease-out-expo`), 150 à 200 ms. Le point « en cours » pulse doucement. `prefers-reduced-motion` coupe animations et transitions.

## Marque

`src/components/brand/capitainerie-mark.tsx` : un môle (arc ouvert vers le large), son feu (point jaune) et la marée (vague). SVG en ligne, `currentColor` pour le trait. La même forme sert de favicon (`public/icon.svg`).

## Mise en page

- **Bandeau** (`components/layout/app-rail.tsx`) : marque, Runs, Kanban, Agents, puis Paramètres en pied. Déplié (208 px, libellés) ou replié (56 px, infobulles) ; le choix est gardé dans `localStorage` (`capitainerie.rail-expanded`). Sous 1024 px il est toujours replié. Flèches haut/bas, Début et Fin déplacent le focus entre les liens. Page active : fond plus clair et trait blanc d’écume ; le jaune reste réservé à ce qui attend une décision humaine.
- **Barre du haut** (`top-bar.tsx`) : nom de l'organisation (`GET /api/v1/me`), indicateur temps réel, sélecteur de thème, menu du compte (profil, « Se déconnecter »).
- **Navigation secondaire** (`settings-nav.tsx`) : colonne de 208 px pour Paramètres › Fournisseurs, Modèles ; onglets horizontaux sous 1024 px.
- **Thème** (`lib/theme.ts`, `hooks/use-theme.ts`) : clair, sombre ou système, gardé dans `localStorage` (`capitainerie.theme`). Un script en tête de document applique la classe `dark` avant le premier rendu.

## Primitives

| Composant | Fichier | Rôle |
|---|---|---|
| `Page` | `components/layout/page.tsx` | conteneur de page (largeur max, marges) |
| `PageHeader` | `components/layout/page-header.tsx` | titre, description, actions, méta (badge), lien de retour |
| `SectionCard` | `components/layout/section-card.tsx` | panneau titré ; `flush` pour un tableau bord à bord |
| `Table` | `components/ui/table.tsx` | dense, en-tête collant ; `containerClassName` pour borner la hauteur |
| `StatusBadge` | `components/layout/status-badge.tsx` | pastille + libellé par ton d'état |
| `Badge` | `components/ui/badge.tsx` | variantes `neutral`, `running`, `success`, `danger`, `attention`, `signal`, `outline` |
| `EmptyState` | `components/layout/empty-state.tsx` | icône, titre, phrase, action facultative |
| `TableSkeleton`, `Skeleton` | `components/layout/table-skeleton.tsx`, `components/ui/skeleton.tsx` | chargement ; annonce « Chargement… » aux lecteurs d'écran |
| `InlineAlert` | `components/layout/inline-alert.tsx` | message en ligne `danger` (role alert), `success`, `info` |
| `FullscreenMessage` | `components/layout/fullscreen-message.tsx` | écrans d'authentification et de rappel OIDC |
| `ThemeToggle` | `components/layout/theme-toggle.tsx` | trois boutons à bascule |

Correspondances d'état : `domain/runs/status.ts` (statut de run → ton), colonnes Kanban (`Proposé` en jaune avec son compteur, les autres neutres), décisions (bouclier vert autorisé, rouge refusé).

## Le geste propre à Capitainerie

Le détail d'un run est mis en page comme une ligne d'annuaire des marées : heure de début à gauche, barre de latence sur une échelle commune à toutes les étapes du run, durée calée à droite. On voit tout de suite quelle étape a pris le temps.

## Règles

- Une couleur d'état ne sert qu'à porter un état.
- Toute valeur numérique en colonne porte `figures`.
- Les composants `ui/` ne contiennent aucun hook et reçoivent tout par props (CONVENTIONS §14).
- Texte de l'interface en français.
