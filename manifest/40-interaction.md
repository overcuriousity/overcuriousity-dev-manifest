# 40 · Interaction

## Destructive actions

- MUST: reversible actions (delete a note, remove a filter, archive, revoke a token) happen at once and show an undo notice; deleted items stay in Trash for 30 days.
- MUST: irreversible actions (purge evidence, rotate a key, delete an account) use a dialog whose destructive button enables only when the item's name is typed exactly.
- NEVER: native `confirm()`, `alert()` or `prompt()`. *Why: confirmations habituate into reflex clicks (Bravo-Lillo et al. 2013); "never use a warning when you mean undo" (Raskin).*

## Timing

- MUST: update optimistically and reconcile with the server's answer; give feedback within 100 ms, show a spinner only after 400 ms, show progress beyond 1 s. *Why: Card, Moran & Newell; Nielsen's 0.1 / 1 / 10 s limits.*

## Clicks

- MUST: left click acts; right click (long-press on touch) offers options. Menus nest; they never replace the page.

## Autocomplete

- SHOULD: a text field that repeats earlier input suggests from the app's own data only: history, saved setups, built-in presets. Nothing is sent anywhere, and the field has `autocomplete="off"`.
- MUST: a suggesting field shows the rest of the best match as grey text after the caret, inside the field. Matching is case-insensitive and anchored at the start. The app hands the field its candidates ranked: same paired context first, then most recent, then most used.
- MUST: in a suggesting field, Tab takes the next word; without a visible suggestion Tab moves focus as usual. Esc hides the suggestion until the text changes. A take button under the field does the same for touch screens and for text that overflows the field, keeps focus in the field, and keeps its space while idle so the layout never moves.
- MUST: under an empty suggesting field, up to five recent values; under a "new item" field, often-used ones. Each fills the field with one tap.
- MUST: a suggesting field inserts a taken word through `document.execCommand('insertText')`, so undo works and the view gets an ordinary `input` event. The fallback sets the value, sends one `input`, and lets `change` follow on blur.
- MUST: suggesting fields share one hidden description ("Tab takes the suggested word. Escape hides it.") linked with `aria-describedby`; a polite live region announces the suggestion after a short pause, cut to about 80 characters; the ghost text is `aria-hidden`.
- MUST: an app with suggesting fields reads their data once per page and keeps it current with its own writes. A failed read is retried once after a pause by the shared copy, not by each field.

## Microcopy

- MUST: English, sentence case, verbs on buttons, actions named by their effect ("Delete 'ACME breach' and 3 items").
- MUST: "Sign in" and "Sign out".
- MUST: errors say what happened and what to do, in one sentence.
- NEVER: marketing tone, exclamation marks, emoji, or naming another product inside the product.
