---
name: Capitainerie Console
description: A tide almanac for AI agents. Dense aligned figures on a marine navy shell, buoyage signals for state.
colors:
  # Light theme (:root in src/styles.css)
  background: "oklch(0.975 0.006 235)"
  foreground: "oklch(0.24 0.045 258)"
  card: "oklch(0.995 0.002 235)"
  popover: "oklch(0.995 0.002 235)"
  primary: "oklch(0.33 0.08 258)"
  primary-foreground: "oklch(0.985 0.005 235)"
  secondary: "oklch(0.935 0.018 235)"
  secondary-foreground: "oklch(0.3 0.07 258)"
  muted: "oklch(0.948 0.01 235)"
  muted-foreground: "oklch(0.47 0.03 252)"
  accent: "oklch(0.935 0.02 232)"
  accent-foreground: "oklch(0.28 0.07 258)"
  destructive: "oklch(0.52 0.19 27)"
  border: "oklch(0.895 0.012 240)"
  input: "oklch(0.66 0.03 245)"
  ring: "oklch(0.55 0.13 238)"
  link: "oklch(0.45 0.13 245)"
  rail: "oklch(0.255 0.06 258)"
  rail-foreground: "oklch(0.83 0.03 245)"
  rail-foreground-active: "oklch(0.985 0.008 240)"
  rail-accent: "oklch(0.335 0.07 256)"
  rail-border: "oklch(0.32 0.055 258)"
  rail-indicator: "oklch(0.93 0.03 230)"
  signal-running: "oklch(0.47 0.13 245)"
  signal-running-soft: "oklch(0.935 0.035 240)"
  signal-success: "oklch(0.45 0.11 158)"
  signal-success-soft: "oklch(0.935 0.045 160)"
  signal-danger: "oklch(0.51 0.19 27)"
  signal-danger-soft: "oklch(0.945 0.03 25)"
  signal-attention: "oklch(0.86 0.155 90)"
  signal-attention-ink: "oklch(0.47 0.1 70)"
  signal-attention-foreground: "oklch(0.24 0.045 258)"
  signal-attention-soft: "oklch(0.955 0.05 95)"
  signal-neutral: "oklch(0.47 0.03 252)"
  signal-neutral-soft: "oklch(0.935 0.012 240)"
  # Dark theme (.dark in src/styles.css)
  background-dark: "oklch(0.175 0.028 258)"
  foreground-dark: "oklch(0.94 0.012 240)"
  card-dark: "oklch(0.21 0.032 258)"
  popover-dark: "oklch(0.23 0.034 258)"
  primary-dark: "oklch(0.93 0.015 240)"
  primary-foreground-dark: "oklch(0.2 0.04 258)"
  secondary-dark: "oklch(0.27 0.035 258)"
  secondary-foreground-dark: "oklch(0.92 0.02 240)"
  muted-dark: "oklch(0.25 0.03 258)"
  muted-foreground-dark: "oklch(0.74 0.025 245)"
  accent-dark: "oklch(0.28 0.04 255)"
  accent-foreground-dark: "oklch(0.94 0.015 240)"
  destructive-dark: "oklch(0.72 0.16 25)"
  border-dark: "oklch(0.31 0.035 258)"
  input-dark: "oklch(0.52 0.035 255)"
  ring-dark: "oklch(0.74 0.12 230)"
  link-dark: "oklch(0.8 0.1 230)"
  rail-dark: "oklch(0.14 0.03 258)"
  rail-foreground-dark: "oklch(0.76 0.03 245)"
  rail-foreground-active-dark: "oklch(0.97 0.01 240)"
  rail-accent-dark: "oklch(0.24 0.045 256)"
  rail-border-dark: "oklch(0.25 0.035 258)"
  rail-indicator-dark: "oklch(0.93 0.03 230)"
  signal-running-dark: "oklch(0.8 0.1 232)"
  signal-running-soft-dark: "oklch(0.3 0.065 245)"
  signal-success-dark: "oklch(0.8 0.13 158)"
  signal-success-soft-dark: "oklch(0.29 0.055 162)"
  signal-danger-dark: "oklch(0.77 0.14 25)"
  signal-danger-soft-dark: "oklch(0.3 0.075 25)"
  signal-attention-dark: "oklch(0.86 0.155 90)"
  signal-attention-ink-dark: "oklch(0.87 0.14 90)"
  signal-attention-soft-dark: "oklch(0.31 0.06 85)"
  signal-neutral-dark: "oklch(0.76 0.025 245)"
  signal-neutral-soft-dark: "oklch(0.28 0.03 258)"
