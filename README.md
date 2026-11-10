# overcuriousity dev manifest

The shared rules for the apps I build: stack, security, visual identity, interaction and ops, written so an AI coding agent can follow them. It is a reference first: the crates show one way to implement the rules, and apps need not depend on them.

- **`manifest/`**: the rules, terse, each with its reason. Start at `manifest/00-principles.md`.
- **`crates/oc-kit/`**: the reference implementation, a Rust crate an app may read, copy from or depend on by tag. It covers design tokens and CSS, fonts and icons, htmx, the security layer, sign-in (password, passkeys, OIDC), API tokens, preferences, live updates and page components.
- **`crates/oc-check/`**: a lint that fails CI on literal colours, inline styles and scripts, native dialogs, CDN URLs and an unsafe CSP.
- **`crates/demo/`**: an incident journal that shows every component in both layout profiles: an everyday notes app and a data-heavy timeline workbench.
- **`templates/`**: files copied into each new repo: CI, releases with an offline bundle, Dependabot, deny rules, systemd unit, nginx, installer.

Agents start with `AGENTS.md`.

## Look

Warm paper in light mode, cool ink in dark. One muted accent, Atkinson Hyperlegible Next and JetBrains Mono, hairlines instead of shadows, and status that never relies on colour alone. Workbench apps turn extra width into more columns; everyday apps get a labelled tab bar on phones and calm, capped columns on wide screens.

## Run the demo

```sh
cargo run -p demo
```

Open `http://localhost:8080`. The `demo` user's password is printed once in the log. For an offline install, see `docs/AIRGAP.md`.

## Versioning

CalVer `YYYY.M.N`, one version for the whole repo. Every rule change is a release; see `CHANGELOG.md`.

## Licence

MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). The bundled fonts (SIL OFL 1.1), Lucide icons (ISC) and htmx (0BSD) keep their own licences; see `THIRD-PARTY-LICENSES`.
