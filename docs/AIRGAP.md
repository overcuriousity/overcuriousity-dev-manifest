# Airgapped install

Every release can be installed on a machine with no network access. The steps below use the demo (`demo`). Other oc apps work the same way under their own name.

## What to carry across

Download these from the release page on a connected machine:

| File | Purpose |
|---|---|
| `demo-<version>-x86_64-unknown-linux-musl-offline.tar.gz` (or `aarch64`) | static binary, `install.sh`, systemd unit, nginx template, example config, licences, `SHA256SUMS` |
| `install.sh` | the installer, also inside each bundle |
| `SHA256SUMS` | checksums of the release files |
| `demo-<version>-src-vendored.tar.gz` | optional: source with every dependency vendored, to build without network |

Check the downloads before you carry them across:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

The release also carries build-provenance attestations. On a connected machine, check them with `gh attestation verify <file> --repo overcuriousity/overcuriousity-dev-manifest`.

## Install

The installer never uses the network and never calls `sudo`.

For your user only, into `~/.local/bin`:

```sh
sh install.sh --archive demo-<version>-x86_64-unknown-linux-musl-offline.tar.gz
```

As a system service, run as root. This installs to `/usr/local/bin`, sets up a hardened systemd unit and creates `/etc/demo/config.toml`:

```sh
sh install.sh --archive demo-<version>-x86_64-unknown-linux-musl-offline.tar.gz --system
```

The installer checks every file against the bundle's `SHA256SUMS`. If any file has changed, it stops with "Checksum mismatch".

## Update and rollback

Run the new release's `install.sh` the same way as the first install (`--system`, as root). It finds the installed service and updates it:

1. It runs the new binary's `check-config` against `/etc/demo/config.toml`. If that fails, nothing changes and the old version keeps running.
2. It stops the service and copies the binary, the unit and the state directory to `/var/backups/demo/<UTC timestamp>/`. The newest two backups are kept; older ones are deleted.
3. It replaces the binary. If you have not edited the unit, it replaces that too; otherwise your unit stays and the new one is written as `demo.service.new`. Drop-ins are never touched.
4. It restarts the service and waits for it to report ready and answer `/healthz`. If either fails, it restores the backup, starts the old version and exits non-zero with "update to X failed, rolled back to Y".

The installer refuses to install an older version over a newer one ("downgrade from Y to X"), because an older binary would start on a newer database schema. It also stops with "already at X" for the same version. `DEMO_FORCE=1` overrides both.

The check runs as root, outside the service. It uses the unit's `Environment=` lines, so drop-in overrides count. It does not read `EnvironmentFile=`, and a secret file it cannot read (such as one under `/run/credentials/`) is skipped. A setting that comes only from there is first tested when the service starts, and a failure then triggers the rollback.

If the rollback itself fails, the installer leaves the service stopped and disabled and names the backup directory. To go back by hand, stop the service, then copy from a backup directory: `demo` to `/usr/local/bin/demo`, `demo.service` to `/etc/systemd/system/`, and `state` to `/var/lib/private/demo` (`cp -a`, after moving the current one aside). Then run `systemctl daemon-reload` and `systemctl enable --now demo`.

## Installer variables

Set these in the environment of `sh install.sh`. Every question the installer asks has one, so it runs without a terminal.

| Variable | Effect |
|---|---|
| `DEMO_FORCE=1` | reinstall the same version, or install an older one |
| `DEMO_VERIFY` | unset: verify build provenance if `gh` is installed and signed in; `1`: require it; `0`: skip it. Only an online install verifies |
| `DEMO_PUBLIC_URL` | the address users type; first install |
| `DEMO_BIND` | the listen address, default `127.0.0.1:8080`; first install |
| `DEMO_ASSUME_DEFAULTS=1` | take the defaults without asking, even with a terminal |

Without a terminal the installer takes the defaults. If a required value such as `public_url` is missing, it stops before writing anything.

## TLS on an airgapped network

The app never terminates TLS itself. By default it listens on `127.0.0.1:8080`, and nginx on the same machine serves https. For plain http instead, see below.

Use either of these:
- A certificate from your local CA. This is best: browsers trust it once the CA is installed on each client.
- A self-signed certificate. Browsers will warn until each client trusts it.

```sh
openssl req -x509 -newkey rsa:3072 -sha256 -days 825 -nodes \
  -keyout /etc/ssl/private/journal.lan.key -out /etc/ssl/certs/journal.lan.pem \
  -subj "/CN=journal.lan" -addext "subjectAltName=DNS:journal.lan"
```

Keep secrets out of `config.toml`; it is world-readable so the service's dynamic user can read it. Put a secret in a root-only file and hand it to the service as a credential:

```sh
printf '%s' 'the-client-secret' | install -m 0600 /dev/stdin /etc/demo/oidc-secret
systemctl edit demo   # add: [Service] LoadCredential=oidc-secret:/etc/demo/oidc-secret
```

and in `config.toml`: `client_secret = "file:///run/credentials/demo.service/oidc-secret"`.

Then:
1. Copy `nginx.conf` from the bundle (also in `~/.local/share/demo/` or `/usr/local/share/demo/`) to `/etc/nginx/conf.d/demo.conf`. Replace `journal.example.org` and the certificate paths, then reload nginx.
2. Set the address users type in `/etc/demo/config.toml`:

   ```toml
   public_url = "https://journal.lan"
   ```

The app refuses to start if `public_url` is not https, except for `localhost` or with `allow_plain_http`.

## Plain http on a LAN

Without TLS, passwords and session cookies cross the network unencrypted, the session cookie loses `Secure` and passkeys are off. If that is acceptable on your network, opt in explicitly in `/etc/demo/config.toml`:

```toml
allow_plain_http = true
public_url = "http://journal.lan"
```

Keep nginx in front on port 80, or skip it and bind the app to the LAN with `bind = "0.0.0.0:8080"` and `public_url = "http://journal.lan:8080"`.

## Build from source without network

```sh
tar -xzf demo-<version>-src-vendored.tar.gz
cd demo-<version>
SQLX_OFFLINE=true cargo build --release --offline --locked -p demo
```

The archive's `.cargo/config.toml` points Cargo at the bundled `vendor/` directory. Fonts, icons and htmx are already inside the source tree.

## First start

On first start the demo creates the user `demo` and prints its password once to the log (`journalctl -u demo`).

For a user install, add more accounts with `demo user add <name> [--admin]`. For the system service, run the command inside the service's sandbox, so it uses the service's data directory and user:

```sh
systemd-run --pty --wait -p DynamicUser=yes -p StateDirectory=demo \
  /usr/local/bin/demo --config /etc/demo/config.toml user add <name> [--admin]
```
