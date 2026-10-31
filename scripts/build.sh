#!/bin/sh
# Build the Orus OS binaries into out/bin: the shell, from shell/, and the runtime
# (waid-shell). The runtime comes from a prebuilt release (scripts/fetch-runtime.sh,
# the default) or, for people who have its source, is built from the checkout that
# ORUS_DETIR_DIR names.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/out/bin"

cargo build --release --manifest-path "$root/shell/Cargo.toml" -p orus-linux -p orus -p orus-appearance -p orus-greeter

mkdir -p "$out"
for bin in orus-linux orus orus-appearance orus-greeter; do
    install -m 755 "$root/shell/target/release/$bin" "$out/"
done

if [ -n "${ORUS_DETIR_DIR:-}" ]; then
    cargo build --release --manifest-path "$ORUS_DETIR_DIR/Cargo.toml" -p waid-shell --bin waid-shell
    runtime_target="${CARGO_TARGET_DIR:-$ORUS_DETIR_DIR/target}"
    install -m 755 "$runtime_target/release/waid-shell" "$out/"
else
    "$root/scripts/fetch-runtime.sh"
fi
