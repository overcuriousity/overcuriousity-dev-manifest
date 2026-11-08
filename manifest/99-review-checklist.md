# 99 · Review checklist

Run before declaring work done. `oc-check .` covers the mechanical items; the rest needs a look.

1. Colours come from tokens only, and the contrast test passes.
2. Links go somewhere, buttons do something; one primary per view.
3. Every copyable value has a copy button.
4. Status is symbol + colour.
5. No slop tells (`manifest/90-anti-patterns.md`).
6. Comments explain why, only where needed; none narrate.
7. The CSP has no `unsafe-*`; no inline styles, scripts or `on*=` attributes (`oc-check`, and the rendered-page test).
8. No new outbound connection unless configured.
9. Destructive actions offer Undo or a typed-name confirmation, never `confirm()`.
10. Phone (390 px), desktop and ultrawide checked; touch targets 44 px.
11. `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo deny check` are green.
12. CHANGELOG updated; the airgap bundle still builds.
