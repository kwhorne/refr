# The .pdfspace format

A `.pdfspace` file is a UTF-8 JSON document that holds the original PDF files and every
edit made on top of them. Refr's format matches the one PdfSpace writes with
`System.Text.Json`, so a workspace moves between the two apps in either direction.

The model is in `crates/refr-core/src/model.rs`, and serialization and validation are in
`crates/refr-core/src/workspace_json.rs`.

## Conventions

- Property names are **PascalCase**.
- Enums are **numbers**.
- Source PDF bytes are **base64** (standard alphabet, with padding).
- Identifiers are UUID strings.
- Dates are RFC 3339 / ISO 8601 with an offset, for example
  `2026-09-27T12:00:00.1234567+00:00`.
- Geometry is in **PDF points** (1/72 inch), in the page's logical coordinate system: origin
  at the **top left**, y growing **downwards**. See
  [Coordinate spaces](architecture.md#coordinate-spaces).
- Colors are **ARGB** packed in a 32-bit unsigned integer. `4279530470` is `0xFF1473E6`,
  opaque blue.
- When reading, Refr ignores unknown properties, including the computed ones PdfSpace
  writes (`Right`, `Bottom`, `Center`, `IsFinite`, `VisibleBox`, `DisplayWidth`,
  `DisplayHeight` and `AnnotationCount`). Missing properties get defaults, except where
  noted.

## Example

```json
{
  "FormatVersion": 1,
  "Title": "Report.pdf",
  "Author": "",
  "Sources": [
    { "Id": "6f1c1f57-5d8e-4a57-9d2a-0c1f5c3b6d11", "Name": "Report.pdf", "Bytes": "JVBERi0xLjcK…" }
  ],
  "Pages": [
    {
      "Id": "0b8f3c4e-2c55-4a1e-8f7e-7b0f1b1f2a33",
      "SourceId": "6f1c1f57-5d8e-4a57-9d2a-0c1f5c3b6d11",
      "SourcePage": 1,
      "Width": 595, "Height": 842,
      "Rotation": 90,
      "Crop": null,
      "Bookmark": "Cover",
      "Annotations": [
        {
          "Id": "1d2e3f40-1111-4222-8333-944455556666",
          "Kind": 4,
          "Bounds": { "X": 72, "Y": 120, "Width": 180, "Height": 64 },
          "Points": [],
          "Color": 4279530470,
          "StrokeWidth": 2,
          "FontSize": 14,
          "Text": "",
          "Author": "You",
          "Created": "2026-09-27T12:00:00+00:00",
          "Resolved": false,
          "Replies": []
        }
      ]
    }
  ]
}
```

## Workspace

| Property | Type | Meaning |
|---|---|---|
| `FormatVersion` | number | Always `1` |
| `Title` | string | Shown on the tab and used for suggested file names |
| `Author` | string | Document author, stored only in the workspace |
| `Sources` | array of Source | The original PDFs |
| `Pages` | array of Page | Pages in display order |

## Source

| Property | Type | Meaning |
|---|---|---|
| `Id` | UUID | Referenced by `Page.SourceId` |
| `Name` | string | The original file name |
| `Bytes` | base64 string | The complete, unmodified PDF file (required) |

Sources are immutable. Refr never changes their bytes, and pages only point into them.

## Page

| Property | Type | Default | Meaning |
|---|---|---|---|
| `Id` | UUID | new | Page identity |
| `SourceId` | UUID or `null` | `null` | The source this page comes from. `null` is a blank page. |
| `SourcePage` | number | `1` | **One-based** page number in the source |
| `Width`, `Height` | number | `595`, `842` | Logical page size in points: the page's effective box with its own `/Rotate` applied |
| `Rotation` | number | `0` | Extra workspace rotation, clockwise: `0`, `90`, `180` or `270` |
| `Crop` | Rect or `null` | `null` | Visible area in logical coordinates. `null` shows the whole page. |
| `Bookmark` | string | `""` | Workspace bookmark name. Empty means none. |
| `Annotations` | array of Annotation | `[]` | Marks on this page, bottom to top |

The same source page can appear more than once, for example after **Duplicate page**.

## Annotation

| Property | Type | Default | Meaning |
|---|---|---|---|
| `Id` | UUID | new | Identity. Unique in the whole workspace. |
| `Kind` | number | `0` | See the table below |
| `Bounds` | Rect | zero | Bounding box in logical page coordinates |
| `Points` | array of Point | `[]` | Path points for ink, signatures, lines and arrows |
| `Color` | ARGB number | `0xFF1473E6` | Stroke, fill or text color |
| `StrokeWidth` | number | `2` | Line width in points |
| `FontSize` | number | `14` | Text size in points |
| `Text` | string | `""` | Text, comment body or stamp wording |
| `Author` | string | `"You"` | Who made it |
| `Created` | date | now | When it was made |
| `Resolved` | boolean | `false` | Whether the comment thread is resolved |
| `Replies` | array of Reply | `[]` | Comment replies |

### Kinds

| Value | Kind | Geometry |
|---|---|---|
| 0 | Highlight | `Bounds` covers one line of text |
| 1 | Underline | `Bounds` covers one line of text; the line is drawn at the bottom |
| 2 | Strikeout | `Bounds` covers one line of text; the line is drawn through the middle |
| 3 | Ink | `Points` is the stroke; `Bounds` encloses it |
| 4 | Rectangle | `Bounds` |
| 5 | Ellipse | Inscribed in `Bounds` |
| 6 | Line | `Points[0]` to `Points[1]` |
| 7 | Arrow | `Points[0]` (tail) to `Points[1]` (head) |
| 8 | Text | Top-left of `Bounds`; `Text` wraps to `Bounds.Width` |
| 9 | Note (comment) | A note icon in `Bounds`; `Text` is the comment |
| 10 | Signature | Like Ink |
| 11 | Stamp | Outlined `Bounds` with `Text` (default "APPROVED") |
| 12 | Check | A check mark filling `Bounds` |

### Rect, Point and Reply

| Type | Properties |
|---|---|
| Rect | `X`, `Y`, `Width`, `Height` (numbers) |
| Point | `X`, `Y` (numbers) |
| Reply | `Id` (UUID), `Author`, `Text` (strings), `Created` (date) |

## Text layout

Text and stamps wrap using Helvetica's standard (AFM) character widths at `FontSize`, with a
line height of 1.35 × `FontSize` (`crates/refr-core/src/text_layout.rs`). Refr lays text out the
same way in the viewer and in exported PDFs, so a workspace looks the same in both.

## Validation

Refr validates every workspace when it opens it, and every change before it's applied
(`workspace_json::validate`). A workspace that fails any rule isn't opened.

| Rule | Limit |
|---|---|
| Whole file | At most 128 MB of JSON |
| `FormatVersion` | Exactly 1 |
| `Title`, `Author` | At most 1,024 and 4,096 bytes |
| Pages | 1–4,096 |
| Sources | At most 4,096. Unique IDs. Names up to 1,024 bytes. Each must start with `%PDF-` in its first kilobyte. |
| Combined source bytes | At most 64 MB |
| Page IDs | Unique |
| Page size | Finite, greater than 0, at most 100,000 pt |
| Rotation | 0, 90, 180 or 270 |
| `SourceId` | `null` or the ID of an existing source |
| `SourcePage` | 1–4,096 |
| `Bookmark` | At most 4,096 bytes |
| `Crop` | Finite, at least 1 × 1 pt, inside the page |
| Annotations | At most 50,000 in the workspace, with unique IDs |
| Annotation geometry | Finite bounds with non-negative size. Stroke width over 0 and up to 100. Font size 1–1,000. |
| Annotation text and author | At most 100,000 and 4,096 bytes |
| Path points | At most 100,000 per annotation and 2,000,000 in the workspace, all finite |
| Replies | At most 1,000 per annotation, with authors up to 4,096 and text up to 100,000 bytes |

## Composition

- **Combining** (`workspace_json::append`) gives the incoming sources, pages, annotations and
  replies new IDs before appending them. You can combine the same workspace with itself.
- **Extracting** (`workspace_json::extract`) copies the chosen pages with new IDs and keeps only
  the sources those pages use.

## Security considerations

- A workspace contains complete PDFs. Treat it with the same confidentiality as the original.
- Refr treats workspace contents as untrusted input. The limits above bound memory use, and
  PDF bytes are only ever parsed by PDFium.
