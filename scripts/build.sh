#!/bin/sh
# Build the Orus OS binaries into out/bin: the shell, from shell/, and the runtime
# plus matched shared Cerlus/WAID hosts by default. ORUS_DETIR_DIR supplies
# source; ORUS_DEPLOYMENT_DIR can supply a verified release set. Standalone WAID
# builds remain available with ORUS_RUNTIME_LAYOUT=standalone.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/out/bin"

cargo build --locked --release --manifest-path "$root/shell/Cargo.toml" -p orus-linux -p orus -p orus-appearance -p orus-greeter

shell_target=$(cargo metadata --locked --no-deps --format-version 1 --manifest-path "$root/shell/Cargo.toml" |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
mkdir -p "$out"
for bin in orus-linux orus orus-appearance orus-greeter; do
    install -m 755 "$shell_target/release/$bin" "$out/"
done

layout=${ORUS_RUNTIME_LAYOUT:-shared}
case "$layout" in
    shared)
        deployment=${ORUS_DEPLOYMENT_DIR:-}
        if [ -z "$deployment" ]; then
            [ -n "${ORUS_DETIR_DIR:-}" ] || {
                echo 'Shared packaging needs ORUS_DETIR_DIR or a verified ORUS_DEPLOYMENT_DIR.' >&2
                exit 1
            }
            deployment=$(mktemp -d "$root/out/deployment.XXXXXX")
            rmdir "$deployment"
            python3 "$ORUS_DETIR_DIR/scripts/build-runtime-deployment.py" --mode shared --out "$deployment"
        fi
        python3 "$root/scripts/stage_runtime_deployment.py" --source "$deployment" --destination "$root/out"
        ;;
    standalone)
        if [ -n "${ORUS_DETIR_DIR:-}" ]; then
            cargo build --locked --release --manifest-path "$ORUS_DETIR_DIR/Cargo.toml" -p waid-shell --bin waid-shell
            runtime_target=$(cargo metadata --locked --no-deps --format-version 1 --manifest-path "$ORUS_DETIR_DIR/Cargo.toml" |
                python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
            "$root/scripts/check-runtime.sh" "$runtime_target/release/waid-shell"
            install -m 755 "$runtime_target/release/waid-shell" "$out/"
        else
            "$root/scripts/fetch-runtime.sh"
        fi
        ;;
    *) echo "Unknown runtime layout: $layout" >&2; exit 1 ;;
esac
"$root/scripts/check-runtime.sh"
