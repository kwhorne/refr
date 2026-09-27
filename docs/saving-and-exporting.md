# Saving and exporting

Refr never writes over a file you opened. Your work is kept in two kinds of output:

| Output | Command | Contains | Can you edit it later? |
|---|---|---|---|
| **Workspace** (`.pdfspace`) | **⌘S** | The original PDF bytes plus your annotations, comments, bookmarks and page operations | Yes. Open it in Refr or PdfSpace and everything is still editable. |
| **Exported PDF** (`.pdf`) | **⇧⌘S** | A new PDF with your changes applied to the pages | Only as a PDF. Annotations are part of the page content. |

A good habit is to keep the workspace as your working copy and export a PDF whenever you
need to send the document to someone.

## Save a workspace

Press **⌘S**, click **Save** in the global bar, or choose **Save editable workspace** in
All tools.

- If the document came from a `.pdfspace` file or was saved as one before, Refr saves over
  that workspace without asking.
- Otherwise Refr asks where to save, suggesting `<title>.pdfspace` in your Documents folder.

To save a copy under a new name, press **⌥⌘S** (**File › Save Workspace As…**) or choose
**Convert › Editable workspace**. Later **⌘S** saves go to the new file.

After saving, the unsaved-changes dot disappears from the tab and the file is added to
Recent files. The workspace embeds the complete original PDF. Treat it as a copy of the
document, and see [Privacy and limitations](privacy-and-limitations.md).

Workspaces are compatible with PdfSpace. The format is described in
[The .pdfspace format](workspace-format.md).

## Export a PDF

Press **⇧⌘S**, click **Export PDF**, or choose **Convert › PDF document**. Refr suggests
`<title>.pdf`.

The exported PDF is built from the **original pages**, not from images of them:

- The page content is copied as is, so text stays selectable and searchable, and vector
  graphics stay sharp.
- Page order, duplicates, deletions and inserted blank pages follow the workspace.
- Workspace rotation is applied to each page's rotation, and crops become the page's crop
  box.
- Annotations are drawn into the page as vector graphics and text.
- Comments are also added as standard PDF comments. See
  [Comments in exported PDFs](comments.md#comments-in-exported-pdfs).

What an exported PDF doesn't have:

- Editable Refr annotations. Keep the workspace for that.
- Workspace bookmarks, resolved status, and the document title and author.
- Interactive form fields you filled with text. The fields are left untouched, and your
  text is drawn on top.

Text in annotations uses the PDF standard font Helvetica, which covers Western European
characters (Windows-1252). Characters outside that set may not appear in the exported PDF.

To export only some pages, see [Extracting pages](organizing-pages.md#extracting-pages).

## Export a PNG image

**Convert › PNG image · current page** saves the current page as a PNG at 2× resolution
(144 pixels per inch), with the longest side capped at 4096 pixels. The image is rendered
from the exported PDF, so annotations look exactly as they do there.

## Export plain text

**Convert › Plain text** saves a `.txt` file with the text of every page, each starting with
a `Page N` heading. The text of added text annotations and comments follows each page's
text. Pages without a text layer, such as scans, have no text.

## Split into single-page PDFs

**Convert › Split into single-page PDFs**, or **Split into PDFs** in Organize mode, saves a
ZIP archive with one PDF per page, named `<title>-page-001.pdf`, `<title>-page-002.pdf`, and
so on. Each PDF has that page's annotations, rotation and crop. You can split documents of
up to 300 pages into archives of up to 256 MB.

## Print

Press **⌘P** or click **Print**. Refr exports the document to a temporary PDF and opens it
in your default PDF viewer, usually Preview. Print from there with **⌘P**. The printed copy
looks exactly like an exported PDF.

## The recovery copy

While a document has unsaved changes, Refr keeps a recovery copy of it on your Mac:

```
~/Library/Application Support/Refr/recovery.pdfspace
```

- The copy is written about a second after each change, and the status bar confirms it.
  When you quit, a pending copy is written right away.
- There is one recovery copy. It holds the document with unsaved changes that you changed
  most recently. If that document is saved, closed or undone back to its saved state, the
  copy switches to the next document with unsaved changes.
- When no open document has unsaved changes, for example after you save or when you
  close a document and discard its changes, the copy is deleted.

If Refr quits unexpectedly, or you quit with unsaved changes, the next launch asks
**Restore unsaved changes?**

- **Restore** opens the copy in a new tab. It's marked as unsaved, so closing it asks
  first, and the recovery copy is kept until you save it.
- **Discard** deletes the copy.

The recovery copy is a safety net, not a save. Save a workspace to keep your work
permanently.

## How files are written

Refr writes every file to a temporary file next to the destination, then renames it into
place. If the Mac crashes during a save, you're left with either the old file or the new
one, never half of each. Large exports run in the background, and the window stays
responsive.
