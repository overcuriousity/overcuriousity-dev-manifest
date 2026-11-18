# 70 · Ops

- MUST: CalVer `YYYY.M.N`, tags `vYYYY.M.N`, Keep a Changelog. Each repo has one version; in this repo the manifest, `oc-kit`, `oc-check` and the demo share it.
- MUST: a rule change (a MUST, SHOULD, NEVER or MAY added, removed or changed) is a release.
- MUST: copy `templates/.github/` into every app. `ci.yml` runs fmt, clippy, tests, cargo-deny, the MSRV check, `oc-check`, shellcheck and the static musl builds for x86_64 and aarch64 with artefacts.
- MUST: `release.yml` checks that the tag, the Cargo version and the CHANGELOG heading agree, builds both targets, runs `templates/scripts/bundle.sh`, attests build provenance and publishes the release.
- MUST: Dependabot for `cargo` and `github-actions`, weekly, grouped, prefix `chore(deps)`.

## Install

- MUST: every release ships offline bundles: binary, `install.sh`, systemd unit, nginx template, example config, licences and `SHA256SUMS`, plus a vendored source archive.
- MUST: every app ships `install.sh`. It does the whole lifecycle under systemd: first install, update, rollback. Whether to use it is the deployer's choice.
- MUST: with `--archive FILE` the installer never touches the network. Without it, it downloads the release bundle it belongs to and verifies it. It never calls sudo in either mode.
- MUST: the whole script is one function called on the last line. *Why: a cut-off download then runs nothing.*
- MUST: an update runs the new binary's `check-config` against the existing config before changing anything. *Why: an update must never restart onto a config the new version refuses.* Every question the installer asks has an environment variable that presets it, so the script runs unattended.
- MUST: every app has a `check-config` subcommand. It loads and validates the config and the app's settings without opening or migrating data. It exits 0 and prints `bind=…` and `public_url=…` lines, or exits non-zero with the reason.
- MUST: every app answers `/healthz`.
- MUST: `AIRGAP.md` documents install, update and rollback, TLS with a local CA or self-signed certificate, plain http, and an offline build. See `docs/AIRGAP.md`.

## Deployment

- MUST: bind address, `public_url`, `trusted_proxies`, data directory and install prefix are configuration. The defaults (`127.0.0.1:8080`, loopback proxies) are a safe start, not a requirement.
- MUST: `public_url` (the address users see) is required unless the app is reached via localhost. It drives the host and origin checks, the OIDC redirect URI and the passkey relying-party ID.
- MAY: `extra_hosts` adds bare host names the host check accepts (for example a short-link domain); writes still need the main origin.
- SHOULD: https, with TLS terminated by a reverse proxy. The app does not terminate TLS itself.
- MAY: plain http on a LAN, when the deployer sets `allow_plain_http = true`; without it the app refuses an http `public_url` on a non-loopback host. *Why opt-in: passwords and cookies then cross the network in clear, a choice to make knowingly.*
- MAY: any reverse proxy, or none. A proxy forwards `Host`, `X-Forwarded-For` and `X-Forwarded-Proto`, does not buffer SSE routes and adds no security headers; `templates/deploy/nginx.conf` is the reference.
- SHOULD: run under the hardened systemd unit from `templates/deploy/app.service` (`DynamicUser`, `ProtectSystem=strict`, `NoNewPrivileges`, `MemoryDenyWriteExecute`, `UMask=0077`, `Type=notify` with watchdog). MAY: another service manager or a container.
- MUST: every repo has README, CHANGELOG, SECURITY.md, AIRGAP.md, THIRD-PARTY-LICENSES, LICENSE and an AGENTS.md naming the manifest version it follows.
