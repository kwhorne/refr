# Fill and sign

**E-Sign** mode brings together the tools for filling in and signing a document that
wasn't built as a fillable form.

| Tool | Does |
|---|---|
| **Add text** | Type on the page: names, dates, amounts. See [Text](annotating.md#text). |
| **Add check mark** | Click a box to place a ✓ |
| **Draw signature** | Draw your signature with the trackpad or mouse |
| **Add initials** | Type your initials, and Refr places them on the page |
| **Add approval stamp** | Places an **APPROVED** stamp. See [Stamps and check marks](annotating.md#stamps-and-check-marks). |
| **Export signed copy** | Exports a PDF with everything burned into the pages |

## Drawing a signature

1. Choose **Draw signature**.
2. Drag to write your signature. Each stroke becomes its own signature mark.
3. Choose **Select** (**V**) to move or resize the strokes.

Signatures use a dark navy ink by default. If you've picked a color other than the default
blue, they use that color instead. Change the thickness with **Stroke width** in Edit mode.

To make a signature easier to move as one piece, write it in a single stroke where you can,
or zoom in first so each stroke is larger and easier to control.

## Initials

Click **Add initials**, type your initials and click **Add**. Refr converts them to upper
case and places them near the bottom-right corner of the current page, at 18 pt in the
current color. Drag them into position with **Select**.

## Filling in form fields

Refr doesn't fill interactive PDF form fields (AcroForm or XFA). Add text on top of the
fields instead. The exported PDF has your text as page content, and the original form
fields are left as they were.

## What a drawn signature is, and isn't

A drawn signature in Refr is a **visual mark**: vector strokes drawn on the page, just like
any other drawing. It is **not** a certificate-based digital signature. Refr doesn't:

- sign the document cryptographically or detect changes made after signing
- verify who signed
- keep an audit trail
- validate signatures in PDFs you open

That is the same kind of signature you get by printing, signing on paper and scanning. It's
fine for many everyday agreements, but if a process requires a qualified or advanced
electronic signature, use a dedicated signing service.

## Sending the signed copy

Click **Export signed copy** (or **Export PDF**) and choose where to save. The exported PDF
has your text, check marks and signature as ordinary page content, so recipients can't
move or edit them as annotations. Keep the `.pdfspace` workspace if you might need to
change something later.
