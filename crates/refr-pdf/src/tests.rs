use std::sync::{Arc, OnceLock};

use refr_core::{Annotation, AnnotationKind, EditorSession, PdfWorkspace, RectD};

use crate::Engine;

/// PDFium binds once per process, so all tests share one engine.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let library = Engine::locate_library().expect("run scripts/fetch-pdfium.sh first");
        Engine::start(library).expect("PDFium binds")
    })
}

fn sample() -> Arc<PdfWorkspace> {
    Arc::new(engine().sample().unwrap())
}

#[test]
fn sample_opens_with_six_bookmarked_pages() {
    let doc = sample();
    assert_eq!(doc.pages.len(), 6);
    assert_eq!(doc.pages[0].bookmark, "Cover");
    assert!((doc.pages[0].width - 595.0).abs() < 0.5 && (doc.pages[0].height - 842.0).abs() < 0.5);
}

#[test]
fn words_are_in_top_left_logical_space() {
    let doc = sample();
    let words = engine().words(&doc, 0).unwrap();
    let brand = words.iter().find(|w| w.text == "FIELDWORK").expect("header word");
    // Drawn with its top edge at y = 34 and left edge at x = 48.
    assert!((brand.bounds.x - 48.0).abs() < 3.0, "{:?}", brand.bounds);
    assert!(brand.bounds.y > 25.0 && brand.bounds.y < 45.0, "{:?}", brand.bounds);
}

#[test]
fn search_finds_page_text_and_annotation_text() {
    let doc = sample();
    let mut session = EditorSession::new((*doc).clone()).unwrap();
    session
        .add_annotation(Annotation { text: "Check the recovery rates".into(), ..Annotation::new(AnnotationKind::Note, RectD::new(10.0, 10.0, 23.0, 23.0)) }, Some(4))
        .unwrap();
    let doc = session.document().clone();
    let results = engine().find(&doc, "less waste", false).unwrap();
    assert!(results.iter().any(|r| r.page_index == 1 && r.annotation.is_none()), "{results:?}");
    let results = engine().find(&doc, "recovery rates", false).unwrap();
    assert!(results.iter().any(|r| r.annotation.is_some()));
}

#[test]
fn render_applies_rotation_and_crop() {
    let doc = sample();
    let bitmap = engine().render(&doc, 0, 1.0).unwrap().unwrap();
    assert_eq!((bitmap.width, bitmap.height), (595, 842));
    // White paper in the margin.
    assert_eq!(&bitmap.bgra[0..3], &[255, 255, 255]);

    let mut session = EditorSession::new((*doc).clone()).unwrap();
    session.rotate_page(0, 90).unwrap();
    session.crop_page(0, Some(RectD::new(48.0, 331.0, 499.0, 332.0))).unwrap();
    let doc = session.document().clone();
    let bitmap = engine().render(&doc, 0, 2.0).unwrap().unwrap();
    assert_eq!((bitmap.width, bitmap.height), (664, 998));
    // The crop is the green panel, so the corner is no longer white.
    assert_ne!(&bitmap.bgra[0..3], &[255, 255, 255]);
}

#[test]
fn blank_pages_have_no_bitmap() {
    let doc = Arc::new(PdfWorkspace::default());
    assert!(engine().render(&doc, 0, 1.0).unwrap().is_none());
}

#[test]
fn export_keeps_text_searchable_and_applies_edits() {
    let doc = sample();
    let mut session = EditorSession::new((*doc).clone()).unwrap();
    session.rotate_page(1, 90).unwrap();
    session.add_annotation(Annotation { text: "Reviewed".into(), ..Annotation::new(AnnotationKind::Text, RectD::new(60.0, 60.0, 200.0, 30.0)) }, Some(1)).unwrap();
    session.add_annotation(Annotation::new(AnnotationKind::Highlight, RectD::new(48.0, 130.0, 200.0, 40.0)), Some(1)).unwrap();
    session.add_annotation(Annotation { text: "A comment".into(), ..Annotation::new(AnnotationKind::Note, RectD::new(300.0, 300.0, 23.0, 23.0)) }, Some(1)).unwrap();
    session.insert_blank(2).unwrap();
    let doc = session.document().clone();
    let bytes = engine().export_pdf(&doc, Some(vec![1, 2])).unwrap();
    let exported = Arc::new(engine().open(bytes, "export.pdf").unwrap());
    assert_eq!(exported.pages.len(), 2);
    // Rotated page: logical size swaps.
    assert!((exported.pages[0].width - 842.0).abs() < 0.5, "{}", exported.pages[0].width);
    let text = engine().extract_text(&exported).unwrap();
    assert!(text.contains("Less waste."), "{text}");
    assert!(text.contains("Reviewed"), "{text}");
    // The added text sits where it was placed, mapped through the rotation.
    let words = engine().words(&exported, 0).unwrap();
    let reviewed = words.iter().find(|w| w.text == "Reviewed").expect("added text");
    let mut rotated = doc.pages[1].clone();
    rotated.annotations.clear();
    let expected = refr_core::layout::display_bounds(&rotated, RectD::new(60.0, 60.0, 50.0, 14.0));
    assert!((reviewed.bounds.x - expected.x).abs() < 20.0 && (reviewed.bounds.y - expected.y).abs() < 60.0, "{:?} vs {:?}", reviewed.bounds, expected);
}

#[test]
fn png_and_split_exports() {
    let doc = sample();
    let png = engine().export_png(&doc, 0, 0.5).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let zip = engine().split(&doc).unwrap();
    assert_eq!(&zip[0..2], b"PK");
}
