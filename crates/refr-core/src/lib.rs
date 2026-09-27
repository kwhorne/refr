//! Refr's document model. No UI, rendering or platform dependencies, so the same crate can
//! back a native GPUI host today and a WebAssembly host later.

pub mod editor;
pub mod geometry;
pub mod layout;
pub mod model;
pub mod page_range;
pub mod text_layout;
pub mod workspace_json;

pub use editor::{EditError, EditorSession, PdfTool};
pub use geometry::{PointD, RectD};
pub use model::{Annotation, AnnotationKind, CommentReply, PdfPageState, PdfSource, PdfWorkspace};
pub use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pdfspace_json_written_by_system_text_json() {
        // Shape produced by PdfSpace's WorkspaceJson.Save, including computed properties.
        let json = r#"{"FormatVersion":1,"Title":"a.pdf","Author":"","Sources":[{"Id":"6f1c1f57-5d8e-4a57-9d2a-0c1f5c3b6d11","Name":"a.pdf","Bytes":"JVBERi0xLjcK"}],
          "Pages":[{"Id":"0b8f3c4e-2c55-4a1e-8f7e-7b0f1b1f2a33","SourceId":"6f1c1f57-5d8e-4a57-9d2a-0c1f5c3b6d11","SourcePage":1,"Width":595,"Height":842,"Rotation":90,"Crop":null,"Bookmark":"Cover",
          "Annotations":[{"Id":"1d2e3f40-1111-4222-8333-944455556666","Kind":4,"Bounds":{"X":72,"Y":120,"Width":180,"Height":64,"Right":252,"Bottom":184,"Center":{"X":162,"Y":152},"IsFinite":true},
          "Points":[],"Color":4279530470,"StrokeWidth":2,"FontSize":14,"Text":"","Author":"You","Created":"2026-09-27T12:00:00.1234567+00:00","Resolved":false,"Replies":[]}],
          "VisibleBox":{"X":0,"Y":0,"Width":595,"Height":842},"DisplayWidth":842,"DisplayHeight":595}],"AnnotationCount":1}"#;
        let doc = workspace_json::load(json).unwrap();
        assert_eq!(doc.pages[0].rotation, 90);
        assert_eq!(doc.pages[0].annotations[0].kind, AnnotationKind::Rectangle);
        assert_eq!(&doc.sources[0].bytes[..5], b"%PDF-");
        let round = workspace_json::load(&workspace_json::save(&doc)).unwrap();
        assert_eq!(round, doc);
    }

    #[test]
    fn coordinates_round_trip_exactly() {
        let mut doc = PdfWorkspace::default();
        let bounds = RectD::new(99.99999933367737, 450.0000114607498, 99.9999776115584, 49.999988805779196);
        doc.pages[0].annotations.push(Annotation::new(AnnotationKind::Rectangle, bounds));
        let round = workspace_json::load(&workspace_json::save(&doc)).unwrap();
        assert_eq!(round.pages[0].annotations[0].bounds, bounds);
    }
}
