# Getting started

## Requirements

- A Mac with Apple silicon (M1 or later)
- macOS 13 Ventura or later

## Install

1. Download `Refr-<version>-arm64.dmg` from
   [Releases](https://github.com/kwhorne/refr/releases).
2. Open the disk image and drag **Refr** to **Applications**.
3. Open Refr from Applications or Spotlight.

Refr is signed with a Developer ID and notarized by Apple, so it opens without a
Gatekeeper warning. To build it yourself, see [Building from source](building.md).

## The first launch

When you start Refr without a file, it opens a six-page **sample report**, so you can try
the tools right away. Nothing you do to the sample affects anything on disk.

If Refr quit with unsaved changes last time, it asks **Restore unsaved changes?** Choose
**Restore** to open them in a new tab, or **Discard** to delete the recovery copy. See
[The recovery copy](saving-and-exporting.md#the-recovery-copy).

## Opening files

Refr opens PDF files (`.pdf`) and Refr/PdfSpace workspaces (`.pdfspace`). You can open a
file in any of these ways:

- Press **⌘O**, or click **+** at the right end of the title bar, and pick one or more files.
- Drag files from Finder onto the Refr window.
- Pick a file under **Recent files** in the Home view.
- In Finder, choose **Open With › Refr**, or drop a file on Refr's icon in the Dock.
- From Terminal: `open -a Refr report.pdf`.

Each file opens in its own tab. Refr keeps up to **eight documents** open at once, so
close one (**⌘W**) before you open a ninth.

Opening a PDF never changes it. Refr reads the file into memory, and every edit applies to
that in-memory copy until you save a workspace or export a new PDF.

## The Home view

Click the house icon next to the Refr name, or press **⇧⌘H**, to show Home. It has four
shortcuts:

| Card | Does |
|---|---|
| **Open a file** | Opens a PDF or `.pdfspace` workspace |
| **Create a PDF** | Starts a new document with one blank A4 page |
| **Open the sample** | Opens the sample report in a new tab |
| **Combine files** | Appends pages from other PDFs to the current document |

Below the cards, **Recent files** lists the last ten files you opened or saved. Files that
no longer exist are left out. Press **⇧⌘H** again, or click a tab, to go back to your
document.

## Creating a new PDF

Press **⌘N**, choose **File › New Blank PDF**, or click **Create a PDF**. The new
document has one blank A4 page (595 × 842 pt). Add text, drawings and pages, then
[export it](saving-and-exporting.md#export-a-pdf).

## Next steps

- [The workspace](interface.md) walks through each part of the window.
- [Annotating](annotating.md) explains the markup and drawing tools.
- [Keyboard shortcuts](keyboard-shortcuts.md) is worth keeping open while you learn.
