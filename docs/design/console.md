# Console : système de design

Statut : en place depuis la vague A3 (spec `docs/specs/2026-10-providers-and-console.md`, décision P5), refondu le 2026-10-18 d'après le panel Mistral Studio. Code : `apps/console`.

## Direction

La console se lit comme le **registre d'une capitainerie** : une page de papier chaud, une encre presque noire, des colonnes de chiffres alignés. Une seule barre latérale claire porte la navigation, rangée en sections. La couleur ne sert qu'aux états, et elle reste douce.

- Inspiration : les principes du panel Mistral Studio (barre latérale claire à sections, grand titre de page, bouton principal noir, tableaux posés sur le fond). Rien n'est copié : ni code, ni composant, ni ressource.
- Ce que la console refuse : les cartes à ombre douce empilées, l'accent de couleur décoratif, la barre du haut qui répète la navigation.

## Couleurs

Tous les jetons sont des variables CSS en OKLCH dans `src/styles.css` (`:root` pour le clair, `.dark` pour le sombre), exposées à Tailwind par `@theme inline`.

| Rôle | Jeton | Usage |
|---|---|---|
| Fond | `--background` | papier chaud (clair) ou charbon chaud (sombre) |
| Surface | `--card`, `--popover` | panneaux, tableaux, menus |
| Encre | `--foreground`, `--muted-foreground` | texte principal et secondaire |
| Action | `--primary` | encre presque noire en clair, papier en sombre (hors des teintes d’état) |
| Lien | `--link` | liens texte, couleur de l'encre, soulignés au survol |
| Barre latérale | `--rail*` | un ton sous le fond ; `--rail-accent` pour la page active, `--rail-label` pour les titres de section |
| Feu | `--signal-attention` | jaune de bouée cardinale : « Proposé », à décider (réservé à cet usage) |

États, d'après le balisage (IALA) :

| État | Jeton | Sens |
|---|---|---|
| En cours | `--signal-running` | bleu de flot, adouci |
| Terminé, autorisé | `--signal-success` | vert tribord |
| Échec, refusé | `--signal-danger` | rouge bâbord |
| À décider | `--signal-attention` (+ `-ink`, `-soft`, `-foreground`) | jaune cardinal |
| Neutre (en attente, annulé) | `--signal-neutral` | encre adoucie |

Chaque état a une version `-soft` pour les fonds de badge. Les teintes sont toutes chaudes (teinte 75 à 92) et peu saturées ; seuls les états gardent du chroma. Cible WCAG AA dans les deux thèmes : texte principal ≥ 14:1, texte secondaire ≥ 4,5:1, texte d'état sur son fond `-soft` ≥ 4,5:1, bordure de champ ≥ 3:1.

## Typographie

- **Atkinson Hyperlegible Next** (variable 200–800) pour l'interface, **Atkinson Hyperlegible Mono** pour les identifiants, modèles, outils et heures. Paquets `@fontsource-variable/atkinson-hyperlegible-next@5.3.0` et `@fontsource-variable/atkinson-hyperlegible-mono@5.3.0`, versions figées, servis par le bundle (aucune requête vers un CDN).
- Pourquoi : la famille est dessinée par le Braille Institute pour distinguer les glyphes ambigus (`Il1`, `0O`, zéro barré). La console affiche surtout des identifiants, des coûts et des comptes de tokens : la lisibilité de chaque caractère compte plus que le style. Les chiffres tabulaires (`tnum`) sont disponibles, ce qui aligne les colonnes. Licence OFL.
- Échelle : titre de page 24 px semi-gras, titre de section 15 px, corps et navigation 14 px, tableaux 13 px, libellés 12 px, micro-texte 11 px.
- Utilitaire `figures` : chiffres tabulaires et alignés, à poser sur toute valeur numérique en colonne.

## Rythme et forme

