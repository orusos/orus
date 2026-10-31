#!/bin/sh
# Regenerate licenses/*-crates.txt: the Rust crates linked into each binary
# package, with the license each one declares. Installed in the package's doc
# directory. Run after bumping a pin.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$root/licenses"

crates() { # workspace-dir package output-name
    {
        for target in aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
            (cd "$1" && cargo tree --locked -p "$2" -e normal --target "$target" \
                --prefix none --format "{p}|{l}")
        done
    } | sed 's/ *(\*)//; s/ ([^)]*)//' \
      | awk -F'|' '{ lic = ($2 == "" ? "no license declared (Splenta Inc crate)" : $2); print $1 " - " lic }' \
      | sort -u > "$root/licenses/$3-crates.txt"
}

crates "${ORUS_DETIR_DIR:?set ORUS_DETIR_DIR to a Detir checkout}" waid-shell waid-runtime
crates "$root/shell" orus-linux detir-shell
