# Releasing

A release is a signed, notarized `Refr-<version>-arm64.dmg` and `.zip` attached to a GitHub
release.

## One-time setup

1. **Developer ID certificate.** Install a "Developer ID Application" certificate in your
   login keychain. Check with:

   ```sh
   security find-identity -v -p codesigning
   ```

2. **Notarization credentials.** Store an app-specific password as a notarytool profile:

   ```sh
   xcrun notarytool store-credentials <profile> --apple-id <apple-id> --team-id <team-id>
   ```

3. **GitHub CLI.** `gh auth login` with access to the repository.

## Steps

1. **Bump the version** in the workspace `Cargo.toml` (`[workspace.package] version`). The
   About window and `Info.plist` read it from there.

2. **Run the tests** and make sure the build has no warnings:

   ```sh
   cargo test
   ```

3. **Build, sign and notarize:**

   ```sh
   NOTARY_PROFILE=<profile> scripts/release.sh
   ```

   This:
   - builds `target/bundle/Refr.app` with `scripts/bundle.sh`, signed with the first Developer
     ID identity in the keychain (or `SIGN_IDENTITY`), with the hardened runtime and a secure
     timestamp. `libpdfium.dylib` is signed first, then the app that seals it.
   - notarizes the app and staples the ticket, so it works offline once it's copied out of the
     disk image
   - writes `Refr-<version>-arm64.zip`
   - builds `Refr-<version>-arm64.dmg` with an Applications shortcut, then signs, notarizes
     and staples it
   - checks both with `spctl` and prints their SHA-256 checksums

   Notarization usually takes a few minutes. Without `NOTARY_PROFILE`, everything is signed
   but not notarized, and Gatekeeper warns on other Macs.

4. **Smoke test** the notarized app: open it, open a PDF, add an annotation and export.

5. **Commit, tag and push:**

   ```sh
   git commit -am "Refr <version>"
   git tag -a v<version> -m "Refr <version>"
   git push && git push origin v<version>
   ```

6. **Publish the release:**

   ```sh
   gh release create v<version> \
     target/bundle/Refr-<version>-arm64.dmg \
     target/bundle/Refr-<version>-arm64.zip \
     --title "Refr <version>" --notes-file notes.md
   ```

   Include what changed, install instructions, known limitations and the SHA-256 checksums
   from step 3.

## The app icon

`resources/AppIcon.icns` is drawn by `scripts/make_icon.swift` with CoreGraphics. To change
it, edit the script and run:

```sh
swift scripts/make_icon.swift
```

It writes every size from 16 to 1024 pixels into `AppIcon.icns`, plus `AppIcon.png` at
1024 pixels. Commit both.

## What's in the bundle

```
Refr.app/Contents/
  Info.plist                 bundle ID no.gets.refr, version, document types
  MacOS/Refr                 the release binary
  Frameworks/libpdfium.dylib
  Resources/AppIcon.icns
  Resources/licenses/        Refr-LICENSE.txt, PDFium-LICENSE.txt
```

`Info.plist` registers Refr as an **alternate** handler for PDFs (so it doesn't take over as
the default PDF app) and the **owner** of `.pdfspace` files. The minimum system version is
macOS 13.

## Architecture

Releases are arm64 only, because the bundled PDFium is the `mac-arm64` build. An Intel or
universal build needs the `mac-x64` PDFium, a separate `x86_64-apple-darwin` build, and
`lipo` to combine the binaries and libraries.
