# Orus OS distro

Everything needed to build Orus OS on Debian: pinned sources, build script,
and (as they are added) session config, packaging and the ISO build. Plan and
status: `BACKLOG.md`.

## Sources

The shell (launcher, panel, Files, Terminal, Text Edit, Viewer, `orus`) is in `shell/`;
it builds with `cargo` and needs only the public `jag` crates, which Cargo fetches from
`github.com/splentainc/jag`. labwc comes from Debian's package, built by `scripts/build-labwc.sh`.

The desktop packages default to a shared runtime: `cerlus` and `waid-shell`
remain separate product executables/processes and dynamically load one matched,
versioned Detir library. Build the matched set from `ORUS_DETIR_DIR`, or import a
verified release directory with `ORUS_DEPLOYMENT_DIR`. The runtime retains its
existing Business Source License 1.1 terms. Standalone source builds remain
available with `ORUS_RUNTIME_LAYOUT=standalone`.

The current lock still records the older combined runtime release. It is retained
as provenance and is refused by the fetcher: renaming that binary does not give
it the standalone WAID CLI. Build from the current Detir source until canonical
standalone/shared releases are published and pinned. Both fetch and package staging
validate Linux ELF architecture and the actual `waid-shell --help` contract.

```sh
ORUS_DETIR_DIR=/path/to/current/detir scripts/build.sh
```

The build respects Cargo's resolved target directory, including a shared
`CARGO_TARGET_DIR`. Source builds are locked; the Debian package checks the
staged executable again before installation into `/usr/bin/waid-shell`.

## Build

```
git clone https://github.com/splentainc/orus.git
cd orus
ORUS_DETIR_DIR=/path/to/current/detir scripts/build.sh
```

Needs a Rust toolchain (rustup, stable) and these Debian 13 packages, the
set the build VM has installed (a superset; GTK/WebKit are not needed):

```
build-essential pkg-config git curl cmake clang python3 patchelf binutils
libwayland-dev libxkbcommon-dev libegl1 libgl1-mesa-dri libgbm-dev libdrm-dev
libudev-dev libinput-dev libasound2-dev libssl-dev libfontconfig-dev
libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

Runtime on the target: `labwc`, `mesa-vulkan-drivers`, and a font package
(`fonts-noto-core` or `fonts-dejavu-core`). Output: `out/bin/orus-linux`,
`out/bin/waid-shell`, `out/bin/cerlus`, and `out/lib/detir/<abi>/`.

## Build machine

On a fresh Debian 13 machine, `scripts/setup-build-vm.sh` installs everything the
build needs (packages, Node, Rust). Then:

```
export ORUS_DETIR_DIR=/path/to/current/detir
scripts/build.sh
scripts/build-apps.sh                          # apps
dpkg-buildpackage -b -us -uc                   # .debs, one level up
scripts/build-iso.sh                           # live ISO, ../orus-iso-build
```

## Apps

Orus OS apps are built web apps (a `dist` with `index.html` and a
`waid-package.json` that has `appUniversalName` and `nativeUrlScheme`). They
are installed under `~/.orus/apps/<appUniversalName>/` and appear in the
launcher. The shell starts each one in its own `waid-shell` window.

```
scripts/build-apps.sh      # npm build of every apps/*, staged in out/apps/
scripts/install-apps.sh    # install the staged apps with orus

out/bin/orus install <folder | file.orusapp | https://URL | name[@version]>
out/bin/orus list
out/bin/orus remove <name>
out/bin/orus sources list|add|remove|enable|disable
out/bin/orus update | search [text] | upgrade | rollback <name>
out/bin/orus catalog build|serve|keygen   # make and serve a catalog (development source)
```

The shell has no embedded browser. Chromium remains its existing browser shortcut;
Cerlus is separately installed as `cerlus`. App launch uses
`waid-shell /path/to/app` directly (a directory or its `index.html`); there is no
`waid` subcommand and no browser-only initial-URL fallback.

For package checks and the booted-image/labwc acceptance command, see
[WAID runtime packaging](docs/waid-runtime.md).

## Publish the runtime

The prebuilt runtime is hosted at
`https://packages.kilne.dev/runtime/<version>/waid-shell-<arch>.tar.gz`. `runtime.lock`
pins each file by SHA-256, and a published version is never changed: a new build is a new
version. Uploading is done by the maintainers with a script that is not in this repo.

```
# Pack a Linux waid-shell build; this prints the runtime.lock line:
scripts/pack-runtime.sh arm64 /path/to/waid-shell \
    https://packages.kilne.dev/runtime/0.1.0-<detir commit>/waid-shell-arm64.tar.gz
```

## License

`waid-runtime`, the common `detir-runtime-shared` library and `cerlus` are under
the Business Source License 1.1
(`licenses/LICENSE-waid-runtime-BSL-1.1.txt`). See that file for the Additional Use
Grant and restrictions. Each version becomes Apache-2.0 four years after its first
public release. Everything
else here is open source: the distribution files are Apache-2.0, and the
shell is Apache-2.0 or MIT. See `debian/copyright`. The crates linked
into each binary are listed in `licenses/`; regenerate them with
`scripts/license-notices.sh` after bumping a pin.