- Grille de 4 px. Page : marges 32 px (16 px sous 640 px), 48 px au-dessus du titre, sections espacées de 24 px, contenu aligné à gauche.
- Densité : ligne de tableau 40 px, en-tête de tableau 36 px, lien de navigation 32 px, bouton 32 px (28 px en `sm`).
- Rayons serrés : 6 px par défaut, 4 px pour les badges.
- Séparation par filets d'un pixel, pas par ombres. Seuls les menus et infobulles flottants portent une ombre (`shadow-float`).
- Mouvement : une seule courbe (`ease-out-expo`), 150 à 200 ms. Le point « en cours » pulse doucement. `prefers-reduced-motion` coupe animations et transitions.

## Marque

`src/components/brand/capitainerie-mark.tsx` : un môle (arc ouvert vers le large), son feu (point jaune) et la marée (vague). SVG en ligne, `currentColor` pour le trait. La même forme sert de favicon (`public/icon.svg`).

## Mise en page

- **Barre latérale** (`components/layout/app-rail.tsx`, liens dans `nav-items.ts`) : marque et bouton de repli en haut ; trois sections titrées, « Travail » (Runs, Kanban), « Supervision » (Agents), « Paramètres » (Fournisseurs, Modèles) ; en pied, l'indicateur temps réel et le menu du compte (organisation, utilisateur, thème, « Se déconnecter »). Dépliée (240 px) ou repliée (56 px, infobulles, titres de section remplacés par un filet) ; le choix est gardé dans `localStorage` (`capitainerie.rail-expanded`). Sous 1024 px elle est toujours repliée. Flèches haut/bas, Début et Fin déplacent le focus entre tous les liens. Page active : fond `--rail-accent` et texte en medium.
- **Pas de barre du haut** : la page commence par son titre.
- **Thème** (`lib/theme.ts`, `hooks/use-theme.ts`) : clair, sombre ou système, choisi dans le menu du compte et gardé dans `localStorage` (`capitainerie.theme`). Un script en tête de document applique la classe `dark` avant le premier rendu.

## Primitives

| Composant | Fichier | Rôle |
|---|---|---|
| `Page` | `components/layout/page.tsx` | conteneur de page (largeur max, marges) |
| `PageHeader` | `components/layout/page-header.tsx` | titre, description, actions, méta (badge), lien de retour |
| `SectionCard` | `components/layout/section-card.tsx` | titre et actions sur le fond, contenu dans un cadre ; `flush` retire la marge du cadre (listes), `bare` retire le cadre (tableaux) |
| `Table` | `components/ui/table.tsx` | dense, en-tête collant ; `containerClassName` pour borner la hauteur |
| `StatusBadge` | `components/layout/status-badge.tsx` | pastille + libellé par ton d'état |
| `Badge` | `components/ui/badge.tsx` | variantes `neutral`, `running`, `success`, `danger`, `attention`, `signal`, `outline` |
| `EmptyState` | `components/layout/empty-state.tsx` | icône, titre, phrase, action facultative |
| `TableSkeleton`, `Skeleton` | `components/layout/table-skeleton.tsx`, `components/ui/skeleton.tsx` | chargement ; annonce « Chargement… » aux lecteurs d'écran |
| `InlineAlert` | `components/layout/inline-alert.tsx` | message en ligne `danger` (role alert), `success`, `info` |
| `FullscreenMessage` | `components/layout/fullscreen-message.tsx` | écrans d'authentification et de rappel OIDC |

Correspondances d'état : `domain/runs/status.ts` (statut de run → ton), colonnes Kanban (`Proposé` en jaune avec son compteur, les autres neutres), décisions (bouclier vert autorisé, rouge refusé).

## Le geste propre à Capitainerie

Le détail d'un run est mis en page comme une ligne d'annuaire des marées : heure de début à gauche, barre de latence sur une échelle commune à toutes les étapes du run, durée calée à droite. On voit tout de suite quelle étape a pris le temps.

## Règles

- Une couleur d'état ne sert qu'à porter un état.
- Toute valeur numérique en colonne porte `figures`.
- Les composants `ui/` ne contiennent aucun hook et reçoivent tout par props (CONVENTIONS §14).
- Texte de l'interface en français.
