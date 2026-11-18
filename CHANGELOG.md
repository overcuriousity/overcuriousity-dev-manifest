# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions are CalVer `YYYY.M.N`.

## [Unreleased]

## [2026.11.2] - 2026-11-18

### Fixed
- The build on main: `tower-sessions-sqlx-store` 0.16 for sqlx 0.9, and the rand 0.10 `RngExt` and sd-notify 0.5 API changes.
- The rate limiter groups IPv6 by /64 and maps IPv4-mapped addresses to IPv4 (#4).
- `password::generate` compiles with rand 0.10 (#5).

### Added
- `oc-kit`: public `Buckets::new` and `take`, the `extra_hosts` config key, `qr-code`, `upload` and `file` icons, per-route body and timeout limits through `Limits.overrides`, and `config::load_for_check` (#2).
- `oc-kit`: an autocomplete helper (`complete.js`) and the `CompleteField` component (#6).
- `demo`: the `check-config` subcommand, and a note title that suggests earlier titles.
- Templates: `install.sh` does first install, update with check-before-change, backup, rollback and a health wait, online or offline with `--archive`; `bundle.sh` bakes the version and repo into it (#7).
- Manifest: the autocomplete SHOULD in `40-interaction.md` and installer items in the review checklist.

### Changed
- Manifest: the deployer decides. Proxy, TLS, bind address, paths and service manager are settings; nginx and the systemd unit are SHOULD, not MUST.
- Manifest: plain http on a LAN MAY be used when `allow_plain_http = true` is set; the old NEVER on an insecure LAN mode is gone.
- Manifest: `oc-kit` is a reference implementation; depending on it is optional, and kit-specific rules moved to "Using `oc-kit`" in `50-stack.md`.
- `oc-kit`: `allow_plain_http` accepts an http `public_url` on a non-loopback host. There the session cookie is `<app>-session` without Secure, passkeys are off and `Referrer-Policy` is `same-origin` so the origin check still works.
- `oc-kit`: `session_layer` takes the deployment again, to pick the cookie name and Secure flag.
- Manifest: `70-ops` "Install" rules cover the installer lifecycle, `check-config`, `/healthz` and the `extra_hosts` MAY.
- Manifest: `60-security` lets uploads raise limits by path.
- Manifest: `20-components` makes Tab an exception to "shortcuts never fire while typing" and describes the autocomplete field.
- `deny.toml` ignores RUSTSEC-2026-0543 (chrono unmaintained), with its reason.

## [2026.11.1]

### Security
- The host check ignores `X-Forwarded-Host`, which a client could set itself.
- JWT bearer validation takes the signing algorithm from the key, never from the token alone.
- A removed passkey can be restored only within an hour.
- The rate limiter shares one bucket per IPv6 /64 and keeps its memory bounded.
- `HxAttrs` drops event handlers, `hx-on` and other disallowed attributes at runtime; `Html` is built from components or the explicit `Html::raw`.

### Fixed
- `*_FILE` environment overrides work for any key, not only secrets.
- `session_layer` no longer takes an unused deployment argument.
- Releases install `cargo-zigbuild` and `cargo-about` as prebuilt binaries.

## [2026.11.0]

### Added
- Manifest rules: principles, visual identity, components, layout profiles, interaction, stack, security, ops, code, anti-patterns and the review checklist.
- `oc-kit`: design tokens with a contrast test, CSS layers and profiles, Atkinson Hyperlegible Next and JetBrains Mono, the Lucide icon vocabulary, vendored htmx with SSE, standalone JS modules, the security layer, users, password and passkey sign-in, OIDC, API tokens, per-user preferences, an SSE helper, soft delete with undo and Trash, and Askama components.
- `oc-check`: a lint for literal colours, inline styles and scripts, inline handlers, `hx-on`, native dialogs, external URLs, an unsafe CSP and small text.
- `demo`: an incident journal with an everyday notes half and a workbench timeline, live ingest, REST API with OpenAPI, and MCP tools.
- Templates for CI, releases with offline bundles, Dependabot, cargo-deny, the systemd unit, nginx and the offline installer.
