#!/bin/sh
# Put the prebuilt runtime (waid-shell) in out/bin. runtime.lock pins one release per
# architecture by SHA-256; the download is refused if the hash differs.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/out/bin"

case "$(dpkg --print-architecture 2>/dev/null || uname -m)" in
    arm64|aarch64) arch=arm64 ;;
    amd64|x86_64) arch=x64 ;;
    *) echo "fetch-runtime: no runtime for this architecture" >&2; exit 1 ;;
esac

line=$(grep -v '^#' "$root/runtime.lock" | grep "^$arch " | tail -n 1 || true)
if [ -z "$line" ]; then
    echo "fetch-runtime: runtime.lock pins no runtime for $arch." >&2
    echo "  Build it from source instead: ORUS_DETIR_DIR=/path/to/detir scripts/build.sh" >&2
    exit 1
fi
sha=$(echo "$line" | awk '{print $2}')
url=$(echo "$line" | awk '{print $3}')

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL -o "$work/runtime.tgz" "$url"
echo "$sha  $work/runtime.tgz" | sha256sum -c - >/dev/null
# A renamed legacy browser binary is not the standalone WAID product. Validate
# the archive, target architecture and actual CLI before replacing staged output.
python3 "$root/scripts/runtime_artifact.py" --arch "$arch" --check-cli \
    --archive "$work/runtime.tgz" --destination "$out/waid-shell"