typography:
  headline:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.375rem"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.015em"
  figure:
    fontFamily: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 600
    lineHeight: 1.25
    fontFeature: "'tnum' 1, 'lnum' 1"
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
  xl: "10px"
  full: "9999px"
spacing:
  unit: "4px"
  xs: "8px"
  sm: "12px"
  md: "16px"
  section: "20px"
  page: "24px"
  table-head: "32px"
  table-row: "36px"
  control: "32px"
  control-sm: "28px"
  top-bar: "48px"
  rail-collapsed: "56px"
  rail-expanded: "208px"
  secondary-nav: "208px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "32px"
  button-primary-hover:
    backgroundColor: "color-mix(in oklch, oklch(0.33 0.08 258) 88%, transparent)"
  button-outline:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "32px"
  button-outline-hover:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-foreground}"
  button-ghost:
    textColor: "{colors.muted-foreground}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "32px"
  button-ghost-hover:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-foreground}"
  button-sm:
    typography: "{typography.dense}"
    rounded: "{rounded.md}"
    padding: "0 10px"
    height: "28px"
  input:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "4px 12px"
    height: "32px"
  badge-running:
    backgroundColor: "{colors.signal-running-soft}"
    textColor: "{colors.signal-running}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  badge-success:
    backgroundColor: "{colors.signal-success-soft}"
    textColor: "{colors.signal-success}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  badge-danger:
    backgroundColor: "{colors.signal-danger-soft}"
    textColor: "{colors.signal-danger}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  badge-attention:
    backgroundColor: "{colors.signal-attention-soft}"
    textColor: "{colors.signal-attention-ink}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  badge-signal:
    backgroundColor: "{colors.signal-attention}"
    textColor: "{colors.signal-attention-foreground}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  badge-neutral:
    backgroundColor: "{colors.signal-neutral-soft}"
    textColor: "{colors.signal-neutral}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "1px 6px"
  section-card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "16px"
  section-card-header:
    typography: "{typography.title}"
    padding: "12px 16px"
  table-head:
    backgroundColor: "{colors.card}"
    textColor: "{colors.muted-foreground}"
    typography: "{typography.label}"
    padding: "0 12px"
    height: "32px"
  table-cell:
    typography: "{typography.dense}"
    padding: "6px 12px"
    height: "36px"
  rail:
    backgroundColor: "{colors.rail}"
    textColor: "{colors.rail-foreground}"
    padding: "12px 8px"
    width: "208px"
  rail-link:
    textColor: "{colors.rail-foreground}"
    typography: "{typography.dense}"
    rounded: "{rounded.md}"
    padding: "0 10px"
    height: "36px"
  rail-link-active:
    backgroundColor: "{colors.rail-accent}"
    textColor: "{colors.rail-foreground-active}"
  top-bar:
    backgroundColor: "{colors.background}"
    padding: "0 24px"
    height: "48px"
  kanban-column-proposed:
    backgroundColor: "{colors.signal-attention-soft}"
    rounded: "{rounded.lg}"
  kanban-card:
    backgroundColor: "{colors.card}"
    rounded: "{rounded.md}"
    padding: "12px"
  tooltip:
    backgroundColor: "{colors.foreground}"
    textColor: "{colors.background}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "4px 8px"
---

# Design System: Capitainerie Console

## Overview

**Creative North Star: "The Tide Almanac"**

The console reads like an annuaire des marées: columns of aligned figures, clock times on the left, durations set tight to the right margin, so one glance compares rows. The harbour master's office supplies the rest of the world: a committed marine navy rail as the shell, a sea-mist ground in light theme and a night-harbour ground in dark, ink-navy type, and the colours of maritime buoyage (IALA) carrying state and nothing else.

Density is the material. Rows are 36px, controls 32px, radii tight (6px), and surfaces are separated by one-pixel hairline seams rather than shadows. Hierarchy comes from scale contrast and weight inside a single highly legible family (Atkinson Hyperlegible), not from colour or decoration. The world refuses the grey SaaS dashboard with one accent colour and soft shadowed cards.

