# Porting notes

Refr is a port of [PdfSpace](https://github.com/wieslawsoltes/PdfSpace) by Wiesław Šoltés,
from C#, Uno Platform, SkiaSharp and PdfPig to Rust, GPUI and PDFium. It keeps PdfSpace's
workspace model, tools, layout and file format, and changes the platform layer underneath.

## Where things went

| PdfSpace | Refr |
|---|---|
| `PdfSpace.Core`: models and validated JSON | `refr-core`: `model.rs`, `geometry.rs`, `workspace_json.rs` |
| `PdfSpace.Layout`: page layouts and transforms | `refr-core`: `layout.rs` |
| `PdfSpace.Editing`: transactions, undo/redo, page operations | `refr-core`: `editor.rs` |
| `PdfSpace.Documents`: import, words, search, page ranges | `refr-pdf`: `lib.rs`, plus `refr-core`: `page_range.rs` |
| `PdfSpace.Skia`: rendering, annotation painting, export, sample | `refr-pdf`: `lib.rs`, `draw.rs`, `frame.rs`, `sample.rs` |
| `PdfSpace.Controls`: buttons, icons, panels, theme | `refr`: `theme.rs`, `text_input.rs`, `assets/icons/` |
| `PdfSpace.Viewer`: viewport and thumbnails | `refr`: `document.rs`, `viewport.rs`, `panels.rs` |
| `PdfSpace.Storage`: files, recovery, clipboard, print | `refr`: `storage.rs`, `workbench.rs` |
| `PdfSpace.Workbench`: shell, tools, panels, workflows | `refr`: `workbench.rs`, `shell.rs`, `panels.rs`, `about.rs` |

As in PdfSpace, the document model has no UI dependencies. `refr-core` doesn't depend on GPUI
or PDFium, so a WebAssembly host (for example Svelte with `wasm-bindgen`) could reuse it and
put its own renderer on top.

## What stayed the same

- **The `.pdfspace` format.** PascalCase names, numeric enums, base64 sources and the same
  validation limits. Computed properties that PdfSpace writes are accepted and ignored. See
  [The .pdfspace format](workspace-format.md).
- **The editing model.** Immutable snapshots, one validated transaction per command, named
  history entries, 100 undo steps, and dirty state tracked by snapshot identity.
- **The workbench.** Eight tabs, the All tools, Edit, Convert, E-Sign and Organize modes,
  floating quick tools, the navigation rail, the five side panels and the Home view.
- **Honesty about limits.** No simulated OCR, protection or redaction, and the same
  wording about crops and drawn signatures.
- **The sample.** The generated six-page "Circular futures" report.
- **The icons.** PdfSpace's original vector paths (MIT).

## What's different

| Area | PdfSpace | Refr |
|---|---|---|
| Platform | Browser (WebAssembly) and desktop, through Uno | Native macOS app |
| PDF engine | PdfPig for parsing and text, Skia for rendering | PDFium for parsing, text, rendering and writing |
| PDF export | A new, flattened visual PDF drawn through Skia | Original pages copied with PDFium. Text stays selectable and searchable, rotation and crop become page properties, annotations become vector content, and comments become PDF text annotations. |
| Threading | .NET tasks | PDFium isn't thread-safe, so it runs on one dedicated thread that owns every parsed document. The app calls it from GPUI's background executor. |
| Page images | Cached Skia pictures | Bitmaps rendered in eighth-zoom steps, with the nearest cached image shown while a sharper one renders. Images leaving the cache are freed from GPUI's atlas. |
| Rotated annotation text | Skia rotates text | GPUI can't rotate text, so on rotated pages Refr fills Helvetica glyph outlines as paths. |
| Printing | Browser print or the platform print dialog | Opens a printable PDF in the default PDF viewer |
| Storage | Browser local storage or files | Files, with atomic writes and a recovery copy in Application Support |
| Keyboard | Uno key routing | GPUI key contexts. Viewport shortcuts use `Viewport && !TextInput` so tool letters never steal typing. |

## Coordinate systems

PdfSpace and Refr both store annotation geometry in top-left page points. Refr defines them
precisely as **logical page space**: the page's effective box (media box ∩ crop box) turned
by its own `/Rotate`. `refr-pdf::PageFrame` maps it to PDF user space, so workspaces made in
either app put marks in the same place on pages with an intrinsic rotation or an offset
media box. See [Coordinate spaces](architecture.md#coordinate-spaces).
