#!/bin/sh
# Exercises templates/install.sh against a fake bundle: usage, user install, tamper detection,
# --system without root, --prefix with --system in either order, question presets, an unbaked
# copy without --archive, and a cut-off script.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

make_bundle() {
  dir="$work/build/oc-install-test-1.0.0-test"
  rm -rf "$work/build"
  mkdir -p "$dir"
  # check-config runs under env -i, so the config copy goes to a path baked in here.
  cat > "$dir/oc-install-test" <<EOF
#!/bin/sh
case "\${1:-}" in
  --version) echo "oc-install-test 1.0.0" ;;
  --config) cp "\$2" "$work/checked.toml"; echo bind=127.0.0.1:9001; echo public_url=http://localhost:9001 ;;
  *) echo oc-install-test ;;
esac
EOF
  chmod 755 "$dir/oc-install-test"
  printf 'oc-install-test\n' > "$dir/name"
  printf 'server {}\n' > "$dir/nginx.conf"
  printf '[Unit]\n' > "$dir/oc-install-test.service"
  printf 'bind = "127.0.0.1:8080"\n' > "$dir/config.example.toml"
  (cd "$dir" && sha256sum oc-install-test name nginx.conf oc-install-test.service config.example.toml > SHA256SUMS)
  if [ "${1:-}" = tamper ]; then printf 'evil\n' >> "$dir/oc-install-test"; fi
  tar -C "$work/build" -czf "$work/bundle.tar.gz" oc-install-test-1.0.0-test
}

fail() { echo "FAIL: $*" >&2; exit 1; }

for arg in --help --bogus; do
  set +e
  sh "$here/templates/install.sh" "$arg" > "$work/out" 2>&1
  code=$?
  set -e
  [ "$code" -eq 2 ] || fail "$arg exited $code, not 2"
  grep -q "^Usage: install.sh" "$work/out" || fail "$arg printed no usage"
done

make_bundle
HOME="$work/home" sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" > "$work/out" 2>&1 || fail "user install exited non-zero: $(cat "$work/out")"
[ -x "$work/home/.local/bin/oc-install-test" ] || fail "binary not installed to ~/.local/bin"
grep -q "nginx.conf" "$work/out" || fail "install did not point at the nginx template"

make_bundle tamper
if HOME="$work/home2" sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" > "$work/out" 2>&1; then fail "tampered bundle was installed"; fi
grep -q "Checksum mismatch" "$work/out" || fail "no checksum message: $(cat "$work/out")"

if [ "$(id -u)" -ne 0 ]; then
  make_bundle
  if sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" --system > "$work/out" 2>&1; then fail "--system ran without root"; fi
  grep -q "Run as root for --system" "$work/out" || fail "no root message"
fi

# A fake root gets through first_install's binary step, then fails at /etc, which only real root
# can write. Skipped as real root, where it would install for real.
if [ "$(id -u)" -ne 0 ]; then
  mkdir -p "$work/fakebin"
  printf '#!/bin/sh\necho 0\n' > "$work/fakebin/id"
  chmod 755 "$work/fakebin/id"
  make_bundle
  for order in "--prefix $work/p1 --system" "--system --prefix $work/p1"; do
    rm -rf "$work/p1" "$work/checked.toml"
    # shellcheck disable=SC2086 # $order is two options and their argument, split on purpose
    PATH="$work/fakebin:$PATH" OC_INSTALL_TEST_ASSUME_DEFAULTS=1 OC_INSTALL_TEST_BIND=127.0.0.1:9001 OC_INSTALL_TEST_PUBLIC_URL=http://localhost:9001 \
      sh "$here/templates/install.sh" --archive "$work/bundle.tar.gz" $order > "$work/out" 2>&1 < /dev/null || true
    [ -x "$work/p1/bin/oc-install-test" ] || fail "$order did not install to the prefix: $(cat "$work/out")"
    grep -q '^bind = "127.0.0.1:9001"$' "$work/checked.toml" || fail "OC_INSTALL_TEST_BIND did not preset the bind question"
    grep -q '^public_url = "http://localhost:9001"$' "$work/checked.toml" || fail "OC_INSTALL_TEST_PUBLIC_URL did not preset the URL question"
  done
fi

if HOME="$work/home3" sh "$here/templates/install.sh" > "$work/out" 2>&1; then fail "unbaked install.sh ran without --archive"; fi
grep -q "no release baked in" "$work/out" || fail "no unbaked message: $(cat "$work/out")"

make_bundle
size=$(wc -c < "$here/templates/install.sh")
head -c $((size / 2)) "$here/templates/install.sh" > "$work/half.sh"
HOME="$work/home4" sh "$work/half.sh" --archive "$work/bundle.tar.gz" > "$work/out" 2>&1 || true
[ ! -e "$work/home4" ] || fail "a cut-off install.sh did something"

echo "install.sh: all checks passed"
