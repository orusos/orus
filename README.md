# Orus OS distro

Everything needed to build Orus OS on Debian: pinned sources, build script,
and (as they are added) session config, packaging and the ISO build. Plan and
status: `BACKLOG.md`.

## Sources

The shell (launcher, panel, Files, Terminal, Text Edit, Viewer, `orus`) is in `shell/`;
it builds with `cargo` and needs only the public `jag` crates, which Cargo fetches from
`github.com/jag-rs/jag`. labwc comes from Debian's package, built by `scripts/build-labwc.sh`.

The web-app runtime (`detir-scene`, packaged as `waid-runtime`) is a prebuilt binary under
the Business Source License 1.1. `scripts/fetch-runtime.sh` downloads the release that `runtime.lock` pins,
checked by SHA-256. If you have the runtime's source, set `ORUS_DETIR_DIR` to that
checkout and `scripts/build.sh` builds it instead.

## Build

```
git clone https://github.com/orusos/orus.git
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
`out/bin/detir-scene`.

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
launcher. The shell starts each one in its own `detir-scene` window.

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

The shell has no built-in browser; Chromium is the browser.

## Publish the runtime

The prebuilt runtime is hosted at
`https://packages.kilne.dev/runtime/<version>/detir-scene-<arch>.tar.gz`. `runtime.lock`
pins each file by SHA-256, and a published version is never changed: a new build is a new
version. Uploading is done by the maintainers with a script that is not in this repo.

```
# Pack a detir-scene build; this prints the runtime.lock line:
scripts/pack-runtime.sh arm64 /path/to/detir-scene \
    https://packages.kilne.dev/runtime/0.1.0-<detir commit>/detir-scene-arm64.tar.gz
```

## License

`waid-runtime` (`detir-scene`) is under the Business Source License 1.1
(`licenses/LICENSE-waid-runtime-BSL-1.1.txt`): production use is allowed, and each
version becomes Apache-2.0 four years after its first public release. Everything
else here is open source: the distribution files are Apache-2.0, and the
shell is Apache-2.0 or MIT. See `debian/copyright`. The crates linked
into each binary are listed in `licenses/`; regenerate them with
`scripts/license-notices.sh` after bumping a pin.
