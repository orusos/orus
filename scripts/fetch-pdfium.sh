#!/bin/sh
# Fetch the pdfium library the Viewer draws PDFs with into out/pdfium: libpdfium.so, its
# LICENSE and the licenses of what it contains. The build is bblanchon/pdfium-binaries,
# pinned by release and SHA-256; the download is refused if the hash differs.
set -eu

release=chromium/8111
sha_arm64=042de247138f96341534cccf24ed21c2550a7245b936bb4a60d2340120000f05
sha_x64=bdc60126399f39fe768566a18d4be055a7b3681bc9e248e8a6656b4f6cae0aa2

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/out/pdfium"

case "$(dpkg --print-architecture 2>/dev/null || uname -m)" in
    arm64|aarch64) arch=arm64; sha=$sha_arm64 ;;
    amd64|x86_64) arch=x64; sha=$sha_x64 ;;
    *) echo "fetch-pdfium: no pdfium build for this architecture" >&2; exit 1 ;;
esac

if [ -f "$out/libpdfium.so" ] && [ "$(cat "$out/.release" 2>/dev/null)" = "$release-$arch" ]; then
    exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
file="pdfium-linux-$arch.tgz"
curl -fsSL -o "$work/$file" \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/$release/$file"
echo "$sha  $work/$file" | sha256sum -c - >/dev/null

rm -rf "$out"
mkdir -p "$out"
tar -xzf "$work/$file" -C "$work" lib/libpdfium.so LICENSE licenses
install -m 644 "$work/lib/libpdfium.so" "$out/"
cp "$work/LICENSE" "$out/LICENSE"
cp -R "$work/licenses" "$out/licenses"
echo "$release-$arch" > "$out/.release"
