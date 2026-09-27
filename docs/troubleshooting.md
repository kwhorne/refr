# Troubleshooting

Refr shows problems in red in the status bar at the bottom of the window. Look up the
message below.

## Opening files

**"This PDF is password protected. Password-protected documents are not supported yet."**
Refr can't open encrypted PDFs. Remove the password in the app that set it, or open the
file in Preview, then choose **File › Export as PDF** without encryption, if you're allowed
to.

**"… is not a PDF or .pdfspace file."**
Refr opens only files ending in `.pdf` or `.pdfspace`. Check the file extension.

**"This file is too large."** or **"The combined source PDFs exceed 64 MB."**
A document's source PDFs can total at most 64 MB. Split the PDF with another tool, or
compress its images, then open the parts separately.

**"Close a document before opening another. Refr keeps up to eight documents open."**
Close a tab with **⌘W** and try again.

**A workspace won't open: "Unsupported workspace format version", "Missing source document" or
"Invalid …".**
The `.pdfspace` file is damaged, was edited by hand, or comes from a newer format. Refr
checks every workspace before opening it and refuses files it can't trust. See
[validation](workspace-format.md#validation) for the rules. If you have the original PDF,
open that instead.

**"Page N could not be rendered."**
PDFium couldn't draw that page, usually because the PDF is damaged. The rest of the
document still works. Try opening and re-saving the PDF in Preview.

## Editing

**Highlighting says "No text here."**
There's no selectable text under the drag, typically on scanned pages. Drag a rectangle over
the area to mark it instead. See [Text markup](annotating.md#text-markup).

**Typing letters switches tools.**
Single-letter shortcuts (V, H, T, D) only apply when the document has focus. Click into the
text field you want to type in first. Text typed in the inline text editor, dialogs and
panels always goes to the field.

**An annotation can't be resized.**
Highlights, underlines, strikethroughs and comments have a fixed size. Move them, or delete
them and mark the text again.

**"Keep at least one page in the document."**
A document always has at least one page. Insert a blank page first if you want to replace
the last one.

## Saving and exporting

**"Recovery could not be saved: … Save your workspace now."**
Refr couldn't write to `~/Library/Application Support/Refr`, usually because the disk is
full or permissions have changed. Save a workspace with **⌘S** right away, then free up disk
space.

**Characters are missing in the exported PDF.**
Exported annotation text uses Helvetica with Western European (Windows-1252) characters.
Other scripts appear in Refr but may be missing in the PDF.

**My bookmarks aren't in the exported PDF.**
Workspace bookmarks are only saved in `.pdfspace` workspaces. See
[Bookmarks](reading-and-navigating.md#bookmarks).

**Printing doesn't show a print dialog.**
Refr opens a printable PDF in your default PDF viewer. Print from there with **⌘P**. If
nothing opens, set a default app for PDFs in Finder with **Get Info › Open with**.

**"Splitting is limited to 300 pages."**
Extract smaller page ranges first, then split each part.

## Starting Refr

**"Refr could not start its PDF engine."**
Refr couldn't load PDFium (`libpdfium.dylib`). In the released app it's inside the bundle,
so reinstall Refr from the latest DMG. When building from source, run
`scripts/fetch-pdfium.sh` or set `REFR_PDFIUM`. See [Building from source](building.md).

**macOS says Refr can't be opened.**
Builds from [Releases](https://github.com/kwhorne/refr/releases) are notarized and open
normally. A build you made with `scripts/bundle.sh` is only signed ad hoc. Right-click it and
choose **Open**, or run it from Terminal.

**Refr asks to restore unsaved changes when it starts.**
Refr quit last time while a document had unsaved changes, or it quit unexpectedly. Choose
**Restore** to get the changes back, or **Discard** to delete them.

## Reporting a problem

Open an issue at [github.com/kwhorne/refr/issues](https://github.com/kwhorne/refr/issues)
with your macOS version, the Refr version (**Refr › About Refr**), the exact message and the
steps that caused it. Don't attach confidential documents.
