# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions are CalVer `YYYY.M.N`.

## [Unreleased]

### Changed
- Manifest: the deployer decides. Proxy, TLS, bind address, paths and service manager are settings; nginx and the systemd unit are SHOULD, not MUST.
- Manifest: plain http on a LAN MAY be used when `allow_plain_http = true` is set; the old NEVER on an insecure LAN mode is gone.
- Manifest: `oc-kit` is a reference implementation; depending on it is optional, and kit-specific rules moved to "Using `oc-kit`" in `50-stack.md`.
- `oc-kit`: `allow_plain_http` accepts an http `public_url` on a non-loopback host. There the session cookie is `<app>-session` without Secure, passkeys are off and `Referrer-Policy` is `same-origin` so the origin check still works.
- `oc-kit`: `session_layer` takes the deployment again, to pick the cookie name and Secure flag.

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
