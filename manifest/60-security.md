# 60 · Security

## Headers

- MUST: every HTML response carries exactly this CSP, set by `oc_kit::security::secure`:

  `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; manifest-src 'self'; worker-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'`

- MUST: also `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cross-Origin-Opener-Policy: same-origin`, HSTS when `public_url` is https, and `Cache-Control: no-store` on authenticated responses.
- NEVER: `style=` attributes, inline `<script>`, `on*=` attributes, `unsafe-inline`, `unsafe-eval` or `unsafe-hashes`. Dynamic sizes go through the CSSOM.

## Sessions and requests

- MUST: the session cookie is `__Host-<app>-session`, HttpOnly, Secure, SameSite=Lax; the session id rotates on sign-in. *Why Lax: the OIDC callback is a cross-site navigation; the origin check covers CSRF.*
- MUST: state-changing requests pass the origin check: a present `Origin` must be ours; otherwise `Sec-Fetch-Site` must be `same-origin` or `none`; with neither, the request may carry no cookie. `Origin: null` counts as absent. *Why: browsers send `Origin: null` on same-origin form posts under `no-referrer`.*
- MUST: the `Host` header must be the public host (or loopback in development); otherwise 421. *Why: DNS rebinding, as the MCP spec warns.*
- MUST: `X-Forwarded-*` is trusted only from `trusted_proxies` (default loopback).
- MUST: rate-limit sign-in and API routes per client address; limit request bodies (default 1 MiB).
- MUST: accounts are created by an admin or by a first OIDC login when `auto_provision` is on; OIDC identities match on issuer and subject, never e-mail.
- MUST: API tokens look like `<prefix>_<id>.<secret>`, are stored as argon2 hashes and are shown once.
- MUST: with an IdP, `/mcp` is an OAuth 2.1 resource server and `/.well-known/oauth-protected-resource` points at the IdP; without one, MCP uses app-issued tokens. NEVER: a built-in authorization server.

## Code and supply chain

- MUST: `unsafe_code = "forbid"`, clippy `-D warnings`, `unwrap_used` and `expect_used` denied outside tests.
- MUST: `cargo-deny` for advisories, licences, bans and sources. Allowed licences: MIT, Apache-2.0 (with LLVM exception), BSD-2/3-Clause, ISC, Zlib, 0BSD, CC0-1.0, Unicode-3.0, Unicode-DFS-2016, MPL-2.0, OFL-1.1 (fonts), CDLA-Permissive-2.0 (CA data).
- NEVER: GPL, AGPL or LGPL dependencies. The app's own licence is free to choose.
- MUST: every ignored advisory carries a written reachability argument in `deny.toml`.
- MUST: GitHub Actions pinned to full commit SHAs with minimal `permissions:`; Dependabot weekly.
- MUST: no egress unless configured, no telemetry, no CDN.
- MUST: `SECURITY.md` asks for private advisories, never public issues.
