---
name: Capitainerie Console
description: The harbour register. Warm paper, near-black ink, one quiet sectioned sidebar; colour only for state, softened.
colors:
  # Light theme (:root in src/styles.css)
  background: "oklch(0.986 0.004 85)"
  foreground: "oklch(0.21 0.006 75)"
  card: "oklch(0.997 0.002 85)"
  popover: "oklch(0.997 0.002 85)"
  primary: "oklch(0.2 0.006 75)"
  primary-foreground: "oklch(0.985 0.003 85)"
  secondary: "oklch(0.935 0.006 85)"
  muted: "oklch(0.955 0.005 85)"
  muted-foreground: "oklch(0.49 0.01 75)"
  accent: "oklch(0.94 0.006 85)"
  destructive: "oklch(0.52 0.16 27)"
  border: "oklch(0.91 0.006 85)"
  input: "oklch(0.66 0.01 80)"
  ring: "oklch(0.38 0.01 75)"
  link: "oklch(0.21 0.006 75)"
  rail: "oklch(0.972 0.005 85)"
  rail-foreground: "oklch(0.36 0.008 75)"
  rail-foreground-active: "oklch(0.2 0.006 75)"
  rail-label: "oklch(0.5 0.01 75)"
  rail-accent: "oklch(0.925 0.007 85)"
  rail-border: "oklch(0.905 0.006 85)"
  signal-running: "oklch(0.48 0.09 245)"
  signal-running-soft: "oklch(0.95 0.018 245)"
  signal-success: "oklch(0.46 0.08 155)"
  signal-success-soft: "oklch(0.95 0.025 155)"
  signal-danger: "oklch(0.5 0.15 27)"
  signal-danger-soft: "oklch(0.955 0.022 27)"
  signal-attention: "oklch(0.85 0.13 88)"
  signal-attention-ink: "oklch(0.47 0.09 70)"
  signal-attention-foreground: "oklch(0.21 0.006 75)"
  signal-attention-soft: "oklch(0.962 0.04 92)"
  signal-neutral: "oklch(0.49 0.01 75)"
  signal-neutral-soft: "oklch(0.94 0.005 85)"
  # Dark theme (.dark in src/styles.css)
  background-dark: "oklch(0.185 0.004 75)"
  foreground-dark: "oklch(0.94 0.004 85)"
  card-dark: "oklch(0.21 0.004 75)"
  popover-dark: "oklch(0.235 0.005 75)"
  primary-dark: "oklch(0.94 0.004 85)"
  primary-foreground-dark: "oklch(0.2 0.004 75)"
  muted-foreground-dark: "oklch(0.73 0.008 80)"
  border-dark: "oklch(0.295 0.005 75)"
  input-dark: "oklch(0.52 0.008 75)"
  ring-dark: "oklch(0.82 0.006 85)"
  rail-dark: "oklch(0.165 0.004 75)"
  rail-accent-dark: "oklch(0.255 0.005 75)"
  signal-running-dark: "oklch(0.8 0.08 240)"
  signal-success-dark: "oklch(0.8 0.1 158)"
  signal-danger-dark: "oklch(0.77 0.12 25)"
  signal-attention-ink-dark: "oklch(0.86 0.12 90)"
typography:
  headline:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.02em"
  title:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.25
  body:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.45
  dense:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: 1.33
  micro:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1.45
  mono:
    fontFamily: "'Atkinson Hyperlegible Mono Variable', ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.33
    fontFeature: "'tnum' 1, 'lnum' 1"
rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
  full: "9999px"
spacing:
  unit: "4px"
  page-x: "32px"
  page-top: "48px"
  section: "24px"
  table-head: "36px"
  table-row: "40px"
  control: "32px"
  control-sm: "28px"
  rail-link: "32px"
  rail-collapsed: "56px"
  rail-expanded: "240px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "32px"
  button-outline:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "32px"
  input:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    height: "32px"
  badge-neutral:
    backgroundColor: "{colors.signal-neutral-soft}"
    textColor: "{colors.signal-neutral}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  section-frame:
    backgroundColor: "{colors.card}"
    rounded: "{rounded.lg}"
    padding: "16px"
  table-head:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.dense}"
    height: "36px"
  table-cell:
    typography: "{typography.dense}"
    padding: "8px 12px"
    height: "40px"
  rail:
    backgroundColor: "{colors.rail}"
    textColor: "{colors.rail-foreground}"
    width: "240px"
  rail-link-active:
    backgroundColor: "{colors.rail-accent}"
    textColor: "{colors.rail-foreground-active}"
  tooltip:
    backgroundColor: "{colors.foreground}"
    textColor: "{colors.background}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
