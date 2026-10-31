# Orus OS distro

Everything needed to build Orus OS on Debian: pinned sources, build script,
and (as they are added) session config, packaging and the ISO build. Plan and
status: `BACKLOG.md`.

## Sources

The shell (launcher, panel, Files, Terminal, Text Edit, Viewer, `orus`) is in `shell/`;
it builds with `cargo` and needs only the public `jag` crates, which Cargo fetches from
`github.com/splentainc/jag`. labwc comes from Debian's package, built by `scripts/build-labwc.sh`.

The web-app runtime (`waid-shell`, packaged as `waid-runtime`) is a prebuilt binary under
the Business Source License 1.1. `scripts/fetch-runtime.sh` downloads the release that `runtime.lock` pins,
checked by SHA-256. If you have the runtime's source, set `ORUS_DETIR_DIR` to that
checkout and `scripts/build.sh` builds it instead.

The current lock still records the older combined runtime release. It is retained
as provenance and is refused by the fetcher: renaming that binary does not give
it the standalone WAID CLI. Build from the current Detir source until canonical
`waid-shell` releases are published and pinned. Both fetch and package staging
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
cd orus && scripts/build.sh
```

Needs a Rust toolchain (rustup, stable) and these Debian 13 packages, the
set the build VM has installed (a superset; GTK/WebKit are not needed):

```
build-essential pkg-config git curl cmake clang python3
libwayland-dev libxkbcommon-dev libegl1 libgl1-mesa-dri libgbm-dev libdrm-dev
libudev-dev libinput-dev libasound2-dev libssl-dev libfontconfig-dev
```

Runtime on the target: `labwc`, `mesa-vulkan-drivers`, and a font package
(`fonts-noto-core` or `fonts-dejavu-core`). Output: `out/bin/orus-linux`,
`out/bin/waid-shell`.

## Build machine

On a fresh Debian 13 machine, `scripts/setup-build-vm.sh` installs everything the
build needs (packages, Node, Rust). Then:

```
scripts/build.sh && scripts/build-apps.sh      # binaries and apps
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

The shell has no built-in browser; Chromium is the browser. App launch uses
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

`waid-runtime` (`waid-shell`) is under the Business Source License 1.1
(`licenses/LICENSE-waid-runtime-BSL-1.1.txt`): production use is allowed, and each
version becomes Apache-2.0 four years after its first public release. Everything
else here is open source: the distribution files are Apache-2.0, and the
shell is Apache-2.0 or MIT. See `debian/copyright`. The crates linked
into each binary are listed in `licenses/`; regenerate them with
`scripts/license-notices.sh` after bumping a pin.
