# Refr

A local-first PDF workspace for macOS, written in Rust.

Refr draws its interface with [GPUI](https://gpui.rs) (the UI framework from Zed) and
reads, renders and writes PDFs with [PDFium](https://pdfium.googlesource.com/pdfium/),
the engine in Chromium. It is a port of [PdfSpace](https://github.com/wieslawsoltes/PdfSpace)
(C#, Uno Platform, SkiaSharp): the same workspace model, tools and layout. It also reads
and writes PdfSpace's `.pdfspace` workspaces. There is no account, no upload and no
telemetry.

## Highlights

- **Read and navigate:** up to eight documents in tabs; continuous, single-page and
  two-page layouts; fit page/width; ⌘-scroll zoom around the pointer; thumbnails,
  workspace bookmarks, full-text search and text selection with copy.
- **Review and annotate:** highlight, underline and strike out text; comments with
  replies and resolve; freehand ink, rectangles, ellipses, lines and arrows; stamps,
  check marks and inline text. Move, resize, recolor and delete, with 100 steps of undo.
- **Organize:** drag thumbnails to reorder; rotate, duplicate, delete and insert blank
  pages; combine PDFs; crop; extract page ranges; split into a ZIP of single-page PDFs.
- **Fill and export:** text, initials and drawn signatures; watermarks and page numbers.
  Export a PDF, a PNG of the current page, plain text, or an editable `.pdfspace`
  workspace. A recovery copy is saved automatically.

### Export keeps the original pages

PdfSpace exports a flattened, re-drawn PDF. Refr copies the original pages with PDFium,
so text stays selectable and searchable. Workspace rotation and crop become page
properties, annotations become vector page content, and comments become standard PDF
text annotations.

| Operation | Output | Behaviour |
|---|---|---|
| **Save workspace** (⌘S) | `.pdfspace` | Original PDF bytes plus editable annotations, comments, bookmarks and page operations. Treat it as containing the full original document. |
| **Export PDF** (⇧⌘S) | `.pdf` | A new PDF. Annotations are burned into the page content and can't be edited as annotations afterwards. |

**Cropping is not redaction. A drawn signature is not a certificate-based digital
signature.** OCR, encryption, password-protected files, form filling and secure
redaction aren't supported, and Refr doesn't pretend otherwise.

## Layout

| Crate | Responsibility |
|---|---|
| `refr-core` | Workspace model, `.pdfspace` JSON and validation, page geometry and layout, transactional editing with undo/redo. Has no UI or platform dependencies, so a WebAssembly/Svelte host can reuse it. |
| `refr-pdf` | PDFium on its own thread: import, words with positions, search, rendering, export, PNG, split, and the generated sample document. |
| `refr` | The GPUI app: workbench, viewport and pointer tools, panels, dialogs, storage. |

See [docs/architecture.md](docs/architecture.md).

## Build and run

You need Rust 1.88 or later and the Xcode Command Line Tools. Full Xcode isn't
required, because GPUI compiles its Metal shaders at startup.

```sh
scripts/fetch-pdfium.sh      # downloads libpdfium.dylib into vendor/pdfium
cargo run -p refr            # or: cargo run -p refr -- some.pdf
cargo test                   # core, engine and headless UI tests
scripts/bundle.sh            # target/release/Refr.app with PDFium in Frameworks
```

Refr finds PDFium in this order: `REFR_PDFIUM`, next to the executable,
`Contents/Frameworks` in the app bundle, then `vendor/pdfium/lib`.

## Shortcuts

| Command | Shortcut |
|---|---|
| Open, new blank PDF | ⌘O, ⌘N |
| Save workspace, save as, export PDF, print | ⌘S, ⌥⌘S, ⇧⌘S, ⌘P |
| Find | ⌘F |
| Undo, redo | ⌘Z, ⇧⌘Z |
| Copy selected text | ⌘C |
| Fit page, actual size, fit width, zoom | ⌘0, ⌘1, ⌘2, ⌘+ / ⌘− |
| Select, hand, text and draw tools | V, H, T, D |
| Previous or next page, first or last page | Page Up / Page Down, Home / End |
| Delete, nudge selected annotation | Delete, arrow keys (⇧ for 10 pt) |
| Cancel | Esc |
| Home, next or previous tab, close tab | ⇧⌘H, ⇧⌘] / ⇧⌘[, ⌘W |

## Known limitations

- Exported annotation text uses the PDF standard Helvetica font (WinAnsi characters).
- PDF document metadata (title, author) isn't written into exported files.
- Printing opens the exported PDF in your default viewer, where you print with ⌘P.

## License

MIT. The icons are PdfSpace's original vector paths (MIT). The text field is adapted
from GPUI's `input` example (Apache-2.0). PDFium is BSD-3-Clause/Apache-2.0; its license
files are in `vendor/pdfium` after fetching.
