#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
CRATE_DIR="$ROOT_DIR/crates/orus-macos"
TARGET_DIR="${TARGET_DIR:-$ROOT_DIR/target/release}"
APP_DIR="$TARGET_DIR/Orus OS Universal.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"

cargo build --release -p orus-macos

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
cp "$CRATE_DIR/Info.plist" "$CONTENTS_DIR/Info.plist"
cp "$TARGET_DIR/orus-macos" "$MACOS_DIR/orus-macos"

echo "Packaged app bundle at: $APP_DIR"
echo "Codesign with sandbox entitlements using:"
echo "codesign --force --deep --sign - --entitlements \"$CRATE_DIR/Orus.entitlements\" \"$APP_DIR\""
