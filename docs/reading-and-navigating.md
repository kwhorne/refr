# Reading and navigating

## Scrolling and changing pages

- Scroll with the trackpad or mouse wheel. Hold **Shift** to scroll sideways with a
  vertical wheel.
- Drag the scrollbar on the right edge of the document.
- With the **Hand** tool (**H**), drag the page to pan. With any tool, drag with the middle
  mouse button.
- **Page Up** and **Page Down** go to the previous and next page, and **Home** and **End**
  go to the first and last page.
- Type a number in the page field on the navigation rail and press Return.
- Click a page in the **Page thumbnails** panel.

The current page follows your scrolling. It is the page that tools such as **Rotate** and
**Crop** apply to, and it's highlighted in the thumbnails.

When nothing is selected, the arrow keys scroll the document. When an annotation is
selected, they move it instead.

## Page layouts

Click **Layout** on the navigation rail to cycle through the layouts:

| Layout | Shows |
|---|---|
| **Continuous** | All pages in one scrolling column (the default) |
| **Single page** | One page at a time |
| **Two pages** | Pages side by side in pairs, like an open book |

## Zoom

| Action | How |
|---|---|
| Zoom around the pointer | **⌘**-scroll or **⌃**-scroll |
| Zoom in and out | **⌘+** and **⌘−**, or **+** and **−** on the navigation rail (20 % per step) |
| Fit the whole page | **⌘0**, or **Fit page** |
| Fit the page width | **⌘2**, or **Fit width** |
| Actual size (100 %) | **⌘1** |
| An exact level | Click the percentage on the navigation rail and type 10–800 |

Refr shows a scaled copy of the page right away while it renders a sharper version in the
background.

## Thumbnails

Open **Page thumbnails** from the navigation rail. The list scrolls to the current page
when it opens. Each thumbnail is labeled with its page number and, if it has one, its
bookmark name. Blank pages have white thumbnails.

To reorder pages by dragging, use [Organize pages](organizing-pages.md).

## Bookmarks

Workspace bookmarks give pages names so you can jump to them quickly.

- In the **Bookmarks** panel, click **Bookmark current page**, type a name and press Return.
- Click a bookmark to go to its page, or click the pencil icon to rename it.
- To remove a bookmark, rename it to an empty name.

Bookmarks are saved in the `.pdfspace` workspace. They are separate from the PDF's own
outline (table of contents), which Refr doesn't show or change, and they aren't written
into exported PDFs.

## Find

Press **⌘F**, or click **Find in document**, then type and press **Return**.

- Refr searches the page text and the text of your annotations, such as comments,
  replies and added text.
- The first match is shown right away, and every match is listed in the **Find** panel
  with its page number. Matches in annotations are marked "annotation".
- Click a result to go to it. If the match is in an annotation, Refr selects that
  annotation.
- Turn on **Match case** in the Find panel to match upper and lower case exactly.
- Press **Esc** in the search field to go back to the document.

Find only works on selectable text. Scanned pages without a text layer have nothing to
find, because Refr doesn't do OCR.

## Selecting and copying text

1. Choose **Select** (**V**).
2. Drag across text that isn't covered by an annotation. The words are highlighted in
   reading order, and the status bar shows how many are selected.
3. Press **⌘C**, or click **Copy** in the selection bar.

From the same selection bar you can also highlight, underline or strike out the selected
text.

When an annotation with text (added text, a comment or a stamp) is selected, **⌘C**
copies its text instead.
