# WAID runtime package

The Debian `waid-runtime` package installs `/usr/bin/waid-shell`. Its default
layout is now a thin WAID executable over `detir-runtime-shared`, also used by
`/usr/bin/cerlus` from the separate `cerlus` package. Each product/app retains its
own process, state and shell implementation. Standalone source/release builds
remain independently usable.

```sh
waid-shell /path/to/built/app
waid-shell /path/to/index.html
orus open installed-app-id
```

An app directory carries `waid-package.json`. Both the desktop launcher and
`orus open` pass the app path directly. They do not use a `waid` subcommand or
try to open a bare page through a browser's initial-URL environment variable.

## Build and validate

```sh
# Choose source, or a release built by build-runtime-deployment.py.
export ORUS_DETIR_DIR=/path/to/current/detir
# For an existing release, set ORUS_DEPLOYMENT_DIR=/path/to/release as well.
# Export it so the Debian rebuild uses the same selection.
# export ORUS_DEPLOYMENT_DIR=/path/to/release
scripts/build.sh
scripts/check-runtime.sh
python3 -m unittest discover -s scripts -p 'test_runtime*.py'
dpkg-buildpackage -b -us -uc

# Standalone WAID is still available outside shared Debian packaging.
ORUS_RUNTIME_LAYOUT=standalone ORUS_DETIR_DIR=/path/to/current/detir scripts/build.sh
```

The release builder checks independent product graphs, compiles the shared set
together, and records an exact source/compiler/target/capability ABI. The
private Rust ABI is not a stable C/plugin interface. Only its matching launchers
may use a library. Both product packages depend on the exact runtime Debian
version; upgrade or roll back the matched set together. The private runtime and
matched Rust standard library live under `/usr/lib/detir/<abi>/`, reached through
relative loader paths. No loader environment override or global runtime symlink
is needed. Missing/foreign versions fail closed. Rust standard-library component
notices are retained under `/usr/share/doc/detir-runtime-shared/rust/`.

Staging validates the passing release receipt, every recorded file hash,
architecture, actual CLI identity, version/ABI and native loader dependencies
before importing the products. Debian preserves those hashes when packaging.
The existing standalone archive fetcher still refuses the legacy runtime.lock
pins; publication of qualified standalone/shared releases remains DIST-RT1.

See the upstream [ABI, artifact and CI contract](../../DETIR_WAID/detir/docs/shared-runtime-deployment.md).

## Booted-image acceptance

After building the desktop image, boot it and log in normally to unlock the
keyring. Install observation tools in the live overlay with
`sudo apt-get install --no-install-recommends wlrctl grim` if they are absent.
Run this from the normal labwc user session (copy the two Python helper files
there if the source is absent):

```sh
python3 scripts/check_runtime_image.py --out /tmp/ss-t14-proof
```

This requires the live boot marker and Orus OS branding. It checks Debian package
ownership and runtime/CLI identity, opens a real WAID fixture, performs native
pointer input, waits for its JS click result, and retains the running executable,
labwc window list, runtime screenshot, desktop screenshot and package/hash receipt.
Use a fresh output directory. Missing services, timeouts, failed input, missing
captures and a runtime exit before the script finishes fail the check. Development
secret-store overrides are refused; use the normal logged-in session/keyring.

A passing SS-T14 check covers its captured standalone package/image launch. It does not qualify
full browser parity, physical GPU performance, all Linux hardware, shared-library
packaging or release publication. Retain the built ISO and Debian artifact hashes
alongside the live-image receipt to identify the booted build.

## SS-T14 standalone evidence (2026-10-31)

The ARM64 Debian package and rebuilt live ISO passed this check after normal
login; both runtime and labwc desktop screenshots show “Clicks: 1”. The running
executable matches the packaged and squashfs binary. See the cross-repository
[SS-T14 evidence](../../DETIR_WAID/docs/findings/ss-t14-distro-runtime-and-labwc.md).
The fixture uses `addEventListener`; earlier `onclick` property attempts failed.
The passing launch also logs a background `zbus` reactor panic. These runtime
issues remain DIST-RT2/DIST-RT3; this package acceptance is not release qualification.

## SS-T17 shared installation acceptance

From a normal logged-in labwc/keyring session, run the upstream
`tools/runtime_deployment/qualify.py --release /path/to/release --out /tmp/ss-t17-proof`.
The release must include both layouts, and the shared packages must be installed.
Install `grim`, `wlrctl`, `patchelf` and `binutils` for observation if absent. The checker
records two real product processes in each layout, installed package ownership,
actual executables, the common library inode, native input, screenshots and IR.
Both product IR exports and the WAID frame must match across layouts. Visually
inspect the images and retain receipts. This is deployment acceptance; existing
DIST-RT2/DIST-RT3 runtime gaps and broader release/platform qualification remain.

The ARM64 installed matrix passed for both matched development package sets
`0.1.0+sst17.3` and `0.1.0+sst17.4` from the final source, including upgrade
and rollback. Earlier `.1`/`.2` package tests are retained as historical evidence. Both products
map the same installed runtime inode; document exports and WAID frame bytes match
standalone mode. A mixed-version apt simulation is refused. Updating the complete
source package set restarts `orus-login`; sign in normally again before checking
apps. See [SS-T17 evidence](../../DETIR_WAID/docs/findings/ss-t17-shared-runtime-deployment.md).
