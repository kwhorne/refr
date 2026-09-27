# Comments and review

A comment is a small note icon on the page with text, an author, a date, and optional
replies. Comments are for questions and suggestions that shouldn't change how the page
looks.

## Add a comment

1. Choose **Add a comment** in the floating tools, **Add comment** in Edit mode, **Add
   comments** in All tools, or **Add comment** at the top of the Comments panel.
2. Click the spot on the page the comment is about.
3. Type the comment in the dialog and click **Post**, or press Return.

The comment appears as a 23 × 23 pt note icon in the current color, and the Comments panel
opens. If you post an empty comment, nothing is added.

To change a comment's text, double-click its icon with **Select**, or select it and click
**Edit text**.

## The Comments panel

Open **Comments** from the navigation rail. It lists every comment **and every other
mark** in the document (highlights, drawings, text, stamps and so on) in page order. Each
card shows:

- the author's initial on the annotation's color, the author's name and the type of mark
- the page number and when it was created
- the text, if the mark has any
- replies, indented under the text
- **Resolved**, if it has been resolved

Click a card to go to that mark and select it. The selected card also shows:

- **Reply…**: type a reply and press Return to add it.
- **Resolve** or **Reopen**: marks the thread as done, or opens it again.
- **Delete**: removes the mark and all its replies.

Click **Hide resolved** to leave resolved marks out of the list, and **Show resolved** to
bring them back. Hiding them only changes the list. Resolved marks are still shown on the
page and included when you export.

All of these actions can be undone with **⌘Z**.

## Authors

Marks and replies you make are attributed to **You**. Marks from a `.pdfspace` workspace
someone else saved keep their original author and date.

## Comments in exported PDFs

When you [export a PDF](saving-and-exporting.md#export-a-pdf), each comment is drawn on the
page as a note icon and also added as a standard **PDF text annotation** (a "sticky
note"). Its replies are appended to the note text as `Author: reply`, and the author is
set as the note's creator. Other PDF readers, such as Preview and Acrobat, show these as
ordinary comments.

Resolved status isn't part of a PDF text annotation, so it isn't exported. Keep the
`.pdfspace` workspace to keep the full review state.

## Reviewing with other people

Refr has no accounts or cloud sync. To review with someone else:

1. Save an editable workspace (**⌘S**) and send them the `.pdfspace` file.
2. They open it in Refr or PdfSpace, add comments and replies, and save it again.
3. Open the returned workspace.

A workspace contains the full original PDF, so share it only with people who may see the
document. See [Privacy and limitations](privacy-and-limitations.md).
