#!/bin/sh
# Builds the offline release bundles from the musl binaries in DIST/bin:
# one archive per target (binary, install files, licences, SHA256SUMS) plus a vendored source archive.
# Paths can be overridden: INSTALL_SH, SERVICE, NGINX, AIRGAP, CONFIG.
# REPO (owner/name, default $GITHUB_REPOSITORY) is the release install.sh downloads from.
set -eu

app=$1
version=$2
dist=$3
install_sh=${INSTALL_SH:-install.sh}
service=${SERVICE:-deploy/app.service}
nginx=${NGINX:-deploy/nginx.conf}
airgap=${AIRGAP:-AIRGAP.md}
config=${CONFIG:-config.example.toml}
repo=${REPO:-${GITHUB_REPOSITORY:-}}
[ -n "$repo" ] || { echo "Set REPO=owner/name; install.sh downloads its release from there." >&2; exit 1; }
out="$dist/out"
mkdir -p "$out"
sed -e "s|@APP@|$app|g" -e "s|@VERSION@|$version|g" -e "s|@REPO@|$repo|g" "$install_sh" > "$dist/install.sh"

mkdir -p "$dist/src/.cargo"
git archive --format=tar HEAD | tar -C "$dist/src" -xf -
# The printed source replacement points at the build path; inside the archive it is ./vendor.
cargo vendor --locked --versioned-dirs "$dist/src/vendor" | sed 's|^directory = .*|directory = "vendor"|' > "$dist/src/.cargo/config.toml"
tar -C "$dist" -czf "$out/$app-$version-src-vendored.tar.gz" --transform "s|^src|$app-$version|" src

cargo about generate --locked about.hbs > "$dist/THIRD-PARTY-LICENSES"
find . -path ./target -prune -o -path "./$dist" -prune -o -type f \
  \( -name 'OFL-*.txt' -o -name 'LICENSE-lucide.txt' -o -name 'LICENSE-htmx*.txt' \) -print | sort | while read -r f; do
  printf '\n\n==== %s ====\n\n' "$f" >> "$dist/THIRD-PARTY-LICENSES"
  cat "$f" >> "$dist/THIRD-PARTY-LICENSES"
done

for target in x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
  name="$app-$version-$target"
  dir="$dist/$name"
  mkdir -p "$dir"
  install -m 0755 "$dist/bin/bin-$target/$app" "$dir/$app"
  printf '%s\n' "$app" > "$dir/name"
  sed "s/@APP@/$app/g" "$service" > "$dir/$app.service"
  cp "$nginx" "$dir/nginx.conf"
  cp "$airgap" "$dir/AIRGAP.md"
  cp "$config" "$dir/config.example.toml"
  cp "$dist/THIRD-PARTY-LICENSES" "$dir/THIRD-PARTY-LICENSES"
  cp "$dist/install.sh" "$dir/install.sh"
  (cd "$dir" && sha256sum -- * | grep -v ' SHA256SUMS$' > SHA256SUMS)
  tar -C "$dist" -czf "$out/$name-offline.tar.gz" "$name"
done
cp "$dist/install.sh" "$out/install.sh"
(cd "$out" && sha256sum -- *.tar.gz install.sh > SHA256SUMS)