Both themes are first-class and every token exists in both. Contrast is held to WCAG AA in each: primary text at or above 14:1, secondary text at or above 5.8:1, state text on its soft ground at or above 5.3:1, field borders at or above 3:1.

**Key Characteristics:**
- Marine navy rail in both themes; the content ground changes, the shell does not.
- Buoyage signals: flood blue running, starboard green allowed/succeeded, port red denied/failed, cardinal yellow awaiting a human.
- Hairline seams, flat surfaces; only floating layers cast a shadow.
- Tabular, lining figures on every number that sits in a column.
- One easing curve, 150 to 200ms, all motion cut under reduced-motion.

## Colors

A restrained navy-and-mist palette in OKLCH (hue 230 to 258) where the only saturated colour is signal, and every signal means a state.

### Primary
- **Harbour Navy** (`primary`, dark: `primary-dark`): the action colour. Deep navy in light theme, sea-foam white in dark, so the primary button never borrows a state hue. Used on the primary button, the user avatar chip, and selected text.
- **Marine Rail** (`rail`, dark: `rail-dark`): the committed shell colour of the left rail, navy in both themes, with its own foreground, hover (`rail-accent`) and seam (`rail-border`) tokens.
- **Sea-Foam Marker** (`rail-indicator`): the 3px bar on the active rail item. Pale on purpose, so yellow stays free for decisions.

### Secondary
- **Flood Blue** (`signal-running`, soft `signal-running-soft`): a run or step in progress; also the info tone of inline alerts.
- **Starboard Green** (`signal-success`, soft `signal-success-soft`): succeeded steps, allowed authorization decisions, connected realtime.
- **Port Red** (`signal-danger`, soft `signal-danger-soft`): failed runs and steps, denied decisions, error alerts, the reject action.

### Tertiary
- **Cardinal Yellow** (`signal-attention`, with `signal-attention-ink` for text, `signal-attention-soft` for grounds, `signal-attention-foreground` for text on the solid yellow): what needs a human. The Proposé column, its count badge, the degraded realtime dot, and the harbour light in the brand mark.

### Neutral
- **Sea Mist / Night Harbour** (`background`, dark: `background-dark`): page ground.
- **Chart Paper** (`card`, `popover`): panels, tables, menus, inputs. In dark theme the popover sits one step lighter than the card.
- **Ink Navy** (`foreground`) and **Softened Ink** (`muted-foreground`): primary and secondary text; `muted-foreground` also doubles as `signal-neutral` for queued and cancelled.
- **Shoal** (`secondary`, `muted`, `accent`): pressed toggles, hover fills, quiet chips, skeletons.
- **Hairline** (`border`): every seam. **Field Stroke** (`input`): the darker stroke that keeps form fields at 3:1.
- **Ring Blue** (`ring`): focus outlines, caret, and text selection (at 30% mix). **Chart Link** (`link`): inline text links.

### Named Rules
**The Buoyage Rule.** A signal colour carries a state and nothing else. Never use green, red, blue or yellow signal tokens for decoration, emphasis, or branding.

**The Cardinal Light Rule.** Solid yellow (`signal-attention`) means "a human must act or look". It marks the Proposé column and its count, the degraded realtime state, and the harbour light in the brand mark; the active rail item uses sea-foam white instead.

**The Soft Ground Rule.** A state shown as a filled shape uses its `-soft` token as ground and its full token (or `-ink` for yellow) as text, never the reverse, except the solid `signal` count badge.

## Typography

**Display Font:** none. The page title is the largest type.
**Body Font:** Atkinson Hyperlegible Next Variable (with ui-sans-serif, system-ui)
**Label/Mono Font:** Atkinson Hyperlegible Mono Variable (with ui-monospace, SFMono-Regular, Menlo)

**Character:** A family drawn by the Braille Institute to separate ambiguous glyphs (`Il1`, `0O`, slashed zero). The console mostly shows identifiers, costs and token counts, so per-character legibility outranks style. Both faces ship self-hosted from pinned `@fontsource-variable` packages; no CDN.

