#!/bin/sh
# Build the Orus OS live ISO with live-build, from the .debs that
# dpkg-buildpackage leaves one directory above the repo.
#
# live-build creates root-owned files, so it builds in ../orus-iso-build,
# outside the source tree (dh_clean cannot walk root-owned directories).
# Needs live-build and sudo. ARCH=amd64 builds the x86_64 ISO (default arm64).
# EDITION=server builds the headless edition: no detir-shell, waid-runtime, login or installer
# .debs, so no runtime and no apps (see iso/server/).
# Output: ../orus-iso-build/live-image-*.iso (../orus-iso-build-server for the server).
#
# ORUS_DEV_SSH_KEY=/path/to/key.pub builds a DEVELOPMENT desktop ISO: the SSH server runs
# on live media for that one key (passwords off), so a developer can log in to the live
# user and fix things. Every copy of such an ISO shares its SSH host keys and trusts that
# key: never publish one. Without the variable the live medium keeps SSH off.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
build="$root/../orus-iso-build"
# The server edition builds in its own directory so it leaves the desktop's cache alone.
[ "${EDITION:-desktop}" = server ] && build="$root/../orus-iso-build-server"

mkdir -p "$build"
rm -rf "$build/auto" "$build/config"
cp -R "$root/iso/auto" "$root/iso/config" "$build/"

mkdir -p "$build/config/packages.chroot"
if [ "${EDITION:-desktop}" = server ]; then
    # Keep os-release; drop the desktop-only package list, hook, login and boot menu theme.
    rm -rf "$build/config/package-lists" "$build/config/hooks" "$build/config/includes.chroot_after_packages/etc/skel" "$build/config/includes.binary"
    cp -R "$root/iso/server/config/." "$build/config/"
else
    cp "$root"/../detir-shell_*.deb "$root"/../waid-runtime_*.deb "$root"/../orus-installer_*.deb "$root"/../orus-login_*.deb "$root"/../labwc_*.deb \
        "$build/config/packages.chroot/"
fi

if [ -n "${ORUS_DEV_SSH_KEY:-}" ]; then
    [ "${EDITION:-desktop}" = server ] && { echo "ORUS_DEV_SSH_KEY is for the desktop edition" >&2; exit 1; }
    [ -f "$ORUS_DEV_SSH_KEY" ] || { echo "ORUS_DEV_SSH_KEY: no such file: $ORUS_DEV_SSH_KEY" >&2; exit 1; }
    grep -q '^ssh-' "$ORUS_DEV_SSH_KEY" || { echo "ORUS_DEV_SSH_KEY must be a public key (ssh-...)" >&2; exit 1; }
    dev="$build/config/includes.chroot_after_packages"
    mkdir -p "$dev/etc/skel/.ssh" "$dev/etc/ssh/sshd_config.d" "$dev/etc/systemd/system/ssh.service.d" "$build/config/hooks/normal"
    # The live user is made from /etc/skel, so it trusts this key.
    cp "$ORUS_DEV_SSH_KEY" "$dev/etc/skel/.ssh/authorized_keys"
    # Key only: the live password is public. sshd reads the first value it sees, so 10- wins.
    printf 'PasswordAuthentication no\nKbdInteractiveAuthentication no\nPermitRootLogin no\n' \
        > "$dev/etc/ssh/sshd_config.d/10-orus-dev.conf"
    # An empty ConditionPathExists= clears the "not on live media" condition set in
    # orus-not-live.conf (this file sorts after it).
    printf '[Unit]\nConditionPathExists=\n' > "$dev/etc/systemd/system/ssh.service.d/zz-orus-dev.conf"
    # sshd refuses a key file that others can write.
    printf '#!/bin/sh\nchmod 700 /etc/skel/.ssh\nchmod 600 /etc/skel/.ssh/authorized_keys\n' \
        > "$build/config/hooks/normal/0200-orus-dev-ssh.hook.chroot"
    chmod 755 "$build/config/hooks/normal/0200-orus-dev-ssh.hook.chroot"
    echo "build-iso: DEVELOPMENT ISO with SSH on live media for $(cut -d' ' -f3- "$ORUS_DEV_SSH_KEY")" >&2
fi

cd "$build"
# Clear the previous build (keeping the package cache) so live-build installs
# the packages it was just given instead of skipping finished stages.
sudo lb clean
lb config
sudo lb build
