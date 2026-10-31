#!/bin/sh
# Install the apps staged by scripts/build-apps.sh into ~/.orus/apps with
# orus (built by scripts/build.sh).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)

for app in "$root"/out/apps/*/; do
    [ -d "$app" ] || continue
    "$root/out/bin/orus" install --yes "$app"
done