---

# Design System: Capitainerie Console

## Overview

**Creative North Star: "The Harbour Register"**

The console reads like the ledger kept at a harbour master's office: a warm paper page, near-black ink, ruled lines between entries, figures in tidy columns. One light sidebar, a single tone under the page, carries the navigation in titled sections. The page starts with its own title; there is no top bar repeating what the sidebar already says.

The direction follows the user's brief of 2026-10-18: the principles of Mistral's Studio panel (light sectioned sidebar, warm off-white ground, large page title with a grey description, black primary button, tables laid straight on the ground). Nothing is copied from it.

Colour is spent on state only, and softened: every neutral is warm (hue 75 to 92, chroma under 0.01), and only the signals carry chroma. Both themes are first-class; the dark theme is a warm charcoal, not a blue night.

**Key Characteristics:**
- Warm paper ground, near-black ink, near-black primary button.
- A light sidebar in three titled sections, account and realtime state at its foot.
- Tables sit on the ground with ruled lines; rich lists and forms sit in a thin frame.
- Softened state colours; yellow still means "a human must act".
- Atkinson Hyperlegible for every glyph, tabular figures in every column.

## Colors

### Primary
- **Ink** (`primary`, `foreground`): the primary button and all main text. Paper-white in dark theme.

### Neutral
- **Paper** (`background`) and **Sheet** (`card`, `popover`): the page and the framed surfaces, one step apart.
- **Faded ink** (`muted-foreground`): descriptions, secondary cells, metadata.
- **Rule** (`border`): every separator. **Field stroke** (`input`): form fields, kept at 3:1.
- **Sidebar** (`rail*`): a tone under the page, `rail-accent` for the current page, `rail-label` for section titles.

### Signals
- Running blue, success green, danger red, attention yellow, neutral grey; each with a `-soft` ground for badges. Lower chroma than before so they read as notes in the margin, not as banners.

### Named Rules
**The State-Only Rule.** A signal colour carries a state and nothing else. No accent colour exists for decoration.

**The Yellow Rule.** Solid yellow (`signal-attention`) means a human must act or look: the Proposé column and its count, the degraded realtime dot, the harbour light in the brand mark.

**The Warm Neutral Rule.** No cool grey anywhere. Every neutral token sits at hue 75 to 92.

## Typography

**Body Font:** Atkinson Hyperlegible Next Variable. **Mono:** Atkinson Hyperlegible Mono Variable. Both self-hosted from pinned `@fontsource-variable` packages.

The family separates ambiguous glyphs (`Il1`, `0O`); the console is full of identifiers, costs and token counts, and the user asked to keep it.

- **Headline** (600, 24px, -0.02em): one page title per page.
- **Title** (600, 15px): section titles.
- **Body** (14px): text, inputs, buttons, sidebar links.
- **Dense** (13px): table cells and headers (headers in medium ink), descriptions under section titles.
- **Label** (12px): badges, sidebar section titles, metadata.
- **Micro** (11px): floor size.
- **Mono** (12px): machine strings only (IDs, times, model and tool names).

**The Figures Rule.** Every number in a column carries `figures` (tabular, lining).

## Layout

- **Shell:** sticky full-height sidebar (240px expanded, 56px collapsed; collapsed below 1024px; preference persisted), then the main column. No top bar.
- **Sidebar:** brand and collapse button at the top; sections "Travail", "Supervision", "Paramètres" with 12px labels; links 32px, 14px, 16px icons; footer with the realtime indicator and the account menu (organization, user, theme radio group, sign out). Collapsed: section labels become 1px rules and stay readable to screen readers.
- **Page:** left-aligned, max 90rem (settings 64rem), padding 32px sides and 48px top from 1024px; 24px between blocks.
- **Page header:** 24px title with an optional inline badge, 14px grey description, actions right. No rule under it.
- **Section:** title, description and actions sit on the ground; the content follows. `bare` for tables (no frame), `flush` for row lists (frame, no padding), default framed with 16px padding.

