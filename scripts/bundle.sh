#!/bin/sh
# Builds target/bundle/Refr.app with libpdfium.dylib in Contents/Frameworks.
#
# Signs ad hoc by default. With SIGN_IDENTITY set to a "Developer ID Application"
# identity it signs for distribution, with the hardened runtime and a timestamp.
# PDFium is signed with the same identity, so library validation accepts it.
set -eu
cd "$(dirname "$0")/.."
[ -f vendor/pdfium/lib/libpdfium.dylib ] || scripts/fetch-pdfium.sh
[ -f resources/AppIcon.icns ] || swift scripts/make_icon.swift
cargo build --release -p refr

APP=target/bundle/Refr.app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Frameworks" "$APP/Contents/Resources"
cp target/release/refr "$APP/Contents/MacOS/Refr"
cp vendor/pdfium/lib/libpdfium.dylib "$APP/Contents/Frameworks/"
cp resources/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
mkdir -p "$APP/Contents/Resources/licenses"
cp LICENSE "$APP/Contents/Resources/licenses/Refr-LICENSE.txt"
cp vendor/pdfium/LICENSE "$APP/Contents/Resources/licenses/PDFium-LICENSE.txt"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Refr</string>
  <key>CFBundleDisplayName</key><string>Refr</string>
  <key>CFBundleIdentifier</key><string>no.gets.refr</string>
  <key>CFBundleExecutable</key><string>Refr</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT License</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>PDF document</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key><array><string>com.adobe.pdf</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>Refr workspace</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Owner</string>
      <key>CFBundleTypeExtensions</key><array><string>pdfspace</string></array>
    </dict>
  </array>
</dict>
</plist>
PLIST

IDENTITY=${SIGN_IDENTITY:--}
# Nested code first, then the app that seals it.
for code in "$APP/Contents/Frameworks/libpdfium.dylib" "$APP"; do
    if [ "$IDENTITY" = "-" ]; then
        codesign --force --sign - "$code"
    else
        codesign --force --options runtime --timestamp --sign "$IDENTITY" "$code"
    fi
done
echo "Built $APP ($VERSION)"
