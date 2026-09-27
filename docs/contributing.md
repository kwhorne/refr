# Contributing

Bug reports, fixes and improvements are welcome at
[github.com/kwhorne/refr](https://github.com/kwhorne/refr).

## Reporting bugs

Open an issue with:

- your macOS version and the Refr version (**Refr › About Refr**)
- what you did, what you expected, and what happened, including any status bar message
- a PDF that shows the problem, if you can share one. **Don't attach confidential
  documents.** Try to reproduce the problem with the sample report or a public PDF first.

## Setting up

Follow [Building from source](building.md), then check that everything passes:

```sh
cargo test
```

## Where changes go

| Change | Crate |
|---|---|
| Document model, file format, validation, geometry, editing commands, undo | `refr-core` |
| Anything that reads, renders or writes PDF bytes | `refr-pdf` |
| UI, input, panels, dialogs, files on disk | `refr` |

Keep `refr-core` free of UI and platform dependencies, so it can be reused by other hosts.
Every document change goes through `EditorSession::execute` (or a method that calls it), so
it's validated and can be undone. Never edit a `PdfWorkspace` in place from the app.

If you change the `.pdfspace` format, keep it compatible with PdfSpace, and update
[The .pdfspace format](workspace-format.md) and the compatibility test in
`crates/refr-core/src/lib.rs`.

## Code style

- Run `cargo fmt`, and keep the build free of warnings.
- Match the code around you: its naming, idioms and comment density.
- Start each module with a `//!` comment that says what it's for. Add comments where the
  reason isn't obvious from the code, not to repeat what it does.
- User-facing text is short and plain, and it doesn't overstate. For example, a crop "hides
  content; it does not redact it".
- Don't add features that pretend to provide security, such as fake redaction or encryption.
  Leave them out, or list them as unavailable.

## Tests

Add or update tests with every change. See [Testing](testing.md).

- Model and format changes: unit tests in `refr-core`.
- Engine changes: tests in `crates/refr-pdf/src/tests.rs` against real PDFium.
- UI behavior: a headless test in `crates/refr/src/tests.rs` that drives the workbench with
  simulated events.

## Commits and pull requests

- One logical change per commit. Keep the subject line short and in the imperative ("Show
  annotation text rotated on rotated pages"), and explain the why in the body.
- Update the docs in `docs/` when you change behavior users can see.
- Describe in the pull request what you changed, how you tested it, and include screenshots
  for UI changes.

## License

Refr is MIT licensed. By contributing, you agree that your contributions are licensed under
the same terms.
