#!/bin/sh
# Builds, signs, notarizes and packages Refr for a GitHub release:
#
#   target/bundle/Refr-<version>-arm64.dmg
#   target/bundle/Refr-<version>-arm64.zip
#
#   NOTARY_PROFILE=<name> scripts/release.sh
#
# NOTARY_PROFILE is a notarytool keychain profile, created once with
#   xcrun notarytool store-credentials <name> --apple-id <id> --team-id <team>
# Without it the app and disk image are signed but not notarized, and Gatekeeper
# warns on other Macs. SIGN_IDENTITY defaults to the first "Developer ID
# Application" identity in the keychain.
set -eu
cd "$(dirname "$0")/.."

IDENTITY=${SIGN_IDENTITY:-$(security find-identity -v -p codesigning |
    sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)}
if [ -z "$IDENTITY" ]; then
    echo "No \"Developer ID Application\" identity in the keychain." >&2
    exit 1
fi
echo "Signing as: $IDENTITY"
SIGN_IDENTITY="$IDENTITY" scripts/bundle.sh

APP=target/bundle/Refr.app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DMG=target/bundle/Refr-$VERSION-arm64.dmg
ZIP=target/bundle/Refr-$VERSION-arm64.zip
codesign --verify --strict --verbose=2 "$APP"

notarize() {
    xcrun notarytool submit "$1" --keychain-profile "$NOTARY_PROFILE" --wait
    xcrun stapler staple "$2"
}

# Notarize the app itself, so its ticket travels with it out of the disk image.
if [ -n "${NOTARY_PROFILE:-}" ]; then
    SUBMISSION=target/bundle/notarize.zip
    ditto -c -k --keepParent "$APP" "$SUBMISSION"
    notarize "$SUBMISSION" "$APP"
    rm -f "$SUBMISSION"
fi

rm -f "$ZIP"
ditto -c -k --keepParent --norsrc "$APP" "$ZIP"

STAGE=target/bundle/dmg
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Refr $VERSION" -srcfolder "$STAGE" -ov -format ULFO "$DMG" >/dev/null
rm -rf "$STAGE"
codesign --force --timestamp --sign "$IDENTITY" "$DMG"
if [ -n "${NOTARY_PROFILE:-}" ]; then
    notarize "$DMG" "$DMG"
    spctl --assess --type open --context context:primary-signature --verbose=2 "$DMG"
fi
spctl --assess --type execute --verbose=2 "$APP" || true
shasum -a 256 "$DMG" "$ZIP"