### Hierarchy
- **Headline** (600, 22px / 1.375rem, 1.25, -0.015em): page titles in the page header, one per page.
- **Figure** (600, 18px, 1.25, tabular): the totals strip values on run detail.
- **Title** (600, 15px, 1.25): section card headings, settings nav title, wordmark (700, -0.01em).
- **Body** (400, 14px, 1.45): base text, inputs, menu items, buttons.
- **Dense** (400 or 500, 13px): table cells, rail and settings nav links, page descriptions (max 70ch), Kanban card titles, alerts.
- **Label** (500, 12px): table headers, badges, section descriptions, metadata lines, tooltips.
- **Micro** (11px): Kanban card dates, provenance caption, agent chips. Floor size; never smaller.
- **Mono** (400, 12px): clock times, run and card IDs, tool names, agent and model identifiers; 13px in the transcript textarea.

### Named Rules
**The Figures Rule.** Every numeric value that sits in a column or is compared across rows carries tabular lining numerals (`figures` utility: `tnum`, `lnum`). Badges carry it by default.

**The Mono-for-Machines Rule.** Mono is for strings a machine produced (IDs, timestamps, tool and model names). Human prose and labels stay in the sans.

**The Sentence Case Rule.** All labels, headers and table headings are sentence case, in French, at normal tracking. Hierarchy is scale and weight.

## Layout

- **Shell:** a sticky full-height rail on the left (208px expanded with labels, 56px collapsed with tooltips; preference persisted; always collapsed below 1024px), then a column with a sticky 48px top bar (organization name left; realtime indicator, theme toggle, user menu right) over the main content.
- **Page:** max width 90rem, centered; padding 24px (16px / 20px vertical below 640px); 20px gap between page blocks.
- **Page header:** title with optional status badge inline, one-line description below, actions right-aligned; closed by a hairline with 20px bottom padding.
- **Section card:** header 12px x 16px with a hairline below, content 16px, or flush (no padding) when it holds a table or a list of rows.
- **Settings:** a 208px secondary nav column, sticky under the top bar; becomes a horizontal scrolling tab row below 1024px.
- **Runs:** submit form on the left third, the runs table filling the rest, sticky header.
- **Kanban:** five columns in a horizontal grid at 1024px and above (min 12rem each), stacked below; columns scroll internally.
- **Run detail:** a four-cell totals strip (two cells below 640px) built from hairline gaps, then the steps and decisions cards, side by side at 1536px and above.
- **Rhythm:** a 4px grid. Common steps: 8px inside controls, 12px between related items, 16px panel padding, 20px between sections, 24px page margins.

## Elevation & Depth

The system is flat. Depth is conveyed by tonal steps (background, card, popover) and by one-pixel hairline seams (`border`). Grouped figures sit on a `border`-coloured ground with 1px gaps, so the seams are the background showing through. Only layers that float above the page cast a shadow. The sticky top bar uses a translucent ground (80 to 92% background) with a small backdrop blur so rows scrolling beneath stay legible without a shadow line.

### Shadow Vocabulary
- **Float** (`box-shadow: 0 1px 2px oklch(0.2 0.04 258 / 0.08), 0 8px 24px -6px oklch(0.2 0.04 258 / 0.18)`): dropdown menus and tooltips only.

### Named Rules
**The Hairline Rule.** Separate with a 1px `border` seam, never a shadow. A resting panel, table, card or column has no shadow.

## Shapes

Tight, almost architectural corners from a 6px base: 4px on badges, chips, toggle segments and the provenance quote; 6px on buttons, inputs, rail links, Kanban cards, tooltips and alerts; 8px on section cards, Kanban columns, menus and the totals strip. Full rounding is kept for status dots, the active rail marker and latency bars. Borders are 1px everywhere; the empty state uses a dashed field-stroke border to read as "nothing here yet".

The brand mark is a stroked open arc (the mole, opening to sea), a wave inside it, and a yellow dot (the harbour light), drawn in `currentColor` with round caps at 2.2 to 2.6 stroke on a 32px grid. The same shape is the favicon.

## Components

### Buttons
Compact and quiet; the navy fill is the only loud control.
- **Shape:** gently squared (6px).
- **Primary:** navy fill, near-white text, 32px high, 14px horizontal padding, 14px medium.
- **Hover / Focus:** fill drops to 88%; colour transitions at 150ms; focus is a 2px ring-blue outline with 2px offset.
- **Outline:** card ground with a 70% field-stroke border; hover fills with `accent`.
- **Ghost:** muted text, no border; hover fills with `accent`. A destructive ghost (Rejeter) uses port-red text with a red-soft hover.
- **Link:** `link` colour, underline on hover.
- **Sizes:** sm 28px (13px text, 10px padding), lg 36px, icon 32px or 28px square. Disabled is 50% opacity.

