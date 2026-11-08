# 40 · Interaction

## Destructive actions

- MUST: reversible actions (delete a note, remove a filter, archive, revoke a token) happen at once and show an undo notice; deleted items stay in Trash for 30 days.
- MUST: irreversible actions (purge evidence, rotate a key, delete an account) use a dialog whose destructive button enables only when the item's name is typed exactly.
- NEVER: native `confirm()`, `alert()` or `prompt()`. *Why: confirmations habituate into reflex clicks (Bravo-Lillo et al. 2013); "never use a warning when you mean undo" (Raskin).*

## Timing

- MUST: update optimistically and reconcile with the server's answer; give feedback within 100 ms, show a spinner only after 400 ms, show progress beyond 1 s. *Why: Card, Moran & Newell; Nielsen's 0.1 / 1 / 10 s limits.*

## Clicks

- MUST: left click acts; right click (long-press on touch) offers options. Menus nest; they never replace the page.

## Microcopy

- MUST: English, sentence case, verbs on buttons, actions named by their effect ("Delete 'ACME breach' and 3 items").
- MUST: "Sign in" and "Sign out".
- MUST: errors say what happened and what to do, in one sentence.
- NEVER: marketing tone, exclamation marks, emoji, or naming another product inside the product.
