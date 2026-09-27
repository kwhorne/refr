# Building from source

## Requirements

- macOS 13 or later on Apple silicon
- Rust 1.88 or later (the workspace uses edition 2024), installed with
  [rustup](https://rustup.rs)
- Xcode Command Line Tools (`xcode-select --install`). Full Xcode isn't required, because
  GPUI compiles its Metal shaders at runtime.
- About 2 GB of free disk space for `target/`

## Get the code and PDFium

```sh
git clone https://github.com/kwhorne/refr.git
cd refr
scripts/fetch-pdfium.sh
```

`fetch-pdfium.sh` downloads a prebuilt PDFium from
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) into
`vendor/pdfium/`, which git ignores. It picks `mac-arm64` or `mac-x64` from `uname -m`. To use
another PDFium release, set `PDFIUM_VERSION`:

```sh
PDFIUM_VERSION=chromium/8066 scripts/fetch-pdfium.sh
```

## Run

```sh
cargo run -p refr                    # opens the sample report
cargo run -p refr -- report.pdf      # opens files given as arguments
cargo run -p refr --release          # optimized build
```

The first build compiles GPUI and its dependencies, which takes a few minutes. Later builds
only compile Refr's crates.

## Test

```sh
cargo test                  # everything
cargo test -p refr-core     # the model, without PDFium
cargo test -p refr -- text  # app tests whose names contain "text"
```

The engine and app tests need PDFium. See [Testing](testing.md).

## Package the app

```sh
scripts/bundle.sh
open target/bundle/Refr.app
```

This builds a release binary and assembles `target/bundle/Refr.app` with PDFium in
`Contents/Frameworks`, the app icon and the license files. It signs the app ad hoc unless you
set `SIGN_IDENTITY`. For signed, notarized releases, see [Releasing](releasing.md).

## Where Refr looks for PDFium

`Engine::locate_library` takes the first of these that exists:

1. The path in `REFR_PDFIUM`
2. `libpdfium.dylib` next to the executable
3. `../Frameworks/libpdfium.dylib` relative to the executable (inside an app bundle)
4. `lib/libpdfium.dylib` next to the executable
5. `vendor/pdfium/lib/libpdfium.dylib` in the source tree

If none exists, Refr shows **Refr could not start its PDF engine** and quits.

## Environment variables

| Variable | Used by | Effect |
|---|---|---|
| `REFR_PDFIUM` | App, tests | Path to `libpdfium.dylib` |
| `REFR_DATA_DIR` | App | Replaces `~/Library/Application Support/Refr` for the recovery copy and recent files. Handy for trying Refr without touching your own data. |
| `PDFIUM_VERSION` | `fetch-pdfium.sh` | The PDFium release tag to download |
| `SIGN_IDENTITY` | `bundle.sh`, `release.sh` | The code-signing identity |
| `NOTARY_PROFILE` | `release.sh` | The notarytool keychain profile |

## Build profile

The workspace `Cargo.toml` keeps `target/` small:

```toml
[profile.dev]
debug = "line-tables-only"
incremental = false

[profile.dev.package."*"]
debug = false
```

Dependencies are built without debug info, and Refr's own crates keep line tables for
backtraces. A full debug build plus tests uses about 1.6 GB instead of more than 5 GB. If you
need to step through code in a debugger, set `debug = true` for the crate you're working
on, for example `[profile.dev.package.refr-core]`.

## Useful examples

```sh
# Render sample pages and an export to PNGs, to check rendering by eye
cargo run -p refr-pdf --example dump -- /tmp/refr-dump

# Write the annotated workspace used for the README screenshot
cargo run -p refr-pdf --example showcase -- /tmp/refr-showcase
```

## Project layout

```
crates/
  refr-core/    model, .pdfspace JSON, geometry, layout, editing (no UI, no PDFium)
  refr-pdf/     PDFium engine: import, text, render, export, sample document
  refr/         the GPUI app
assets/icons/   SVG icons compiled into the binary
resources/      AppIcon.icns and AppIcon.png
scripts/        fetch-pdfium.sh, bundle.sh, release.sh, make_icon.swift
docs/           this documentation
vendor/pdfium/  downloaded PDFium (git-ignored)
```

See [Architecture](architecture.md) for how the pieces fit together.