### Chips
- **Badge:** 4px radius, 1px x 6px padding, 12px medium, tabular. Variants: neutral, running, success, danger, attention (soft ground, full text), signal (solid yellow, ink-navy text, semibold), outline, secondary, muted.
- **Status badge:** a badge led by a 6px dot in the state colour; the running dot pulses (1.6s) when motion is allowed.
- **Toggle group:** a bordered 2px-padded strip of 24px segments (theme toggle, cost grouping); the pressed segment takes `secondary` ground.

### Cards / Containers
- **Corner Style:** 8px.
- **Background:** `card` on the `background` ground.
- **Shadow Strategy:** none (see The Hairline Rule).
- **Border:** 1px `border`.
- **Internal Padding:** 12px x 16px header, 16px body, or flush for tables and row lists.

### Inputs / Fields
- **Style:** `card` ground, 1px `input` stroke, 6px radius, 32px high, 12px horizontal padding; textareas grow with content from 64px.
- **Focus:** border turns `ring` with a 3px ring at 25%; caret is ring-blue.
- **Error / Disabled:** `aria-invalid` turns the border destructive with a 20% (40% dark) ring; disabled is 50% opacity.

### Tables
Dense and scannable. 13px cells, 36px rows, 32px header in 12px medium muted text on `card`, sticky to the top of the scroll container. Rows are separated by 70% hairlines (none on the last row); hover tints the row with 60% `muted` in 100ms. First and last cells pad 16px, others 12px.

### Navigation
- **Rail:** marine navy, 12px x 8px padding, brand mark and wordmark at the top, primary links (Runs, Kanban, Agents), then Paramètres and the collapse control at the foot behind a rail seam. Links are 36px, 13px medium, 18px icons. Hover fills `rail-accent` at 70%. Active: `rail-accent` fill, bright text, and a 3px sea-foam bar on the rail's left edge. Arrow keys, Home and End move focus; focus is a 2px `rail-indicator` outline. Width animates over 200ms on the expo curve.
- **Secondary nav:** `card` column, 32px links in 13px medium muted text; hover `muted`, active `secondary`.

### Tide Table Timeline (signature)
Run detail steps are set as tide-table rows: clock time in mono on the left, a status icon, the step name with its status word and a metadata line (agent, model, tokens, cost), a latency bar, and the duration in 13px medium tabular figures right-aligned in a 4.5rem column. The bar (6px, full-rounded, `muted` track) is the step's exact latency on one fixed scale per run (the longest step fills it), coloured by step state. Tool calls nest under a hairline left rule. Decision rows share the time column and use a green or red shield.

### Proposed Column (signature)
The Kanban's Proposé column is the only tinted column: 70% yellow border, 60% `signal-attention-soft` ground, a solid soft header with a yellow dot, and a solid `signal` count badge when it holds cards. The other columns sit on 50% `muted` with a plain tabular count. Proposed cards carry Valider (primary sm) and Rejeter (destructive ghost sm); other cards use a 28px "Déplacer vers…" select.

## Do's and Don'ts

### Do:
- **Do** take every colour from the tokens in `src/styles.css` and define any new token in both `:root` and `.dark`.
- **Do** put `figures` on every number in a column, and mono on machine-produced strings.
- **Do** separate surfaces with 1px `border` seams and tonal steps.
- **Do** keep controls at 32px (28px small) and table rows at 36px.
- **Do** pair a state colour with an icon or a word, so state never depends on hue alone.
- **Do** use the single `ease-out-expo` curve at 150 to 200ms and let reduced-motion cut it.

### Don't:
- **Don't** use a signal colour for anything that is not a state.
- **Don't** use solid yellow for anything that does not need a human; the active rail item stays sea-foam white.
- **Don't** put shadows on resting panels, cards, tables or Kanban columns; `shadow-float` is for menus and tooltips.
- **Don't** add uppercase tracked labels or eyebrow text above titles; headings are sentence case at normal tracking.
- **Don't** go below 11px text, or use a second type family.
- **Don't** let the rail change colour with the theme; only its navy depth shifts.
