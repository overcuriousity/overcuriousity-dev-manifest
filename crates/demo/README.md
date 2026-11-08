# Incident journal (demo)

The reference app for the manifest. It uses every component and pattern the manifest defines, in both layout profiles, and consumes `oc-kit` exactly as an external app would.

All data is synthetic and generated on first start. The demo is meant to run locally, not on the internet.

## Run

```sh
cargo run -p demo
```

- Open `http://localhost:8080` and sign in as `demo`. The password is printed once in the log on first start.
- Add more accounts with `demo user add <name> [--admin]`.
- Settings go in `demo.toml` (see `config.example.toml`) or in `DEMO__…` environment variables.
- Live ingest adds about two events per second. Turn it off with `DEMO__APP__INGEST__ENABLED=false`.

## Component inventory

Every component and pattern from `manifest/20-components.md`, `30-layout.md` and `40-interaction.md`, and where it appears.

| Component or pattern | Where |
|---|---|
| Link: prose | `/notes/{id}` (case link in facts), error pages ("Back to notes") |
| Link: data (muted underline) | note list, `/cases`, breadcrumb, facts |
| Primary button (one per view) | "New note" on `/notes`, "Save note" on the form, "Sign in" |
| Default button | Edit, Undo, Restore, verdict buttons |
| Danger button | Delete, Revoke, Purge case |
| Icon button with tooltip and shortcut | timeline rail, row actions, top bar toggles |
| Copy button | IDs, links, hashes, command lines, logon IDs, tokens |
| Notice (info, success, warning, danger) | inspector "Pinned to the story", undo notices, "Shown once. Copy it now.", form and query errors |
| Undo notice (6 s, pauses on hover and focus) | delete note, remove filter, set verdict, revoke token, remove passkey |
| Trash (30 days) | `/trash` |
| Typed-name confirmation | "Purge case" on `/cases` |
| Facts list | note detail, inspector, account |
| Empty state | empty Trash, empty case timeline, no matches, no tokens, no passkeys |
| Table: header dividers, sticky header, hover, framed selection | `/cases/{id}/timeline` |
| Tiered columns (100/150/200rem) | timeline: SHA-256 and PID, then parent, command line and record, then FQDN, integrity, logon ID and source file |
| Row actions on hover, visible on touch | timeline: filter to, exclude, pin |
| Right-click or long-press menu | timeline rows |
| Keyboard: `?`, `/`, Esc, j/k, Enter, F, X, P, S, `\` | timeline; `?` everywhere |
| Filter pills | timeline toolbar |
| Facets with counts | timeline left region (folds into the rail below 100rem) |
| Search syntax | timeline search box (`host:ws-01 -user:SYSTEM powershell`) |
| Histogram (server-rendered SVG, live) | timeline strip |
| Live updates over SSE, "N new events · Show" | timeline |
| Optimistic update with undo | verdict buttons in the inspector |
| Opt-in side pane (story) | rail "Story" (S), resizable |
| Native select on touch, custom menu on desktop | note form "Case", account "Theme" and "Text size" |
| Dialog on desktop, bottom sheet on touch | purge confirmation, shortcut list |
| Theme toggle (system, light, dark) | top bar, account |
| Text-size toggle (default, compact) | top bar, account |
| Workbench profile | `/cases/{id}/timeline` |
| Everyday profile: phone tab bar, centred column, list and detail, third region | `/notes`, `/notes/{id}` (related notes at 150rem) |
| Installable app (manifest, service worker, offline page) | `/manifest.webmanifest`, `/sw.js`, `/offline` |
| Sign in: password, passkey, SSO when configured | `/auth/sign-in` |
| API tokens (shown once) | `/account` |
| REST API with OpenAPI | `/api/v1/openapi.json` |
| MCP tools | `/mcp`: `list_cases`, `search_events`, `get_event`, `add_note` |
| Error pages: one line, one way back | any unknown URL |
