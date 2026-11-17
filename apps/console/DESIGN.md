---
name: Capitainerie Console
description: Compact console. Cool grey ground, one blue accent, colour only for state.
colors:
  # Light theme (:root in src/styles.css)
  background: "#f3f4f6"
  foreground: "#16181d"
  card: "#ffffff"
  primary: "#3d5df0"
  primary-foreground: "#ffffff"
  secondary: "#eceef2"
  muted-foreground: "#5b606a"
  destructive: "#c8414a"
  border: "#dcdfe5"
  input: "#8f949d"
  ring: "#3d5df0"
  rail: "#ffffff"
  rail-accent: "#eceef2"
  signal-running: "#3d5df0"
  signal-success: "#187a40"
  signal-danger: "#c8414a"
  signal-attention: "#d49a2a"
  signal-attention-ink: "#8a6008"
  signal-neutral: "#5b606a"
  # Dark theme (.dark in src/styles.css)
  background-dark: "#141517"
  foreground-dark: "#e9eaec"
  card-dark: "#191a1d"
  primary-dark: "#4964ee"
  muted-foreground-dark: "#8d9199"
  border-dark: "#2b2d32"
  input-dark: "#62666e"
  ring-dark: "#7a93ff"
  signal-running-dark: "#8aa2ff"
  signal-success-dark: "#3fb76a"
  signal-danger-dark: "#e8737a"
typography:
  body:
    fontFamily: "'Inter Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "13px"
    lineHeight: 1.4
  title:
    fontFamily: "'Inter Variable', ui-sans-serif, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 600
  label:
    fontSize: "12px"
    fontWeight: 500
  mono:
    fontFamily: "'JetBrains Mono Variable', ui-monospace, Menlo, monospace"
    fontSize: "11.5px"
    fontFeature: "'tnum' 1, 'lnum' 1"
rounded:
  sm: "5px"
  md: "7px"
  lg: "9px"
spacing:
  unit: "4px"
  control: "28px"
  rail-expanded: "204px"
  rail-collapsed: "48px"
  topbar: "40px"
---

# Design System: Capitainerie Console

## Overview

A compact working console: one light or dark sidebar, a thin top bar with a breadcrumb and a search field (⌘K), pages that start with a small title and a grey one-line description. Density first: 13px text, 28px controls, tables laid on the ground, panels in a thin frame.

The layout and density come from the "Capitainerie" mockup (2026-11-17, see `docs/specs/2026-11-console-ui-refresh.md`). It replaces the former "Harbour Register" direction (warm paper, Atkinson). Both themes follow the system by default and are first-class.

## Colors

- **Ground** (`background`), **panel** (`card`, `rail`), **hover** (`secondary`, `muted`, `rail-accent`), **rule** (`border`). Field strokes use `input`, kept at 3:1.
- **Accent** (`primary`, `ring`, `link`): the primary button, the focus ring and links. It is the only brand colour.
- **Signals**: running blue, success green, danger red, attention amber, neutral grey, each with a `-soft` ground for badges.

### Named Rules

**The State-Only Rule.** A signal colour carries a state and nothing else.

**The Amber Rule.** Amber (`signal-attention`) means a human must act or look: the Proposé column and its count, the degraded realtime dot.

**The Token Rule.** Components use the token names of `src/styles.css`, never a raw hex value.

## Typography

Inter for text, JetBrains Mono for identifiers, paths and figures. Tabular figures in every column. Page title 18px, section title 12px semibold, body 13px.

## Components

- **Sidebar**: 204px, collapses to a 48px rail, drawer under 768px. Current link `aria-current="page"`.
- **Top bar**: breadcrumb, realtime state, search (⌘K), theme toggle.
- **Tabs**: links, one URL per tab.
- **Panel**: `card` ground, 1px `border`, 9px radius, header 12px semibold.
- **Demonstration banner**: shown on every page fed by `domain/<d>/mock/` data.

## Accessibility

WCAG AA contrast in both themes, visible focus, full keyboard navigation of sidebar, tabs and palette, `prefers-reduced-motion` respected.
