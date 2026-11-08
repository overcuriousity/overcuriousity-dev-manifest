#!/bin/sh
# Exercises templates/install.sh against a fake bundle: user install, tamper detection, --system without root.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

make_bundle() {
  dir="$work/build/demo-1.0.0-test"
  rm -rf "$work/build"
  mkdir -p "$dir"
  printf '#!/bin/sh\necho demo\n' > "$dir/demo"
  chmod 755 "$dir/demo"
  printf 'demo\n' > "$dir/name"
  printf 'server {}\n' > "$dir/nginx.conf"
  printf '[Unit]\n' > "$dir/demo.service"
  printf 'bind = "127.0.0.1:8080"\n' > "$dir/config.example.toml"
  (cd "$dir" && sha256sum demo name nginx.conf demo.service config.example.toml > SHA256SUMS)
  if [ "${1:-}" = tamper ]; then printf 'evil\n' >> "$dir/demo"; fi
  tar -C "$work/build" -czf "$work/bundle.tar.gz" demo-1.0.0-test
}

fail() { echo "FAIL: $*" >&2; exit 1; }

make_bundle
HOME="$work/home" sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" > "$work/out" 2>&1 || fail "user install exited non-zero: $(cat "$work/out")"
[ -x "$work/home/.local/bin/demo" ] || fail "binary not installed to ~/.local/bin"
grep -q "nginx.conf" "$work/out" || fail "install did not point at the nginx template"

make_bundle tamper
if HOME="$work/home2" sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" > "$work/out" 2>&1; then fail "tampered bundle was installed"; fi
grep -q "Checksum mismatch" "$work/out" || fail "no checksum message: $(cat "$work/out")"

if [ "$(id -u)" -ne 0 ]; then
  make_bundle
  if sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" --system > "$work/out" 2>&1; then fail "--system ran without root"; fi
  grep -q "Run as root for --system" "$work/out" || fail "no root message"
fi

echo "install.sh: all checks passed"
