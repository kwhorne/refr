//! Writes showcase.pdfspace: the sample report with a tidy set of review marks, used for
//! the README screenshot.
//!
//!   cargo run -p refr-pdf --example showcase -- <output directory>

use refr_core::{Annotation, AnnotationKind, CommentReply, EditorSession, PointD, RectD, Uuid};
use refr_pdf::Engine;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    let engine = Engine::start(Engine::locate_library().expect("libpdfium")).unwrap();
    let doc = std::sync::Arc::new(engine.sample().unwrap());
    let words = engine.words(&doc, 0).unwrap();
    let find = |text: &str| words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("{text}")).bounds;
    let span = |from: &str, to: &str| RectD::union(find(from), find(to));

    let mut s = EditorSession::new((*doc).clone()).unwrap();
    let mut add = |a: Annotation| s.add_annotation(a, Some(0)).unwrap();
    // Highlight the headline's first line and underline the subtitle.
    add(Annotation { color: 0xFFFFCA28, ..Annotation::new(AnnotationKind::Highlight, span("Good", "ideas.").inflate(1.0)) });
    let subtitle = span("A", "tomorrow.");
    add(Annotation { color: 0xFF1473E6, stroke_width: 1.5, ..Annotation::new(AnnotationKind::Underline, subtitle) });
    // A comment with a reply, next to the headline.
    let now = chrono::Utc::now();
    add(Annotation {
        color: 0xFFE19815,
        text: "Can we make the headline promise more concrete?".into(),
        author: "Ingrid".into(),
        replies: vec![CommentReply { id: Uuid::new_v4(), author: "You".into(), text: "Agreed. I'll draft two options.".into(), created: now }],
        ..Annotation::new(AnnotationKind::Note, RectD::new(520.0, 150.0, 23.0, 23.0))
    });
    // Circle the chart, point at it, and add a note in red.
    add(Annotation { color: 0xFFD93830, stroke_width: 2.5, ..Annotation::new(AnnotationKind::Ellipse, RectD::new(212.0, 404.0, 180.0, 180.0)) });
    add(Annotation {
        color: 0xFFD93830,
        stroke_width: 2.0,
        points: vec![PointD::new(440.0, 298.0), PointD::new(372.0, 414.0)],
        ..Annotation::new(AnnotationKind::Arrow, RectD::between(PointD::new(440.0, 298.0), PointD::new(372.0, 414.0)))
    });
    add(Annotation { color: 0xFFD93830, font_size: 13.0, text: "Use the new colours".into(), ..Annotation::new(AnnotationKind::Text, RectD::new(400.0, 276.0, 140.0, 18.0)) });
    // Approve it.
    add(Annotation { color: 0xFF29834B, stroke_width: 2.0, font_size: 16.0, text: "APPROVED".into(), ..Annotation::new(AnnotationKind::Stamp, RectD::new(430.0, 690.0, 118.0, 34.0)) });
    std::fs::write(out.join("showcase.pdfspace"), refr_core::workspace_json::save(s.document())).unwrap();
}

