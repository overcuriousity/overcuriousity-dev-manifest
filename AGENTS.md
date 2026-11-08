# AGENTS.md

This repo is the overcuriousity dev manifest. It holds the rules for stack, security, look, interaction and ops of every app the owner builds, plus `oc-kit`, the crate that implements them. You are probably here because you were told to look at it.

## Read in this order

1. `manifest/00-principles.md`
2. `manifest/10-visual.md`
3. `manifest/20-components.md`
4. `manifest/30-layout.md`
5. `manifest/40-interaction.md`
6. `manifest/50-stack.md`
7. `manifest/60-security.md`
8. `manifest/70-ops.md`
9. `manifest/80-code.md`
10. `manifest/90-anti-patterns.md`
11. `manifest/99-review-checklist.md`

Rules are MUST, SHOULD, NEVER or MAY. Values live in `crates/oc-kit/assets/css/00-tokens.css`; the manifest quotes them.

## Starting a new app

1. Depend on the kit by tag: `oc-kit = { git = "https://github.com/overcuriousity/overcuriousity-dev-manifest", tag = "v2026.11.0" }`.
2. Copy `templates/` into the new repo: `.github/`, `deny.toml`, `rust-toolchain.toml`, `rustfmt.toml`, the lint table from `templates/lints.toml`, `.gitignore`, `install.sh`, `deploy/`, `scripts/bundle.sh`, `about.toml`, `about.hbs`, `AGENTS.md`, `SECURITY.md`, `CHANGELOG.md`, `config.example.toml`, `AIRGAP.md`. Replace `app` with the binary name.
3. Wire the kit the way `crates/demo/src/app.rs` does:
   - `oc_kit::config::load`, `cfg.deployment()`
   - `oc_kit::db::connect`, `migrate_kit`, then the app's migrator with `set_ignore_missing(true)`
   - `AuthState::new`, `session_layer`, `auth::router` at `/auth`, `prefs::prefs_router` at `/_oc/prefs`, `resource_metadata_router`
   - `assets::asset_router`, and `app_assets::<YourFiles>` for the app's own files
   - `security::secure(...)` around everything
4. Render pages with `oc_kit::ui::Page` and the components in `crates/oc-kit/src/ui/`. Pick one profile (workbench or everyday).
5. Record the manifest version the app follows in its own `AGENTS.md`.

## Changing an existing app

Do not migrate wholesale. Lift pieces with `docs/retrofit.md`, one at a time.

## Before you say you are done

- Run `manifest/99-review-checklist.md`.
- Run `oc-check .`: `cargo install --git https://github.com/overcuriousity/overcuriousity-dev-manifest --tag v2026.11.0 oc-check`.
- Never commit agent working files (`.superpowers/`, `.claude/`).

## This repo

- `crates/oc-kit/`: the kit. `crates/oc-check/`: the lint. `crates/demo/`: the reference app (see `crates/demo/README.md`).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check`, `node --test 'crates/oc-kit/assets/js/*.test.mjs'`, `cargo run -p oc-check -- .`
- sqlx query data is committed per crate (`.sqlx/`). After changing a query, run `cargo sqlx prepare` inside that crate against a database with both migration sets.
