# WAID runtime package

The Debian package keeps its `waid-runtime` name and installs the standalone
`/usr/bin/waid-shell` executable. Each web app has its own process. Apps reuse
one installed executable; shared-runtime dynamic-library packaging remains
SS-T17 in the cross-repository shell-split backlog.

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
ORUS_DETIR_DIR=/path/to/current/detir scripts/build.sh
scripts/check-runtime.sh
python3 -m unittest discover -s scripts -p 'test_runtime*.py'
dpkg-buildpackage -b -us -uc
```

The build queries Cargo's target directory rather than guessing it. The package
rechecks the staged binary during `dh_install`, including the ELF architecture,
its executable bit, actual CLI identity and required WAID flags. A host Mach-O,
wrong architecture, stale browser binary or script cannot stand in for WAID.
Archive extraction requires exactly one regular `waid-shell` member and replaces
previous staged output only after validation. The old public runtime.lock pins
are retained, with an explicit refusal; they need a newly published standalone
release before the default download route can be used again. Source builds are
the current local validation route. License terms are unchanged.

## Booted-image acceptance

After building the desktop image, boot it and run this from its normal labwc
user session (copy the two Python helper files there if the source is absent):

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

A passing check covers this standalone package/image launch. It does not qualify
full browser parity, physical GPU performance, all Linux hardware, shared-library
packaging or release publication. Retain the built ISO and Debian artifact hashes
alongside the live-image receipt to identify the booted build.
