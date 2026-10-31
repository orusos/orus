#!/bin/sh
# Build every web app under apps/ and stage it in out/apps/<name>. An app with a
# package.json is built with npm; one that already holds a built bundle (a
# waid-package.json and an index.html) is copied as it is. Name apps as arguments to
# stage only those. Install the staged apps with scripts/install-apps.sh.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/out/apps"

mkdir -p "$out"
for app in "$root"/apps/*/; do
    name=$(basename "$app")
    if [ $# -gt 0 ]; then
        case " $* " in *" $name "*) ;; *) continue ;; esac
    fi
    rm -rf "$out/$name"
    if [ -f "$app/package.json" ]; then
        (cd "$app" && npm ci --no-audit --no-fund && npm run build)
        cp -R "$app/dist" "$out/$name"
    elif [ -f "$app/waid-package.json" ] && [ -f "$app/index.html" ]; then
        cp -R "$app" "$out/$name"
    fi
done
