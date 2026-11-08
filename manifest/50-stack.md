# 50 · Stack

| Area | Choice |
|---|---|
| Web | axum 0.8 + tower-http; security headers from `oc-kit` |
| Templates | Askama (compile-time checked, auto-escaping) |
| Interactivity | htmx 2, vendored, with its SSE extension; small vanilla ES modules from `oc-kit` |
| Live updates | server-sent events fed by `tokio::sync::broadcast` |
| OpenAPI | utoipa + utoipa-axum, spec generated from the routes at `/api/v1/openapi.json` |
| MCP | rmcp streamable HTTP at `/mcp`; tools are adapters over the same service functions the pages use |
| Database | SQLite through sqlx with compile-time-checked queries (`.sqlx` committed) and migrations |
| Sessions | tower-sessions with the SQLite store |
| Browser auth | backend-for-frontend: OIDC code flow + PKCE (openidconnect 4), passkeys (webauthn-rs), argon2 local passwords |
| API and MCP auth | bearer: IdP JWTs validated against JWKS, or app-issued prefixed tokens stored argon2-hashed and shown once |
| Assets | rust-embed, content-hashed URLs, `immutable` caching |
| Releases | static musl binaries for x86_64 and aarch64 |

- MUST: depend on `oc-kit` by CalVer tag: `oc-kit = { git = "https://github.com/overcuriousity/overcuriousity-dev-manifest", tag = "v2026.11.1" }`.
- MUST: htmx runs with this config: `{"allowEval":false,"allowScriptTags":false,"includeIndicatorStyles":false,"selfRequestsOnly":true,"attributesToSettle":["class","width","height"],"responseHandling":[{"code":"204","swap":false},{"code":"[23]..","swap":true},{"code":"422","swap":true},{"code":"[45]..","swap":false,"error":true}]}`. *Why: anything else needs eval or writes inline styles under the strict CSP; 422 swaps so validation messages appear in place.*
- NEVER: `hx-on` or htmx event filters (both need eval).
- MUST: wrap table elements in out-of-band swaps in `<template>`. *Why: a bare `<td>` outside a table is dropped by the HTML parser.*
- MUST: SSE events always carry a data line (`oc_kit::sse` does this). *Why: browsers drop events without data.*
- MUST: run kit and app migrations with `set_ignore_missing(true)` on both migrators; kit versions are small integers, app versions are 14-digit timestamps.
- MUST: apps own every table except `oc_*`; `oc-kit` authenticates, the app authorises.
- MAY: Postgres, but only when a project needs several nodes. `oc-kit` v1 is SQLite only.
- NEVER: an SPA framework (React, Svelte, Leptos), Alpine, or Datastar. *Why: Datastar needs `unsafe-eval` (data-star.dev/reference/security).*
- MUST: pin current stable versions in `Cargo.lock`; this page names minimum majors only.
