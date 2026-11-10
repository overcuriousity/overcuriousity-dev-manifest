# 20 · Components

Every component has a reference implementation in `crates/oc-kit/src/ui/` and appears in the demo; see the inventory in `crates/demo/README.md`.

## Links and buttons

- MUST: a **link** goes somewhere: an `<a href>` that changes the page, has no side effect and can open in a new tab. Underlined, same size and weight as the text around it.
  - in prose: accent colour
  - in data (tables, facts, breadcrumbs): text colour, muted underline, accent on hover
- MUST: a **button** does something: a `<button>`, always a box or an icon, never accent-coloured text. Variants:
  - primary: accent fill, at most one per view
  - default: elevated fill and border
  - danger: danger tint and border, filled on hover
  - icon: borderless, 32 px target (24 px in rows), tooltip naming the action and its shortcut
- MUST: disabled is 0.4 opacity and a disabled primary turns neutral; busy is `aria-busy` with a spinner shown only after 400 ms.
- MUST: one focus ring everywhere: 1.5 px accent outline, 2 px offset, `:focus-visible` only.
- NEVER: accent text that is not a link, underlined text that is not a link, links styled as buttons, buttons styled as links, text-only buttons, bold or enlarged links, two primaries in one view.

## Tables

- MUST: header in surface fill, weight 500, secondary colour, visible vertical dividers between header cells, `border-strong` underline, sticky.
- MUST: rows 2.25rem high (compact 1.95rem) with `border-subtle` hairlines; hover fills `bg-hover`.
- MUST: selection is an `accent-dim` tint plus a 1 px accent frame around the row. *Why: visible at a glance without a heavy left bar.*
- MUST: a copy button next to every value worth copying, always visible, solid muted colour.
- MUST: other row actions (filter to, exclude, pin) appear on hover or focus, stay visible on the selected row and are always visible on touch.
- MUST: the actions column is right-aligned, its header visually hidden but named "Actions" for screen readers.
- MUST: long text truncates with an ellipsis; copy and the inspector give the full value.
- MUST: extra width shows more columns, by tier (see `30-layout.md`).

## Notices

- MUST: a coloured status symbol plus text in normal colour, separated by hairlines, with an optional action button at the end.
- NEVER: a box, a tint or a coloured left stripe around a notice.
- MUST: undo notices read "Deleted 'X' · Undo", hide after 6 s and pause while hovered or focused.

## Facts, empty states, forms

- MUST: facts lists pair a muted label column with values, copy buttons and hairline rows.
- MUST: an empty state is one line saying what is missing and how to add it, plus the button to do it. No illustrations.
- MUST: labels above fields; placeholders never replace labels; correct `type`, `inputmode` and `autocomplete`; inputs at least 16 px on touch; validation uses the notice pattern inline.

## Menus, pickers, dialogs

- MUST: the server renders a native `<select>`; on `(pointer: fine)` the kit upgrades it to a keyboard-navigable listbox; on `(pointer: coarse)` the native picker stays. The same applies to dates.
- NEVER: a native picker on desktop or a custom picker on touch.
- MUST: dialogs are `<dialog>` (max 26rem, scrim) on desktop and the same content as a bottom sheet on touch or narrow screens.

## Copy, keyboard, text size

- MUST: copy writes the full value, flips the icon to a check for 1.5 s and announces "Copied"; it is offered only in secure contexts.
- MUST: every shortcut is shown in its tooltip ("Filter to this (F)"); `?` lists them all, `/` focuses search, Esc closes the innermost layer; shortcuts never fire while typing.
- MUST: text size has two steps, default (100 %) and compact (87.5 %, rows tighten more than text), toggled in the top bar and stored like the theme.
