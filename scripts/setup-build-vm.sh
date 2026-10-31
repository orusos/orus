#!/bin/sh
# Prepare a fresh Debian 13 machine to build Orus OS: the cargo build, the
# web apps, the .deb packages and the live ISO. Run as a normal user with sudo.
# Safe to run again.
set -eu

# Build the shell and the runtime: toolchain, graphics and input libraries.
build="build-essential pkg-config git curl cmake clang python3
 libwayland-dev libxkbcommon-dev libegl1 libgl1-mesa-dri libgbm-dev libdrm-dev
 libudev-dev libinput-dev libasound2-dev libssl-dev libfontconfig-dev"

# Web apps (Vite 8 needs Node 20.19+; Debian 13 ships 20.19).
web="nodejs npm"

# Packages and ISO.
packaging="patchelf binutils debhelper dpkg-dev fakeroot lintian live-build debootstrap squashfs-tools xorriso"

# Copying sources in and out, and running the session for a headless check.
# labwc is built from source (scripts/build-labwc.sh): its build packages.
labwc_build="meson ninja-build libwayland-dev wayland-protocols libxkbcommon-dev
 libpixman-1-dev libinput-dev libseat-dev libdrm-dev libliftoff-dev libdisplay-info-dev
 libvulkan-dev glslang-tools libgbm-dev libegl-dev libgles-dev libxml2-dev libglib2.0-dev
 libcairo2-dev libpango1.0-dev librsvg2-dev libpng-dev libudev-dev hwdata scdoc
 libvulkan-volk-dev libxcb1-dev libxcb-composite0-dev libxcb-ewmh-dev libxcb-icccm4-dev
 libxcb-render-util0-dev libxcb-res0-dev libxcb-xinput-dev libxcb-errors-dev libxcb-dri3-dev
 libxcb-present-dev libxcb-shm0-dev libxcb-xfixes0-dev libxcb-render0-dev libxcb-dri2-0-dev
 libx11-xcb-dev libxcb-image0-dev"

# Headers for the runtime's GStreamer media backend (detir linux-media), and the tools
# to make test clips with.
media="libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-tools
 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav"

tools="rsync openssh-server sudo labwc grim wlrctl"

sudo apt-get update
# shellcheck disable=SC2086
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y $build $web $packaging $tools $labwc_build $media

if ! command -v cargo >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi

echo
echo "Ready. In a new shell (or after '. ~/.cargo/env'):"
echo "  export ORUS_DETIR_DIR=/path/to/current/detir"
echo "  scripts/build.sh && scripts/build-apps.sh && dpkg-buildpackage -b -us -uc && scripts/build-iso.sh"
