#!/usr/bin/env bash
# Refreshes the third-party files in crates/oc-kit/assets. Maintainers run it; CI never does.
# Needs curl, unzip, python3 and pyftsubset (fonttools with brotli).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
assets="$root/crates/oc-kit/assets"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

atkinson=https://raw.githubusercontent.com/googlefonts/atkinson-hyperlegible-next/7925f50f649b3813257faf2f4c0b381011f434f1

fetch() {
  local url=$1 sum=$2 out
  out="$work/$(basename "$url")"
  curl -fsSL -o "$out" "$url"
  echo "$sum  $out" | sha256sum -c --quiet -
}

fetch "$atkinson/fonts/ttf/AtkinsonHyperlegibleNext-Regular.ttf" 88ed5c31a71584c7772963b02d04bef1eb7e3d2e9c8b9cb204339b1f82cf432c
fetch "$atkinson/fonts/ttf/AtkinsonHyperlegibleNext-Medium.ttf" dd50b08b3c560846097d23baaaf6a97ffa20dd077115d23c59df68083b9ea05e
fetch "$atkinson/fonts/ttf/AtkinsonHyperlegibleNext-SemiBold.ttf" 8e2fff779f35ac25239656bb0f3b2bd35959a2009e3dd21a12141c3b9030362b
fetch "$atkinson/OFL.txt" aca6a428580965d2297d1b718042dd427c2a9443ece3b0d02d758e161e0c4030
fetch https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip 6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf
fetch https://registry.npmjs.org/lucide-static/-/lucide-static-1.70.0.tgz 3d1f09df1c468abe512ee6d6596db0dc773fa46c61050e365f0c5fee5120f16a
fetch https://registry.npmjs.org/htmx.org/-/htmx.org-2.0.11.tgz 165a2655073e9e6e55e307a7c74e8ebcdb76afb7701f9c16cfd106a13add408e
fetch https://registry.npmjs.org/htmx-ext-sse/-/htmx-ext-sse-2.2.4.tgz 90b46f5ab915ba2f64e6348448c8dba509277a47e15c125ffb65754eb4f6b5dc

mkdir -p "$assets/fonts" "$assets/icons" "$assets/vendor"

unicodes="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+2000-206F,U+2074,U+20AC,U+2122,U+2190-2199,U+2212,U+2215,U+2713,U+2717,U+26A0"
subset() {
  pyftsubset "$1" --output-file="$2" --flavor=woff2 --layout-features='*' --unicodes="$unicodes"
}
subset "$work/AtkinsonHyperlegibleNext-Regular.ttf" "$assets/fonts/atkinson-next-400.woff2"
subset "$work/AtkinsonHyperlegibleNext-Medium.ttf" "$assets/fonts/atkinson-next-500.woff2"
subset "$work/AtkinsonHyperlegibleNext-SemiBold.ttf" "$assets/fonts/atkinson-next-600.woff2"
cp "$work/OFL.txt" "$assets/fonts/OFL-AtkinsonHyperlegibleNext.txt"
unzip -q -o "$work/JetBrainsMono-2.304.zip" fonts/ttf/JetBrainsMono-Regular.ttf OFL.txt -d "$work/jbm"
subset "$work/jbm/fonts/ttf/JetBrainsMono-Regular.ttf" "$assets/fonts/jetbrains-mono-400.woff2"
cp "$work/jbm/OFL.txt" "$assets/fonts/OFL-JetBrainsMono.txt"

mkdir -p "$work/lucide" "$work/htmx" "$work/sse"
tar xzf "$work/lucide-static-1.70.0.tgz" -C "$work/lucide"
tar xzf "$work/htmx.org-2.0.11.tgz" -C "$work/htmx"
tar xzf "$work/htmx-ext-sse-2.2.4.tgz" -C "$work/sse"

icons="copy check filter filter-x pin archive trash-2 undo-2 pencil share-2 download search settings x plus history link tag eye eye-off sun moon monitor a-large-small info circle-check triangle-alert circle-x ellipsis chevron-right chevron-down notebook folder user log-out panel-left book-open key-round list"
rm -f "$assets"/icons/*.svg
# Sizing and stroke come from CSS, so the SVG keeps only geometry.
python3 -I - "$work/lucide/package/icons" "$assets/icons" $icons <<'PY'
import re, sys
src, dst, names = sys.argv[1], sys.argv[2], sys.argv[3:]
for n in names:
    s = open(f"{src}/{n}.svg").read()
    s = re.sub(r"<!--.*?-->", "", s, flags=re.S)
    s = re.sub(r'\s(class|width|height|stroke-width)="[^"]*"', "", s.split("<svg", 1)[0] + "<svg" + s.split("<svg", 1)[1].split(">", 1)[0]) + ">" + s.split("<svg", 1)[1].split(">", 1)[1]
    s = s.replace("<svg", '<svg class="oc-icon" aria-hidden="true" focusable="false"', 1)
    s = re.sub(r"\s+", " ", s).replace("> <", "><").strip()
    open(f"{dst}/{n}.svg", "w").write(s + "\n")
PY
cp "$work/lucide/package/LICENSE" "$assets/icons/LICENSE-lucide.txt"

cp "$work/htmx/package/dist/htmx.min.js" "$assets/vendor/htmx.min.js"
cp "$work/htmx/package/LICENSE" "$assets/vendor/LICENSE-htmx.txt"
cp "$work/sse/package/dist/sse.min.js" "$assets/vendor/htmx-ext-sse.js"
cp "$work/sse/package/LICENSE" "$assets/vendor/LICENSE-htmx-ext-sse.txt"

echo "Vendored assets written to $assets"
