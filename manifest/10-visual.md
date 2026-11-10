# 10 · Visual

Source of truth: `crates/oc-kit/assets/css/00-tokens.css`. This page quotes it; a test keeps them equal.

## Colour

Warm paper in light mode, cool ink in dark mode, one muted accent: blue in light, teal in dark. The hue shift is intentional.

| Token | Light | Dark |
|---|---|---|
| `--color-bg-base` | `#f8f6f1` | `#0e1015` |
| `--color-bg-surface` | `#f2f0ea` | `#14171d` |
| `--color-bg-elevated` | `#ffffff` | `#1b1e26` |
| `--color-bg-hover` | `#ece9e2` | `#262a34` |
| `--color-bg-active` | `#e2ded3` | `#2d3140` |
| `--color-fg-primary` | `#2d2d2d` | `#e2e4ec` |
| `--color-fg-secondary` | `#5a5a5a` | `#9599b0` |
| `--color-fg-muted` | `#6c6c65` | `#8185a3` |
| `--color-border` | `#ddd8cc` | `#232636` |
| `--color-border-strong` | `#c9c3b4` | `#2e3244` |
| `--color-border-subtle` | `#eae7de` | `#191c26` |
| `--color-border-control` | `#8a8478` | `#6a6f87` |
| `--color-accent` | `#386889` | `#5aa8b0` |
| `--color-accent-fg` | `#ffffff` | `#0e1015` |
| `--color-danger` | `#b3382c` | `#e77676` |
| `--color-warning` | `#845b16` | `#e8a839` |
| `--color-success` | `#2b7048` | `#4caf7d` |
| `--color-accent-dim`, `-danger-dim`, `-warning-dim`, `-success-dim` | 10 % | 15 % |
| `--color-accent-muted` | 25 % | 35 % |

- MUST: primary text ≥ 7:1, all other text ≥ 4.5:1, meaningful non-text UI (focus ring, selection frame, form-field borders) ≥ 3:1. *Why: WCAG 1.4.3 and 1.4.11; 7:1 supports long reading sessions.*
- MUST: a contrast unit test checks every documented pair in both themes (reference: `crates/oc-kit/tests/contrast.rs`).
- MUST: muted text switches to `fg-secondary` on hovered and selected rows. *Why: muted is 4.4:1 (light) and 4.0:1 (dark) on `bg-hover`.*
- MUST: form-field borders use `border-control`; every other border stays a soft hairline. *Why: `border-strong` is only about 1.6:1.*
- MUST: status is symbol + colour (✓ success, ⚠ warning, ✗ danger, i info), never colour alone. *Why: WCAG 1.4.1; about 8 % of men have a red-green colour vision deficiency.*
- NEVER: a literal colour outside the token file. `oc-check` enforces it.
- NEVER: pure `#000` or `#fff` as page background or body text. *Why: halation and glare over long sessions.*
- NEVER: gradients, glows or saturated decorative fills. Saturated colour is reserved for state and selection.

## Theme

- MUST: three states: system (default), light, dark. With no system preference, use light. *Why: dark-on-light reads better in lit rooms (Piepenbrock et al. 2013); the system setting expresses dim rooms.*
- MUST: light tokens on `:root`; dark tokens under `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) }` and repeated under `:root[data-theme="dark"]`.
- MUST: store the choice per user and render `data-theme` / `data-density` on `<html>` server-side. Signed-out pages use `localStorage` through the blocking `theme.js`.
- MUST: the toggle is an icon button in the top bar whose tooltip names the next state, not the current one.

## Typography

| Role | Size | Weight / line height | Use |
|---|---|---|---|
| page | 1.375rem (22) | 600 / 1.25, −0.02em | page title |
| section | 1.125rem (18) | 600 / 1.3, −0.01em | section heading |
| prose | 1rem (16) | 400 / 1.6, max 68ch | running text |
| ui | 0.875rem (14) | 400–500 / 1.5 | everyday body, forms, menus |
| dense | 0.8125rem (13) | 400–500 / 1.5 | workbench body, tables, buttons |
| micro | max(0.75rem, 12px) | 400–500 | metadata; label variant uppercase 500, 0.06em |
| mono | 0.8125rem (13) | 400 | IDs, hashes, timestamps, paths |

- MUST: Atkinson Hyperlegible Next for the interface, JetBrains Mono for data, self-hosted woff2 with `font-display: swap`. *Why: letterforms built to be told apart (I/l/1, 0/O); peer-reviewed comparative evidence is limited.*
- MUST: numbers in tables are tabular and right-aligned.
- MUST: running text at 16 px and at most 68 characters per line. *Why: Rello et al. 2016; Dyson 2004.*
- MUST: no rendered text below 12 px at any density. *Why: reading speed and accuracy drop below that.*
- MUST: the root font size is a percentage (100 %, compact 87.5 %), never px. *Why: WCAG 1.4.4; px overrides the user's setting.*

## Spacing, geometry, icons, motion

- MUST: spacing from the scale 0.25 / 0.5 / 0.75 / 1 / 1.5 / 2 rem; radii 3 px (small) and 6 px (controls, dialogs) only.
- MUST: separate with borders and hairlines. The one shadow (`--shadow-float`) is for floating layers only.
- MUST: z-index values come from the token scale.
- MUST: Lucide icons, inline SVG, `aria-hidden`, `currentColor`, with a constant rendered stroke of 1.4 px (`vector-effect: non-scaling-stroke`). Sizes: 14 px in rows, 16 px in toolbars, 20 px in the phone tab bar.
- MUST: one icon per meaning, from a fixed vocabulary (reference: `crates/oc-kit/src/ui/icons.rs`). Add new meanings to the vocabulary; never improvise.
- MUST: motion lasts 120 ms (hover, press) or 160–180 ms (panels), ease-out, and only explains what moved. Everything stops under `prefers-reduced-motion`.
- NEVER: emoji as icons; icons inside coloured rounded tiles.

## Charts

- MUST: the categorical palette `--chart-cat-1` to `-8`, assigned by index and never reshuffled when the series count changes. Slots below 3:1 get direct labels or a legend.
- MUST: sequential `--chart-seq-1` to `-4` (reversed in dark), diverging blue ↔ orange (`--chart-div-*`), never red/green.
- SHOULD: render charts as server-side SVG. A charting library, if ever needed, is vendored.
