# 30 · Layout

Every app declares one profile. Breakpoints are container queries in rem on the app body, so compact text size shows more automatically.

## Workbench (data processing, long sessions)

- MUST: a full-height shell: top bar (44 px: logo, breadcrumb, search with `/`, theme and text-size toggles, account), icon rail (44 px), filter facets, main region (histogram strip, toolbar with filter pills, table), inspector, status bar (24 px).
- MUST: use the full width; extra width becomes more columns:
  - below 100rem: facets fold into the rail (toggle `\`), inspector 22rem
  - from 100rem: facets open (15rem), tier-2 columns, inspector 24rem
  - from 150rem: tier-3 columns, inspector 28rem
  - from 200rem: tier-4 columns
- MUST: side panes beyond the inspector are opt-in from the rail and never open on their own.
- SHOULD: on phones the workbench stays readable and usable read-only; it is not optimised.

## Everyday (quick tasks, phone and desktop)

- MUST: below 48rem: one column, top bar with title and one action, a bottom tab bar with 3–5 destinations as icon + label, the primary action bottom-right in thumb reach. *Why: about half of phone use is one-handed (Hoober); unlabelled icons are misread (NN/g).*
- MUST: from 48rem: top navigation, search and one centred column of at most 46rem.
- MUST: from 90rem: list and detail side by side (28rem + 52rem); from 150rem: a third region (22rem).
- MUST: regions keep readable widths; content never stretches across an ultrawide screen.
- SHOULD: installable as a PWA with an offline page; app data is not cached on the device.
- NEVER: a hamburger menu as primary navigation on phones.

## Touch (both profiles)

- MUST: targets at least 44 px with 8 px between them. *Why: Parhi et al. 2006; Apple HIG; WCAG 2.5.8.*
- MUST: primary actions and navigation at the bottom, rare actions at the top.
- MUST: every gesture has a visible button; long-press opens the same menu as right-click.
- MUST: respect `env(safe-area-inset-*)`.
- NEVER: pull-to-refresh; live data arrives over SSE.
