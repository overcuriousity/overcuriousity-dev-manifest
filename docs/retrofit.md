# Retrofitting an existing app

Bring an existing app closer to the manifest one piece at a time, without taking the `oc-kit` dependency. Each piece is a plain file you copy, plus a few lines to wire it.

## Tokens and CSS

Copy:
- `crates/oc-kit/assets/css/00-tokens.css`
- `crates/oc-kit/assets/css/10-base.css`, `20-layout.css`, `30-components.css`, `40-tables.css`, `50-touch.css`, `60-charts.css` (concatenate them in that order)
- `crates/oc-kit/assets/css/profile-workbench.css` or `crates/oc-kit/assets/css/profile-everyday.css`

Wire:
- Serve the files from your own origin and link them in `<head>`.
- Put `class="oc-app oc-app--<profile>"` on `<body>` and `oc-shell oc-shell--<profile>` on the shell.
- Replace literal colours with tokens; `oc-check` finds them.

## Fonts

Copy `crates/oc-kit/assets/fonts/` with its OFL files. `10-base.css` already declares the `@font-face` rules with paths relative to `css/`.

## Icons

Copy the SVGs you need from `crates/oc-kit/assets/icons/` and keep the meanings from `crates/oc-kit/src/ui/icons.rs`. Inline them with `class="oc-icon"`; the CSS sets size and stroke.

## JS modules

Copy `crates/oc-kit/assets/js/`. Each module is standalone with no build step:
- `theme.js`: blocking, in `<head>`
- every other module: `<script type="module">`

They rely on the markup hooks described in `manifest/20-components.md` (`data-oc-copy`, `data-oc-pref`, `data-oc-menu` and the rest). Also copy `crates/oc-kit/assets/vendor/` for htmx, and use the htmx config from `manifest/50-stack.md`.

## Security headers and origin check

Copy `crates/oc-kit/src/security/`. It depends only on axum, tower-http and `Deployment` from `crates/oc-kit/src/config.rs`.

Wire: wrap the whole router with the copied `secure(router, &deployment, &trusted_proxies, Limits::default())`, outermost. For a non-Rust app, reproduce the exact CSP from `manifest/60-security.md` and the origin rules listed there.

## Session cookie

Copy `crates/oc-kit/src/session.rs`, or set the same attributes by hand: `__Host-<app>-session`, HttpOnly, Secure, SameSite=Lax. Rotate the session id on sign-in.

## Contrast test

Copy `crates/oc-kit/src/tokens.rs` and `crates/oc-kit/tests/contrast.rs`, and point `include_str!` at your token file.

## oc-check in CI

Add the `oc-check` job from `templates/.github/workflows/ci.yml`. To test rendered pages, use `oc_check::check_html_for` as `crates/demo/tests/rendered.rs` does.

## CI, releases and offline install

Copy `templates/.github/`, `templates/install.sh`, `templates/deploy/`, `templates/scripts/bundle.sh`, `templates/about.toml`, `templates/about.hbs` and `templates/AIRGAP.md`. Set `BIN` in both workflows and replace `@APP@`/`app` with the binary name.
