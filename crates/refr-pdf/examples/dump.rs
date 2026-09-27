//! Writes sample-page PNGs to the given directory, for eyeballing rendering and export.
use std::sync::Arc;

use refr_core::{Annotation, AnnotationKind, EditorSession, PointD, RectD};
use refr_pdf::Engine;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    let engine = Engine::start(Engine::locate_library().expect("libpdfium")).unwrap();
    let doc = engine.sample().unwrap();
    let mut s = EditorSession::new(doc).unwrap();
    let add = |s: &mut EditorSession, a: Annotation| s.add_annotation(a, Some(0)).unwrap();
    add(&mut s, Annotation::new(AnnotationKind::Highlight, RectD::new(46.0, 133.0, 230.0, 48.0)));
    add(&mut s, Annotation { color: 0xFFD93830, ..Annotation::new(AnnotationKind::Ellipse, RectD::new(40.0, 340.0, 180.0, 130.0)) });
    add(&mut s, Annotation { points: vec![PointD::new(300.0, 200.0), PointD::new(420.0, 320.0)], ..Annotation::new(AnnotationKind::Arrow, RectD::new(300.0, 200.0, 120.0, 120.0)) });
    add(&mut s, Annotation { text: "Looks good — ship it".into(), font_size: 16.0, ..Annotation::new(AnnotationKind::Text, RectD::new(300.0, 470.0, 200.0, 40.0)) });
    add(&mut s, Annotation { color: 0xFF2D9D78, ..Annotation::new(AnnotationKind::Stamp, RectD::new(380.0, 90.0, 150.0, 40.0)) });
    add(&mut s, Annotation { text: "Comment".into(), color: 0xFFF5A623, ..Annotation::new(AnnotationKind::Note, RectD::new(520.0, 250.0, 23.0, 23.0)) });
    add(&mut s, Annotation::new(AnnotationKind::Check, RectD::new(60.0, 700.0, 30.0, 24.0)));
    let doc = s.document().clone();
    let raw = engine.render(&doc, 0, 1.0).unwrap().unwrap();
    std::fs::write(out.join("sample-p1.png"), raw.to_png().unwrap()).unwrap();
    std::fs::write(out.join("export-p2.png"), engine.export_png(&doc, 1, 1.0).unwrap()).unwrap();
    std::fs::write(out.join("review.pdfspace"), refr_core::workspace_json::save(&doc)).unwrap();
    std::fs::write(out.join("export.pdf"), engine.export_pdf(&Arc::clone(&doc), None).unwrap()).unwrap();
}
