#!/bin/sh
# Check the exact executable that waid-runtime will install on this Debian host.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
arch=$(dpkg --print-architecture)
exec python3 "$root/scripts/runtime_artifact.py" --arch "$arch" --check-cli \
    --binary "${1:-$root/out/bin/waid-shell}"