## Elevation & Depth

Flat. Depth is the paper/sheet step and 1px rules. Only floating layers (menus, tooltips, dialogs) carry `shadow-float`, a warm two-layer shadow.

## Shapes

6px on buttons, inputs, sidebar links; 8px on frames, columns, menus; 4px on badges. The empty state is a sheet-coloured card in a dashed field-stroke frame.

## Components

- **Buttons:** primary near-black, outline on sheet with a light stroke, ghost muted. 32px (28px small).
- **Tables:** header 36px in 13px medium ink on the page ground, rows 40px ruled by `border`, last row ruled too, hover tints with `muted`.
- **Badges:** soft ground, full-tone text, 4px radius.
- **Sidebar link:** hover `rail-accent` at 60%, current page `rail-accent` with medium weight.
- **Account menu:** trigger shows initials in a framed square, organization name, user name; the menu holds the profile, the theme choice and sign out.

## Cartes

A card is a block that leads somewhere or holds one fact: the home figures, the settings entries.

- **Surface:** `card` on the page ground, 8px radius, 1px `border`. No shadow at rest, ever.
- **Hover:** the surface moves to `muted`, nothing else (no lift, no border change). A card that is a link is one `<a>` as a whole, with the 2px `ring` outline on keyboard focus.
- **Content:** a 13px `muted-foreground` title with a 16px icon, one figure or state in `headline`-weight ink (`figures` on numbers), one 13px hint in `muted-foreground`. Padding 16px, 8px between lines.
- **Colour:** a card carries a signal only for a state, paired with a word. The count of proposed cards uses solid yellow when above zero (the Yellow Rule) and plain ink at zero.
- **Grid:** one column on small screens, two from 640px, as many as the content needs on wide ones (four on home, three in settings), 12px gap.
- **Empty state:** the same surface with a dashed `input` frame, an icon, a title, one sentence and a single action that fixes the emptiness.
- **Kanban card:** three zones, title, meta (description, quote, chips, date), footer (actions, only when there is one), 8px apart.
- **Kanban card details:** in the meta zone, one row of signals. A priority badge only for urgent (danger) and high (neutral), always a word and an icon. A due date with a calendar icon, in `signal-danger` and followed by « en retard » once the day has passed. Criteria progress as `n/m`. Two labels then `+n`. The assignee's initials in a 20px square at the end of the row.

## Barre d'outils de filtres

The row above a table: filter chips, then a full-width search field.

- **Chip:** 32px, 6px radius, 13px medium. Empty: dashed `input` border, `muted-foreground` text. Filled: solid border, `card` ground, ink text, followed by the value or the count of choices.
- **Menu:** a popover of checkboxes (several values allowed, the menu stays open on each choice), a separator and « Effacer » once something is chosen. Values within a chip are combined with "or", chips with "and".
- **Search:** fills the rest of the row (min 12rem), leading search icon, matches regardless of case and accents, labelled for screen readers.
- **Empty result:** an empty-state card with a « Réinitialiser les filtres » action. Options offered are the values present in the data.
- **Wrap:** the row wraps on narrow screens; the search field keeps its minimum width.

## Formulaires

> Inspiré des principes de la console Qovery (GPL-3.0) ; aucun code repris.

Le kit vit dans `src/components/ui`. La revue visuelle se fait sur `/_dev/ui` (mode développement seulement), qui montre chaque composant dans tous ses états.

