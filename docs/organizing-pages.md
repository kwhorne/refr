# Organizing pages

Page changes are workspace operations. Refr records the new page order, rotation and crop
in the workspace and never edits the source PDF. Every operation can be undone with
**⌘Z**.

## Organize pages mode

Choose **All tools › Organize pages**. The document is replaced by a grid of page
thumbnails, and the tool panel shows the page commands. The selected page has a blue
border.

- Click a page to select it.
- Drag a page onto another page to move it to that position.
- Double-click a page to open it in the document view.
- Click **Back to document** to leave the grid.

Choosing a drawing tool, or switching to another tab, also leaves Organize mode.

## Page commands

These apply to the selected page (the current page in the document view):

| Command | Does |
|---|---|
| **Rotate clockwise** and **Rotate counterclockwise** | Turn the page 90°. **Rotate** on the navigation rail rotates clockwise from any mode. |
| **Move page earlier** and **Move page later** | Move the page one position |
| **Duplicate page** | Inserts a copy right after the page, with copies of its annotations |
| **Delete page** | Removes the page after you confirm |
| **Insert blank page** | Inserts a blank page after the current one, with the same size as the current page |
| **Insert from PDF** | Appends the pages of other PDFs or workspaces (see [Combining files](#combining-files)) |
| **Extract pages** | Exports chosen pages as a new PDF (see [Extracting pages](#extracting-pages)) |
| **Split into PDFs** | Saves every page as its own PDF in a ZIP archive (see [Splitting](saving-and-exporting.md#split-into-single-page-pdfs)) |

A workspace always has at least one page, so you can't delete the last one.

Annotations stay with their page when you rotate or move it. On a rotated page they turn
with the content.

## Combining files

**Combine files** (in All tools and on Home) and **Insert from PDF** (in Organize mode) let
you pick one or more PDFs or `.pdfspace` workspaces. Their pages are **appended to the end**
of the current document, with any annotations and bookmarks they have. Drag pages
afterwards to put them where you want.

The combined workspace keeps each source PDF as a separate source. Adding the same file
twice is fine, because every page gets new identifiers. The combined sources can be at most
64 MB, and a workspace can have at most 4,096 pages.

## Cropping

1. Choose **Crop page** in Edit mode, or **Crop pages** in All tools.
2. Drag a rectangle over the part of the page to keep.

The page now shows only that area, in the viewer and in exported PDFs. **Reset page crop**
in Edit mode shows the whole page again. Crop is stored per page, and you can undo it.

> **Cropping is not redaction.** The content outside the crop is still in the PDF. It's
> hidden by the page's crop box and can be recovered by anyone with a PDF editor. Never
> crop to hide sensitive information.

## Extracting pages

Choose **Extract pages** (Organize mode) or **Extract selected pages** (Convert mode).
Type the pages to extract and click **Extract**:

| You type | Refr extracts |
|---|---|
| `3` | Page 3 |
| `1, 3-5` | Pages 1, 3, 4 and 5 |
| `all`, or nothing | Every page |

Pages are extracted in the order you list them, and duplicates are skipped. Ranges must go
upwards (`3-5`, not `5-3`). Refr then asks where to save the new PDF, suggesting
`<title>-extract.pdf`. The extracted PDF includes the pages' annotations, rotation and
crop. The current document doesn't change.

## Bookmarks and page labels

Pages can have workspace bookmark names, which are shown under thumbnails as `3 · Summary`.
See [Bookmarks](reading-and-navigating.md#bookmarks).
