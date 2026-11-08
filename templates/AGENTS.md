# AGENTS.md

<One line: what this app is and who uses it.>

## Rules

This app follows the overcuriousity dev manifest **v2026.11.1**:
https://github.com/overcuriousity/overcuriousity-dev-manifest/tree/v2026.11.1/manifest

- Profile: <workbench | everyday>.
- Read the manifest files in number order before changing UI, security or ops.
- Before declaring work done, run `manifest/99-review-checklist.md` and `oc-check .`.

## Layout

- `src/` …
- `migrations/` app tables; `oc-kit` owns the `oc_*` tables.

## Commands

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo deny check`
