# Annotating

Annotations are marks you add on top of a page: highlights, drawings, text, stamps and
comments. They never change the original PDF content. Refr keeps them as editable records
in the workspace and writes them into the page only when you
[export a PDF](saving-and-exporting.md#export-a-pdf).

Most tools are in **Edit** mode. The ones you use most are also in the
[floating tools](interface.md#floating-tools).

## Text markup

| Tool | Result |
|---|---|
| **Highlight text** | A translucent band behind the text |
| **Underline text** | A line under the text |
| **Strikethrough text** | A line through the text |

Pick a tool and drag across text. Refr snaps the mark to the words between where you start
and where you stop, in reading order. A selection over several lines gets one mark per
line.

If there's no selectable text where you drag, for example on a scanned page, drag a
rectangle of at least 4 × 4 pt and Refr marks that area instead.

You can also select text with **Select** first, then click **Highlight**, **Underline** or
**Strikethrough** in the selection bar.

## Drawing tools

| Tool | How |
|---|---|
| **Draw freehand** (D) | Drag to draw a stroke |
| **Rectangle** | Drag from one corner to the opposite corner |
| **Ellipse** | Drag out the box that holds the ellipse |
| **Line** | Drag from start to end |
| **Arrow** | Drag from the tail to the head |

Very small drags are ignored (rectangles and ellipses under 10 × 10 pt, for example), so
a stray click doesn't leave a mark.

## Text

1. Choose **Add text** (**T**) and click where the text should start.
2. Type. New text wraps at 220 pt, or at the page edge if that comes first. Drag a
   corner afterwards to make the box wider or narrower.
3. Press **Return** or click outside the box to apply it, or press **Esc** to cancel.

To change existing text, double-click it with **Select**, or select it and click **Edit
text** in the selection bar. If you clear all the text and apply, the annotation is
deleted.

Text uses the current color and font size. It's drawn in Helvetica, the same font the
exported PDF uses, so line breaks match. On a rotated page, the text turns with the page.

## Stamps and check marks

- **Add stamp** puts a 150 × 40 pt **APPROVED** stamp where you click. To change the
  wording, double-click the stamp, or select it and click **Edit text**.
- **Add check mark** (in **E-Sign** mode) places a check mark where you click.

## Appearance

The **Appearance** section in Edit mode sets the style for new annotations:

- **Color**: blue, yellow, red, green, purple or black.
- **Font size**: 4–200 pt, in 1 pt steps. Used by text and stamps.
- **Stroke width**: 0.5–30 pt, in 0.5 pt steps. Used by lines, shapes, ink and the
  thickness of underline and strikethrough.

If an annotation is selected, changing any of these also changes that annotation, and you
can undo the change. When text gets a new font size, its box grows or shrinks to fit.

## Selecting, moving and resizing

Choose **Select** (**V**) and click an annotation to select it. Refr picks the topmost
annotation under the pointer. For lines, arrows and freehand strokes, you need to click
near the stroke itself.

| To | Do |
|---|---|
| Move | Drag the annotation, or press the arrow keys (1 pt; hold **Shift** for 10 pt) |
| Resize | Drag a corner handle. Text rewraps to the new width. Highlights, underlines, strikethroughs and comments can't be resized. |
| Recolor | Pick a color in the selection bar or in Edit mode |
| Edit its text | Double-click it, or click **Edit text** |
| See its details | Click **Annotation properties**, or open **Properties** |
| Delete | Press **Delete** or **Backspace**, or click the trash icon |
| Deselect | Click an empty area, or press **Esc** |

Press **Esc** while you're drawing or dragging to cancel.

## Undo and redo

Every change to the document can be undone: adding, moving, resizing, recoloring and
deleting annotations, page operations, bookmarks and document properties.

- **⌘Z** undoes and **⇧⌘Z** (or **⌃Y**) redoes. The status bar says what was undone.
- Refr keeps the last **100** changes for each document.
- Undo history belongs to the open document. It isn't saved in the workspace and is lost
  when you close the tab.
- If you undo back to the state you last saved, the tab's unsaved-changes dot goes away.

Changing tools, selecting and navigating are not document changes, so they aren't undone.
