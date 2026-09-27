# Architecture

```
refr (GPUI app) ──► refr-pdf (PDFium thread) ──► refr-core
        └──────────────────────────────────────► refr-core
```

Refr is three crates. `refr-core` is the document model and has no UI or PDF dependencies.
`refr-pdf` wraps PDFium. `refr` is the macOS app built with GPUI. For how this maps onto
PdfSpace, see [Porting notes](porting-notes.md).

## Source map

| File | Responsibility |
|---|---|
| `refr-core/src/model.rs` | `PdfWorkspace`, `PdfSource`, `PdfPageState`, `Annotation`, `CommentReply` |
| `refr-core/src/geometry.rs` | `PointD` and `RectD` |
| `refr-core/src/layout.rs` | Page ↔ display transforms and viewport placement for the three layouts |
| `refr-core/src/editor.rs` | `EditorSession`: transactions, undo/redo, tools, selection, page operations |
| `refr-core/src/workspace_json.rs` | `.pdfspace` load and save, validation, combining and extracting |
| `refr-core/src/text_layout.rs` | Annotation text wrapping with Helvetica widths |
| `refr-core/src/page_range.rs` | Parsing of "1, 3-5" page ranges |
| `refr-pdf/src/lib.rs` | `Engine`: the PDFium thread, import, words, search, render, export, PNG, split |
| `refr-pdf/src/frame.rs` | `PageFrame`: logical ↔ PDF user space |
| `refr-pdf/src/draw.rs` | Annotations written as PDF page objects |
| `refr-pdf/src/sample.rs` | The generated sample report |
| `refr/src/main.rs` | Startup, key bindings, menus, files opened from Finder |
| `refr/src/workbench.rs` | `Workbench` state and commands: documents, modes, dialogs, files, recovery |
| `refr/src/shell.rs` | Chrome rendering: title bar, global bar, tool panel, floating tools, rail, Home |
| `refr/src/panels.rs` | Side panels and the Organize grid |
| `refr/src/about.rs` | The About Refr window |
| `refr/src/document.rs` | `DocumentView`: session, zoom, scroll, image caches, text selection, inline text |
| `refr/src/viewport.rs` | Page painting, annotation marks, hit testing, pointer tools |
| `refr/src/glyphs.rs` | Helvetica outlines for rotated annotation text |
| `refr/src/text_input.rs` | Single-line text field |
| `refr/src/storage.rs` | Loading, atomic writes, recovery copy, recent files |
| `refr/src/theme.rs` | Colors, palette, buttons and icons |
| `refr/src/tests.rs` | Headless UI tests |

## Document state (`refr-core`)

A `PdfWorkspace` holds immutable source PDFs (`Arc<[u8]>`, shared across every history
snapshot), ordered page records and annotation records. A page points to a source and
a one-based source page, or to no source for a blank page. Rotation and crop are
workspace properties. They never change source bytes.

`EditorSession::execute` validates the new snapshot and records one named history entry
(up to 100). Selection, tool and current page are view state and never enter history.
The saved snapshot is tracked by `Arc` identity, so undoing back to it makes the
document clean again.

The JSON format matches PdfSpace's `System.Text.Json` output: PascalCase names, numeric
enums and base64 source bytes. Computed properties that PdfSpace writes are ignored when
reading. `workspace_json::validate` applies PdfSpace's guardrails: 64 MB of sources,
1–4096 pages, 50,000 annotations, 100,000 points per path.

## Coordinate spaces

- **Logical page space:** PDF points with a top-left origin. This is the page as PDFium
  shows it with no extra rotation: the effective box (media ∩ crop) turned by the page's
  own `/Rotate`. Annotations, crops and word boxes live here.
- **Display space:** logical space after workspace crop and quarter-turn rotation
  (`layout::to_display` / `to_page`).
- **Screen space:** display space × zoom, placed by `layout::arrange`.

`refr_pdf::PageFrame` maps logical space to and from PDF user space. Export uses that
mapping as a matrix, so annotations land correctly on pages with an intrinsic `/Rotate`
or an offset box.

## PDF engine (`refr-pdf`)

PDFium is not thread-safe. `Engine::start` binds it on a dedicated thread that owns every
parsed document (an LRU of eight). `Engine` is a cloneable handle whose calls send a
closure to that thread and block for the answer. The app calls it from GPUI's background
executor. Parsed documents borrow their bytes through an `Arc` kept in the same cache
entry.

Rendering applies workspace rotation and crop inside PDFium: a crop-sized bitmap and a
negative origin. It returns BGRA, which is what GPUI's `RenderImage` expects. Export copies
source pages into a new document, sets rotation and CropBox, writes annotations as path
and text objects, and adds comments as PDF text annotations.

## App (`refr`)

- `Workbench` (`workbench.rs`, `shell.rs`, `panels.rs`) owns the open `DocumentView`s,
  tool modes, side panels, the modal prompt, file commands and debounced recovery.
- `DocumentView` (`document.rs`, `viewport.rs`) owns an `EditorSession`, zoom and scroll,
  page-image and thumbnail caches, word caches and the current pointer gesture.
  - In the canvas prepaint, `prepare` gets the real bounds. It lays out pages, picks the
    best cached image (a sharper one is requested in the background, one at a time per
    page), converts annotations to screen-space marks and stores the placements for hit
    testing.
  - Pointer handlers turn window positions into page points with the stored placements,
    so rendering and input share one transform.
  - Images leaving the caches are released from GPUI's atlas with `drop_image`.
- Annotation text is painted with GPUI's text system on upright pages. On rotated pages
  `glyphs.rs` lays out outlines from the system Helvetica and fills them as paths
  through the page transform, because GPUI can't rotate text.
- Viewport shortcuts use the context `Viewport && !TextInput`, so single-letter tool
  keys never steal typing from the inline text editor.

## Tests

- `refr-core`: editing, history, geometry and PdfSpace JSON compatibility.
- `refr-pdf`: real PDFium import, words, search, rotation/crop rendering, export
  round trips, PNG and split.
- `refr`: a real workbench in GPUI's headless test window, driven by simulated pointer
  and keyboard events. Covers highlighting, drawing, moving and resizing, undo, rotated
  pages, the text and comment tools, cropping, copying, zoom, shortcuts and the About
  window. These tests set `REFR_DATA_DIR` so they never touch the user's recovery file.

See [Testing](testing.md) for the full list and how to write new tests.
