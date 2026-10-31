#!/bin/sh
# Build labwc from upstream source as a .deb, one directory above the repo, where
# dpkg-buildpackage leaves the other packages. Trixie ships labwc 0.8.3; the
# Alt+Tab thumbnail switcher needs 0.9 or newer, which needs wlroots 0.19, so
# wlroots is built into the labwc binary (as labwc's own build does).
# Needs the build packages from scripts/setup-build-vm.sh and network access.
set -eu

LABWC_VERSION=0.9.8
LABWC_SHA256=f97bdbcb4b2b907b0eda243a62d8e06db822e889de14126156505245ffb7e6cb
WLROOTS_VERSION=0.19.3
WLROOTS_SHA256=a6ff89b64ea15e424d1b0db4a22145fccf5ec2ff2e7b8af0fa35e2ac8975986f
DEB_VERSION="$LABWC_VERSION-1orus1"

root=$(cd "$(dirname "$0")/.." && pwd)
work="$root/../labwc-build"
arch=$(dpkg --print-architecture)

fetch() { # url sha256 file
    [ -f "$3" ] || curl -fsSL "$1" -o "$3"
    echo "$2  $3" | sha256sum -c - >/dev/null
}

rm -rf "$work/src" "$work/stage"
mkdir -p "$work/src" "$work/stage"
cd "$work"
fetch "https://github.com/labwc/labwc/archive/refs/tags/$LABWC_VERSION.tar.gz" "$LABWC_SHA256" "labwc-$LABWC_VERSION.tar.gz"
fetch "https://gitlab.freedesktop.org/wlroots/wlroots/-/archive/$WLROOTS_VERSION/wlroots-$WLROOTS_VERSION.tar.gz" "$WLROOTS_SHA256" "wlroots-$WLROOTS_VERSION.tar.gz"

tar -xf "labwc-$LABWC_VERSION.tar.gz" -C src
mv "src/labwc-$LABWC_VERSION" src/labwc
mkdir -p src/labwc/subprojects
rm -rf src/labwc/subprojects/wlroots
tar -xf "wlroots-$WLROOTS_VERSION.tar.gz" -C src/labwc/subprojects
mv "src/labwc/subprojects/wlroots-$WLROOTS_VERSION" src/labwc/subprojects/wlroots

# No XWayland (apps run on Wayland) and no window icons (Orus OS windows set no
# app id, so labwc could only show its placeholder icon). Without icons labwc turns
# the layout's icon button into a menu button; the themes draw it empty.
meson setup src/labwc/build src/labwc --prefix=/usr --buildtype=release \
    -Dxwayland=disabled -Dicon=disabled -Dtest=disabled
ninja -C src/labwc/build
DESTDIR="$work/stage" ninja -C src/labwc/build install

# Licenses and where the source came from (labwc is GPL-2.0, wlroots MIT).
doc="$work/stage/usr/share/doc/labwc"
mkdir -p "$doc"
cp src/labwc/LICENSE "$doc/copyright"
cp src/labwc/subprojects/wlroots/LICENSE "$doc/copyright.wlroots"
cat > "$doc/README.source" <<DOC
This labwc is upstream $LABWC_VERSION built with wlroots $WLROOTS_VERSION linked in.
Source:
  https://github.com/labwc/labwc/archive/refs/tags/$LABWC_VERSION.tar.gz
  sha256 $LABWC_SHA256
  https://gitlab.freedesktop.org/wlroots/wlroots/-/archive/$WLROOTS_VERSION/wlroots-$WLROOTS_VERSION.tar.gz
  sha256 $WLROOTS_SHA256
Built with: -Dxwayland=disabled -Dicon=disabled (see scripts/build-labwc.sh in the
Orus OS distro repository).
DOC

# Runtime dependencies from the binary's shared libraries.
mkdir -p tmp/debian
printf 'Source: labwc\n\nPackage: labwc\nArchitecture: any\n' > tmp/debian/control
depends=$(cd tmp && dpkg-shlibdeps -O -e "$work/stage/usr/bin/labwc" 2>/dev/null | sed 's/^shlibs:Depends=//')

mkdir -p stage/DEBIAN
cat > stage/DEBIAN/control <<CTRL
Package: labwc
Version: $DEB_VERSION
Architecture: $arch
Maintainer: Orus OS <partha.yp@gmail.com>
Section: x11
Priority: optional
Depends: $depends, hwdata
Provides: wayland-compositor
Conflicts: labwc (<< $DEB_VERSION)
Replaces: labwc (<< $DEB_VERSION)
Description: labwc $LABWC_VERSION window-stacking Wayland compositor
 Upstream labwc built with wlroots $WLROOTS_VERSION, for the thumbnail window
 switcher that the version in Debian trixie lacks.
CTRL
dpkg-deb --root-owner-group --build stage "$root/../labwc_${DEB_VERSION}_$arch.deb"
echo "built $root/../labwc_${DEB_VERSION}_$arch.deb"
