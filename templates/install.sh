#!/bin/sh
# Installer for an oc app: first install, update and rollback under systemd, or the binary alone
# for one user. Never calls sudo; with --archive it never uses the network.
# bundle.sh fills in the app, version and repository below.

# Everything sits in main, called on the last line, so a cut-off download runs nothing.
main() {
  set -eu

  app=@APP@
  version=@VERSION@
  repo=@REPO@

  usage() {
    echo "Usage: install.sh [--archive FILE] [--system] [--prefix DIR]" >&2
    exit 2
  }
  die() { echo "$*" >&2; exit 1; }
  say() { echo "$*"; }

  # Placeholders bundle.sh has not replaced, as in the template itself.
  baked() { case "$1" in @*@) return 1 ;; esac; }

  env_of() { eval "printf '%s' \"\${${envp}_$1:-}\""; }

  interactive() {
    [ "$(env_of ASSUME_DEFAULTS)" = 1 ] && return 1
    # stdin is the script itself under curl | sh, so questions go to the terminal.
    [ -t 0 ] || (true </dev/tty) 2>/dev/null
  }

  # ask VAR PROMPT DEFAULT
  ask() {
    ask_v=""
    if interactive; then
      printf '%s [%s]: ' "$2" "$3" >/dev/tty
      read -r ask_v </dev/tty || ask_v=""
    fi
    [ -n "$ask_v" ] || ask_v=$3
    eval "$1=\$ask_v"
  }

  # put_secret NAME VALUE: a root-only credential file, handed to the service with LoadCredential=.
  put_secret() {
    case "$1" in ''|*[!A-Za-z0-9_-]*) die "Invalid credential name: $1" ;; esac
    mkdir -p "$work/credentials"
    (umask 077 && printf '%s' "$2" >"$work/credentials/$1")
    echo "LoadCredential=$1:/etc/$app/credentials/$1" >>"$work/credentials.conf"
  }

  # The app's own first-install questions go here. Use `ask`, keep secrets out of the config with
  # `put_secret` (the config refers to them as file:///run/credentials/$app.service/NAME), and
  # append whole [sections] to "$staged", the config being written.
  ask_app() {
    :
  }

  fetch() {
    if ! baked "$app" || ! baked "$version" || ! baked "$repo"; then
      die "This copy of install.sh has no release baked in; pass --archive FILE."
    fi
    command -v curl >/dev/null 2>&1 || die "curl is needed to download the release; or pass --archive FILE."
    case "$(uname -m)" in
      x86_64|amd64) target=x86_64-unknown-linux-musl ;;
      aarch64|arm64) target=aarch64-unknown-linux-musl ;;
      *) die "No release build for $(uname -m)." ;;
    esac
    name="$app-$version-$target-offline.tar.gz"
    base="https://github.com/$repo/releases/download/v$version"
    say "Downloading $name"
    curl -fsSL -o "$work/$name" "$base/$name" || die "Download failed: $base/$name"
    curl -fsSL -o "$work/SHA256SUMS" "$base/SHA256SUMS" || die "Download failed: $base/SHA256SUMS"
    grep "  $name\$" "$work/SHA256SUMS" >"$work/sum" || die "SHA256SUMS does not list $name."
    (cd "$work" && sha256sum -c --quiet sum >/dev/null 2>&1) || die "Checksum mismatch: $name. The download is damaged or was altered."
    verify_provenance "$work/$name"
    archive="$work/$name"
  }

  verify_provenance() {
    mode=$(env_of VERIFY)
    [ "$mode" = 0 ] && { say "Skipping the provenance check (${envp}_VERIFY=0)."; return 0; }
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1 && gh api rate_limit --silent >/dev/null 2>&1; then
      gh attestation verify "$1" --repo "$repo" >"$work/attest.log" 2>&1 || {
        cat "$work/attest.log" >&2
        die "Provenance check failed for $(basename "$1"). Nothing was installed."
      }
      say "Provenance verified."
    elif [ "$mode" = 1 ]; then
      die "${envp}_VERIFY=1 needs gh, signed in and able to reach GitHub, to check provenance."
    else
      echo "Warning: gh is missing, not signed in or offline; provenance not checked. Set ${envp}_VERIFY=1 to require it." >&2
    fi
  }

  is_loopback_bind() {
    case "$1" in 127.*|'[::1]':*) return 0 ;; *) return 1 ;; esac
  }

  # stage_config BIND PUBLIC_URL: the example config with both values set, in "$staged".
  stage_config() {
    awk -v bind="$1" -v url="$2" '
      function rest() {
        if (!b) print "bind = \"" bind "\""
        if (!u && url != "") print "public_url = \"" url "\""
        b = u = 1
      }
      /^\[/ { rest() }
      !b && /^#? *bind *=/ { print "bind = \"" bind "\""; b = 1; next }
      !u && /^#? *public_url *=/ { if (url != "") print "public_url = \"" url "\""; else print; u = 1; next }
      { print }
      END { rest() }
    ' "$bundle/config.example.toml" >"$staged"
  }

  # check_config BIN CONFIG: runs check-config with the unit's Environment=, sets cc_bind and cc_url.
  check_config() {
    cc_bin=$1
    cc_cfg=$2
    env_line=""
    [ -f "$unit" ] && env_line=$(systemctl show -p Environment --value "$app" 2>/dev/null || true)
    # The unit's Environment= words are KEY=VALUE pairs; run the check with them.
    if ! (eval "set -- $env_line"; env "$@" "$cc_bin" --config "$cc_cfg" check-config) >"$work/check.out" 2>&1; then
      return 1
    fi
    cc_bind=$(sed -n 's/^bind=//p' "$work/check.out")
    cc_url=$(sed -n 's/^public_url=//p' "$work/check.out")
    [ -n "$cc_bind" ] && [ -n "$cc_url" ]
  }

  # wait_healthy BIND PUBLIC_URL
  wait_healthy() {
    if command -v curl >/dev/null 2>&1; then
      port=${1##*:}
      addr=${1%:*}
      case "$addr" in 0.0.0.0) addr=127.0.0.1 ;; '[::]') addr='[::1]' ;; esac
      host=${2#*://}
      host=${host%%/*}
      i=0
      while [ $i -lt 30 ]; do
        curl -fsS -o /dev/null --max-time 2 -H "Host: $host" "http://$addr:$port/healthz" 2>/dev/null && return 0
        i=$((i + 1))
        sleep 1
      done
      return 1
    fi
    # Without curl, Type=notify readiness is all there is.
    systemctl is-active --quiet "$app"
  }

  install_bin() {
    mkdir -p "$(dirname "$bin")" && install -m 0755 "$1" "$bin.new" && mv -f "$bin.new" "$bin"
  }

  # The unit names /usr/local/bin; point it at the chosen prefix.
  stage_unit() {
    sed "s|/usr/local/bin/|$prefix/bin/|g" "$bundle/$app.service" >"$work/unit"
  }

  sum_of() { sha256sum "$1" | cut -d' ' -f1; }

  summary() {
    say ""
    say "$app $version is running."
    say "  Address:  $cc_url"
    say "  Config:   /etc/$app/config.toml"
    say "  Status:   systemctl status $app"
    say "  Logs:     journalctl -u $app -f"
  }

  first_install() {
    staged="$work/config.toml"
    if [ -f "/etc/$app/config.toml" ]; then
      say "Keeping the existing /etc/$app/config.toml."
      cp "/etc/$app/config.toml" "$staged"
    else
      bind=$(env_of BIND)
      [ -n "$bind" ] || ask bind "Listen address" 127.0.0.1:8080
      url=$(env_of PUBLIC_URL)
      if [ -z "$url" ]; then
        if is_loopback_bind "$bind"; then
          ask url "Public URL users type (empty: http://localhost only)" ""
        else
          ask url "Public URL users type, e.g. https://$app.example.org" ""
        fi
      fi
      if [ -z "$url" ] && ! is_loopback_bind "$bind"; then
        die "public_url is required when binding to $bind; set ${envp}_PUBLIC_URL. Nothing was written."
      fi
      case "$bind$url" in *[\"\\]*) die "Quotes and backslashes are not allowed in bind or public_url." ;; esac
      stage_config "$bind" "$url"
      ask_app
    fi
    check_config "$bundle/$app" "$staged" || {
      cat "$work/check.out" >&2
      die "check-config refused the config. Nothing was written."
    }

    install_bin "$bundle/$app"
    mkdir -p "/etc/$app"
    if [ ! -f "/etc/$app/config.toml" ]; then
      # World-readable because the service runs as a dynamic user; secrets go in as credentials.
      install -m 0644 "$staged" "/etc/$app/config.toml"
      say "Wrote /etc/$app/config.toml"
    fi
    if [ -f "$work/credentials.conf" ]; then
      install -d -m 0700 "/etc/$app/credentials"
      for f in "$work/credentials"/*; do
        install -m 0600 "$f" "/etc/$app/credentials/$(basename "$f")"
      done
      mkdir -p "/etc/systemd/system/$app.service.d"
      { echo "[Service]"; cat "$work/credentials.conf"; } >"/etc/systemd/system/$app.service.d/credentials.conf"
    fi
    stage_unit
    install -m 0644 "$work/unit" "$unit"
    sum_of "$unit" >"/etc/$app/.unit.sha256"
    systemctl daemon-reload
    if ! systemctl enable --now "$app" || ! wait_healthy "$cc_bind" "$cc_url"; then
      die "$app did not become healthy. See: journalctl -u $app -n 50"
    fi
    summary
  }

  backup() {
    bdir="/var/backups/$app/$(date -u +%Y%m%dT%H%M%SZ)"
    mkdir -p "$bdir"
    chmod 0700 "/var/backups/$app"
    # Stopped first so the SQLite files, WAL included, are consistent.
    systemctl stop "$app"
    # DynamicUser keeps StateDirectory under /var/lib/private.
    state=""
    for d in "/var/lib/private/$app" "/var/lib/$app"; do
      if [ -d "$d" ]; then state=$d; break; fi
    done
    if ! { cp -a "$bin" "$bdir/$app" && cp -a "$unit" "$bdir/$app.service" &&
      { [ ! -f "/etc/$app/.unit.sha256" ] || cp -a "/etc/$app/.unit.sha256" "$bdir/unit.sha256"; } &&
      { [ -z "$state" ] || cp -a "$state" "$bdir/state"; }; }; then
      rm -rf "$bdir"
      systemctl start "$app" || true
      die "Backup to $bdir failed; nothing changed; $old keeps running."
    fi
    say "Backed up to $bdir"
  }

  prune_backups() {
    find "/var/backups/$app" -mindepth 1 -maxdepth 1 -type d | sort -r | tail -n +3 | while read -r d; do
      rm -rf "$d"
    done
  }

  rollback() {
    systemctl stop "$app" 2>/dev/null || true
    cp -a "$bdir/$app" "$bin.new"
    mv -f "$bin.new" "$bin"
    cp -a "$bdir/$app.service" "$unit"
    if [ -f "$bdir/unit.sha256" ]; then cp -a "$bdir/unit.sha256" "/etc/$app/.unit.sha256"; fi
    if [ -n "$state" ]; then
      rm -rf "$state"
      cp -a "$bdir/state" "$state"
    fi
    systemctl daemon-reload
    systemctl reset-failed "$app" 2>/dev/null || true
    systemctl start "$app" || echo "The old version did not start either." >&2
    die "update to $version failed, rolled back to $old. See: journalctl -u $app -n 50"
  }

  # Runs as `replace || rollback`, where set -e is off, so every step is chained.
  replace() {
    install_bin "$bundle/$app" && stage_unit || return 1
    new_sum=$(sum_of "$work/unit")
    cur_sum=$(sum_of "$unit")
    if [ "$(cat "/etc/$app/.unit.sha256" 2>/dev/null)" = "$cur_sum" ] || [ "$cur_sum" = "$new_sum" ]; then
      install -m 0644 "$work/unit" "$unit" && echo "$new_sum" >"/etc/$app/.unit.sha256" || return 1
    else
      install -m 0644 "$work/unit" "$unit.new" || return 1
      say "$unit was edited, so it stays; the new unit is $unit.new."
    fi
    systemctl daemon-reload
  }

  update() {
    check_config "$bundle/$app" "/etc/$app/config.toml" || {
      cat "$work/check.out" >&2
      die "check-config of $version refused /etc/$app/config.toml; nothing changed; $old keeps running."
    }
    backup
    prune_backups

    replace || rollback

    if systemctl is-enabled --quiet "$app"; then
      systemctl restart "$app" || rollback
    else
      systemctl enable --now "$app" || rollback
    fi
    wait_healthy "$cc_bind" "$cc_url" || rollback
    say "Updated $app from $old to $version."
    summary
  }

  archive=""
  system=0
  prefix="${HOME:-}/.local"
  while [ $# -gt 0 ]; do
    case "$1" in
      --archive) [ $# -ge 2 ] || usage; archive=$2; shift 2 ;;
      --system) system=1; prefix=/usr/local; shift ;;
      --prefix) [ $# -ge 2 ] || usage; prefix=$2; shift 2 ;;
      -h|--help) usage ;;
      *) echo "Unknown option: $1" >&2; usage ;;
    esac
  done
  if [ -n "$archive" ] && [ ! -f "$archive" ]; then die "Archive not found: $archive"; fi
  if [ "$system" -eq 1 ] && [ "$(id -u)" -ne 0 ]; then
    die "Run as root for --system; this script never calls sudo."
  fi

  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  envp=$(printf '%s' "$app" | tr 'a-z-' 'A-Z_')
  [ -n "$archive" ] || fetch

  mkdir "$work/x"
  tar -C "$work/x" -xzf "$archive"
  set -- "$work/x"/*/
  [ $# -eq 1 ] && [ -d "$1" ] || die "The archive should contain exactly one directory."
  bundle=${1%/}

  if ! (cd "$bundle" && sha256sum -c --quiet SHA256SUMS) >"$work/sums.log" 2>&1; then
    bad=$(grep -v ': OK$' "$work/sums.log" | head -n 1 | cut -d: -f1)
    die "Checksum mismatch: ${bad:-SHA256SUMS}. The bundle is damaged or was altered."
  fi

  name=$(head -n 1 "$bundle/name")
  case "$name" in ''|*[!a-z0-9_-]*) die "The bundle names an invalid app: $name" ;; esac
  if baked "$app"; then
    [ "$name" = "$app" ] || die "The bundle is for $name, this installer for $app."
  else
    app=$name
    envp=$(printf '%s' "$app" | tr 'a-z-' 'A-Z_')
  fi
  bin="$prefix/bin/$app"

  if [ "$system" -eq 1 ]; then
    version=$("$bundle/$app" --version | cut -d' ' -f2)
    unit="/etc/systemd/system/$app.service"
    if [ ! -f "$unit" ]; then
      first_install
    else
      [ -x "$bin" ] || die "$unit exists but $bin does not; restore it or remove the unit."
      old=$("$bin" --version | cut -d' ' -f2)
      force=$(env_of FORCE)
      if [ "$old" = "$version" ] && [ "$force" != 1 ]; then
        say "$app is already at $version; set ${envp}_FORCE=1 to reinstall."
        exit 0
      fi
      # Migrations ignore unknown ones, so an older binary would start quietly on a newer schema.
      if [ "$old" != "$version" ] && [ "$(printf '%s\n%s\n' "$old" "$version" | sort -V | head -n 1)" = "$version" ] && [ "$force" != 1 ]; then
        die "downgrade from $old to $version; set ${envp}_FORCE=1 to do it anyway."
      fi
      update
    fi
  else
    install_bin "$bundle/$app"
    say "Installed $bin"
  fi

  doc="$prefix/share/$app"
  mkdir -p "$doc"
  for f in nginx.conf AIRGAP.md THIRD-PARTY-LICENSES config.example.toml; do
    if [ -f "$bundle/$f" ]; then install -m 0644 "$bundle/$f" "$doc/$f"; fi
  done
  say "Reverse proxy template: $doc/nginx.conf"
}

main "$@"