- **Champ (`Field`):** libellé au-dessus (13 px, medium), puis le contrôle, puis l'aide (`muted-foreground`, 12 px), puis l'erreur. Marqueur « * » avec « (obligatoire) » pour les lecteurs d'écran, ou « (facultatif) » en gris. Le compteur de caractères est à droite du libellé et passe en `signal-danger` avec « · trop long » quand la limite est dépassée. Le libellé est relié au contrôle, l'aide et l'erreur par `aria-describedby`, l'état par `aria-invalid`. Un groupe (boutons radio) prend `group` et se nomme par `aria-labelledby`.
- **Un champ, un contrôle:** un `Field` enveloppe exactement un contrôle. Si le contrôle porte son propre `id`, il l'emporte sur celui du `Field`. Les listes de cases à cocher et les boutons radio utilisent le mode `group` (`fieldset` et `legend`). Les erreurs vivent dans une zone `aria-live="polite"` toujours présente; un message qui en remplace un autre attend 700 ms avant d'être annoncé, pour ne pas parler à chaque frappe.
- **Boîte de saisie:** 32 px (`md`) ou 28 px (`sm`), rayon 6 px, trait `input`, fond `card`. Le focus remplace le trait par `ring` et ajoute un halo de 3 px à 25 %. L'erreur passe le trait en `destructive` et s'écrit toujours avec un mot et une icône sous le champ (State-Only Rule). Désactivé: 50 % d'opacité, curseur interdit.
- **Icônes et actions:** icône de début et de fin à 16 px en `muted-foreground`; bouton d'effacement de 20 px, atteignable au clavier, visible seulement quand le champ est rempli et modifiable; chargement par un indicateur tournant doublé du texte « Chargement… » pour les lecteurs d'écran.
- **Zone de texte:** hauteur libre ou croissante avec le contenu, plafonnée par `maxRows` (défilement au-delà); compteur optionnel.
- **Select (un bouton) et Combobox (un champ):** le Select est un bouton `role="combobox"`; le Combobox est un champ de saisie qui filtre sans tenir compte de la casse ni des accents. Les deux ouvrent la même liste (`role="listbox"`) dans une surface flottante (`popover`, `shadow-float`, rayon 8 px) large comme le champ, avec options groupées sous un titre 12 px, texte vide (« Aucun résultat »), état de chargement et option « Aucun » facultative. La sélection multiple montre des puces de 20 px retirables (bouton « Retirer … ») dans le champ; la liste reste ouverte à chaque choix.
- **Clavier des listes:** flèches pour naviguer (en boucle, options désactivées sautées), Entrée pour choisir, Échap pour fermer, Tab pour quitter. Pour le Select, Home et End vont aux extrémités et la frappe de lettres saute à l'option qui commence par elles. Home et End ne servent à la liste que lorsqu'elle est ouverte (et, pour le Combobox, que le champ est vide); Retour arrière retire la dernière puce; flèche gauche depuis un champ vide passe aux puces, flèches se déplacent entre puces, Retour arrière ou Suppr retire la puce active. Entrée ne soumet pas le formulaire tant que la liste est ouverte.
- **Menu déroulant:** sections avec titre 12 px, icônes de 16 px, raccourci clavier à droite en 12 px, sous-menus avec chevron, cases à cocher. Une action destructive est écrite en `signal-danger` et son libellé dit l'action (« Supprimer »), jamais la couleur seule.
- **Cases, interrupteur, boutons radio:** 16 px (interrupteur 36 x 20 px), même focus que les champs, coché en `primary`. Libellé et description cliquables, reliés par `aria-describedby`.
- **Date:** le champ natif `type="date"` dans la même boîte, avec `min`, `max` et un bouton « Effacer la date ». Pas de bibliothèque de calendrier.
- **Mouvement:** les transitions de 150 ms et l'ouverture des listes sont neutralisées par `prefers-reduced-motion`.

## Do's and Don'ts

### Do:
- **Do** take every colour from `src/styles.css` and define new tokens in both themes.
- **Do** lay tables straight on the ground with `SectionCard bare`.
- **Do** put `figures` on every number in a column and mono on machine strings.
- **Do** pair a state colour with a word or an icon.

### Don't:
- **Don't** reintroduce a top bar or a secondary settings column.
- **Don't** use a cool grey or a decorative accent colour.
- **Don't** put shadows on resting surfaces.
- **Don't** use yellow for anything that does not need a human.
- **Don't** go below 11px or add a second type family.
