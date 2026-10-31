#!/bin/sh
# Pack a built waid-shell for release and print the runtime.lock line for it.
# usage: scripts/pack-runtime.sh arm64|x64 /path/to/waid-shell URL
# Upload out/waid-shell-<arch>.tar.gz to URL, then add the printed line to runtime.lock.
set -eu

[ $# -eq 3 ] || { sed -n '2,4p' "$0" >&2; exit 2; }
arch=$1
binary=$2
url=$3

root=$(cd "$(dirname "$0")/.." && pwd)
[ "$(basename "$binary")" = waid-shell ] || { echo 'pack-runtime: expected the standalone waid-shell binary' >&2; exit 1; }
python3 "$root/scripts/runtime_artifact.py" --arch "$arch" --binary "$binary"
mkdir -p "$root/out"
archive="$root/out/waid-shell-$arch.tar.gz"
tar -czf "$archive" -C "$(dirname "$binary")" "$(basename "$binary")"
sha=$(sha256sum "$archive" | awk '{print $1}')
echo "$arch $sha $url"
