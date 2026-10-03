# Changelog

All notable changes to Refr. Release notes for each version are also on the
[releases page](https://github.com/kwhorne/refr/releases).

## [0.2.0] - 2026-09-27

Requires an Apple silicon Mac with macOS 13 or later.

### Fixed

- **The recovery copy only exists while you have unsaved changes.** Before, it was
  never removed, so Refr offered to restore it on every launch, even after you'd
  saved. Now:
  - It follows the most recently changed document with unsaved changes, and is
    deleted once nothing is unsaved: after you save, undo back to the saved state,
    or close without saving.
  - Quitting with the menu, ⌘Q or the close button writes a pending copy right
    away, so a change made in the last second before quitting isn't lost.
  - At launch, Refr asks **Restore unsaved changes?** with **Restore** or
    **Discard**. A restored copy opens as unsaved, so closing it asks first.
- **Workspaces keep coordinates exactly.** Annotation positions could drift in the
  last decimal after a save and reload. They're now saved and loaded exactly.

### Added

- **Complete documentation** in [docs/](https://github.com/kwhorne/refr/tree/main/docs):
  a user guide (getting started, the workspace, annotating, comments, organizing
  pages, fill and sign, saving and exporting, shortcuts, privacy and limitations,
  troubleshooting) and a developer guide (building, architecture, the `.pdfspace`
  format, testing, releasing, porting notes, contributing).

### Good to know

- Cropping is not redaction, and a drawn signature is a visual mark, not a
  certificate-based digital signature.
- OCR, encryption, password-protected PDFs and form filling aren't supported.
- Exported annotation text uses the standard Helvetica font, which covers Western
  European characters.

## [0.1.0] - 2026-09-27

The first release of Refr: a native, local-first PDF workspace for macOS, written in
Rust with GPUI and PDFium. It is a port of
[PdfSpace](https://github.com/wieslawsoltes/PdfSpace). Requires an Apple silicon Mac
with macOS 13 or later.

### Added

- **Read and navigate:** up to eight documents in tabs; continuous, single-page and
  two-page layouts; fit page and fit width; ⌘-scroll zooms around the pointer;
  thumbnails, bookmarks, full-text search, and text selection with copy.
- **Review and annotate:** highlight, underline and strike out text; comments with
  replies and resolve; freehand ink, shapes, lines and arrows; stamps, check marks
  and inline text. Move, resize, recolor and delete, with undo.
- **Organize:** drag thumbnails to reorder pages; rotate, duplicate, delete and
  insert blank pages; combine PDFs; crop; extract page ranges; split into
  single-page PDFs.
- **Fill and export:** text, initials and drawn signatures; watermarks and page
  numbers. Export a PDF, a PNG of the current page or plain text, or save an
  editable `.pdfspace` workspace that is compatible with PdfSpace. A recovery copy is
  saved automatically.
- **Export keeps the original pages.** Text in exported PDFs stays selectable and
  searchable. Annotations become page content, and comments become standard PDF
  comments.

### Good to know

- Cropping is not redaction, and a drawn signature is a visual mark, not a
  certificate-based digital signature.
- OCR, encryption, password-protected PDFs and form filling aren't supported yet.
- Exported annotation text uses the standard Helvetica font, which covers Western
  European characters.
