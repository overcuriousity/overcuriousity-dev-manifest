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
