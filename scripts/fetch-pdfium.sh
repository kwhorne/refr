#!/bin/sh
# Downloads the prebuilt PDFium library (bblanchon/pdfium-binaries) into vendor/pdfium.
# PDFium is BSD-3-Clause / Apache-2.0; its license files are kept in vendor/pdfium.
set -eu
VERSION="${PDFIUM_VERSION:-chromium/8066}"
case "$(uname -m)" in
  arm64) ARCH=mac-arm64 ;;
  x86_64) ARCH=mac-x64 ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/vendor/pdfium"
mkdir -p "$DEST"
curl -fsSL "https://github.com/bblanchon/pdfium-binaries/releases/download/$VERSION/pdfium-$ARCH.tgz" | tar xz -C "$DEST"
echo "PDFium $(cat "$DEST/VERSION" 2>/dev/null | tr '\n' ' ')installed in $DEST/lib"
