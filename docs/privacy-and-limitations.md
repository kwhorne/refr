# Privacy and limitations

## Everything stays on your Mac

- Refr has no account, sign-in or subscription.
- It doesn't upload documents, send telemetry or analytics, or check for updates. It makes
  no network connections of its own. The only time it reaches the internet is when you
  click a link in About Refr, which opens your browser.
- Documents are read into memory. Refr writes only files you ask it to save, plus the
  files below.

| File | Holds |
|---|---|
| `~/Library/Application Support/Refr/recovery.pdfspace` | The [recovery copy](saving-and-exporting.md#the-recovery-copy), including its original PDF. It exists only while a document has unsaved changes. |
| `~/Library/Application Support/Refr/recent.json` | Paths of the last ten files you opened or saved |
| `$TMPDIR/Refr/<title>-print.pdf` | The PDF created when you print |

To remove all traces, quit Refr and delete `~/Library/Application Support/Refr`.

## Workspaces contain the whole document

A `.pdfspace` workspace embeds the **complete original PDF**, including anything that's
cropped out or hidden under an annotation. Share a workspace only with people who may see
the entire original document.

## What Refr deliberately doesn't do

Refr leaves features out rather than doing them badly. The **All tools** panel lists the
ones that are missing, disabled, so nobody mistakes a visual effect for a security feature.

| Not supported | Why it matters |
|---|---|
| **Redaction** | Cropping only hides content with the page's crop box, and covering text with a shape only draws on top of it. The original text and images are still in the PDF and can be recovered. To remove sensitive content, use a tool that does true redaction. |
| **Certificate-based digital signatures** | A drawn signature is a visual mark. It doesn't prove who signed or show whether the document changed afterwards. See [Fill and sign](fill-and-sign.md#what-a-drawn-signature-is-and-isnt). |
| **Encryption and password protection** | Refr can't add a password, and it can't open password-protected PDFs. |
| **OCR** | Scanned pages without a text layer can't be searched, selected or marked up by word. You can still draw on them and mark areas. |
| **Interactive form filling** | Form fields aren't filled. Add text on top of them instead. |
| **Editing existing page content** | Refr adds content on top of pages. It doesn't change the PDF's own text or images. |

A watermark added with **Add watermark** is also a visual mark. It's easy to remove from the
workspace, and in an exported PDF it's ordinary page content, not a protection.

## Other limits

| Limit | Value |
|---|---|
| Open documents | 8 |
| Pages per document | 4,096 |
| Combined size of source PDFs in a document | 64 MB |
| Annotations per document | 50,000 |
| Undo steps per document | 100 |
| Pages in a split archive | 300 |
| Exported PNG | Longest side 4096 px |

Exported annotation text uses Helvetica with Windows-1252 characters. Text in other scripts
shows in the viewer but may be missing from exported PDFs.
