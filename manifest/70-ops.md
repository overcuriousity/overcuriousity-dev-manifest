# 70 · Ops

- MUST: CalVer `YYYY.M.N`, tags `vYYYY.M.N`, Keep a Changelog. Each repo has one version; in this repo the manifest, `oc-kit`, `oc-check` and the demo share it.
- MUST: a rule change (a MUST, SHOULD, NEVER or MAY added, removed or changed) is a release.
- MUST: copy `templates/.github/` into every app. `ci.yml` runs fmt, clippy, tests, cargo-deny, the MSRV check, `oc-check`, shellcheck and the static musl builds for x86_64 and aarch64 with artefacts.
- MUST: `release.yml` checks that the tag, the Cargo version and the CHANGELOG heading agree, builds both targets, runs `templates/scripts/bundle.sh`, attests build provenance and publishes the release.
- MUST: Dependabot for `cargo` and `github-actions`, weekly, grouped, prefix `chore(deps)`.

## Airgapped install

- MUST: every release ships offline bundles: binary, `install.sh`, systemd unit, nginx template, example config, licences and `SHA256SUMS`, plus a vendored source archive.
- MUST: `install.sh --archive <bundle>` installs without network, to `~/.local/bin` by default or system-wide with `--system` (root only), verifies checksums and never calls sudo.
- MUST: `AIRGAP.md` documents install, TLS with a local CA or self-signed certificate, and an offline build. See `docs/AIRGAP.md`.

## Deployment

- MUST: in production the app runs behind nginx on the same host and never terminates TLS. It binds `127.0.0.1:<port>`.
- MUST: `public_url` (the https address users see) is required unless the app is reached via localhost. It drives the origin check, the OIDC redirect URI and the passkey relying-party ID. The app refuses to start with a non-https `public_url` on a non-loopback host.
- NEVER: an insecure LAN mode. Development uses `http://localhost`, which browsers treat as secure.
- MUST: nginx forwards `Host`, `X-Forwarded-For` and `X-Forwarded-Proto`, turns buffering off for SSE routes and adds no security headers (`templates/deploy/nginx.conf`).
- MUST: the hardened systemd unit from `templates/deploy/app.service` (`DynamicUser`, `ProtectSystem=strict`, `NoNewPrivileges`, `MemoryDenyWriteExecute`, `UMask=0077`, `Type=notify` with watchdog).
- MUST: every repo has README, CHANGELOG, SECURITY.md, AIRGAP.md, THIRD-PARTY-LICENSES, LICENSE and an AGENTS.md naming the manifest version it follows.
