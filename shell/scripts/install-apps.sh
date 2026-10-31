#!/usr/bin/env bash
# Install Orus OS apps from the repo into ~/.orus/apps/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APPS_SRC="$REPO_ROOT/apps"
# Install to ~/.orus/apps (primary) if it exists, else ~/.orus/apps (legacy).
if [ -d "$HOME/.orus" ]; then
    APPS_DST="$HOME/.orus/apps"
else
    APPS_DST="$HOME/.orus/apps"
fi

mkdir -p "$APPS_DST"

for app in "$APPS_SRC"/*/; do
    name="$(basename "$app")"
    echo "installing $name -> $APPS_DST/$name"
    rm -rf "$APPS_DST/$name"
    cp -r "$app" "$APPS_DST/$name"
done

echo "done — $(ls -1 "$APPS_SRC" | wc -l | tr -d ' ') app(s) installed"
