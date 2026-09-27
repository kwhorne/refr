//! Headless interaction tests: a real workbench in GPUI's test window, driven by
//! simulated pointer and keyboard events. Nothing touches the desktop.

use std::sync::OnceLock;

use gpui::{Entity, Modifiers, MouseButton, Point, Pixels, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};
use refr_core::{AnnotationKind, PdfTool, PointD, layout};
use refr_pdf::Engine;

use crate::document::DocumentView;
use crate::workbench::Workbench;

fn engine() -> Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::start(Engine::locate_library().expect("run scripts/fetch-pdfium.sh")).expect("PDFium binds")).clone()
}

fn setup(cx: &mut TestAppContext) -> (Entity<Workbench>, &mut VisualTestContext) {
    let dir = std::env::temp_dir().join(format!("refr-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // SAFETY: every test sets the same value before any storage access.
    unsafe { std::env::set_var("REFR_DATA_DIR", &dir) };
    cx.update(|cx| cx.bind_keys(crate::key_bindings()));
    let engine = engine();
    let (workbench, cx) = cx.add_window_view(|window, cx| Workbench::new(engine, Vec::new(), window, cx));
    cx.run_until_parked();
    (workbench, cx)
}

fn doc(workbench: &Entity<Workbench>, cx: &mut VisualTestContext) -> Entity<DocumentView> {
    workbench.read_with(cx, |w, _| w.doc().cloned().expect("a document is open"))
}

/// Window position of a page-space point on a placed page.
fn at(doc: &Entity<DocumentView>, cx: &mut VisualTestContext, page: usize, p: PointD) -> Point<Pixels> {
    doc.read_with(cx, |d, _| {
        let placement = d.placements.iter().find(|pl| pl.index == page).expect("page is placed");
        let s = placement.to_screen(&d.doc().pages[page], p, d.zoom);
        point(d.viewport.origin.x + px(s.x as f32), d.viewport.origin.y + px(s.y as f32))
    })
}

fn drag(cx: &mut VisualTestContext, from: Point<Pixels>, to: Point<Pixels>) {
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        cx.simulate_mouse_move(point(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t), MouseButton::Left, Modifiers::none());
    }
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

fn use_tool(workbench: &Entity<Workbench>, cx: &mut VisualTestContext, tool: PdfTool) {
    workbench.update_in(cx, |w, window, cx| w.use_tool(tool, window, cx));
    cx.run_until_parked();
}

fn annotations(doc: &Entity<DocumentView>, cx: &mut VisualTestContext, page: usize) -> Vec<refr_core::Annotation> {
    doc.read_with(cx, |d, _| d.doc().pages[page].annotations.clone())
}

#[gpui::test]
fn opens_the_sample_fitted_to_the_window(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    doc.read_with(cx, |d, _| {
        assert_eq!(d.doc().pages.len(), 6);
        assert!(!d.placements.is_empty(), "pages are laid out after the first frame");
        assert!(d.zoom > 0.1 && d.zoom < 8.0);
    });
}

#[gpui::test]
fn highlight_tool_marks_the_dragged_words(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Highlight);
    let words = doc.update(cx, |d, cx| d.words(0, cx));
    cx.run_until_parked();
    let words = words.or_else(|| doc.update(cx, |d, cx| d.words(0, cx))).expect("words load");
    let good = words.iter().find(|w| w.text == "Good").expect("sample has 'Good'").bounds;
    let ideas = words.iter().find(|w| w.text == "ideas.").expect("sample has 'ideas.'").bounds;
    let from = at(&doc, cx, 0, PointD::new(good.x + 2.0, good.y + good.height / 2.0));
    let to = at(&doc, cx, 0, PointD::new(ideas.right() - 2.0, ideas.y + ideas.height / 2.0));
    drag(cx, from, to);
    let marks = annotations(&doc, cx, 0);
    assert_eq!(marks.len(), 1, "one line becomes one highlight: {marks:?}");
    assert_eq!(marks[0].kind, AnnotationKind::Highlight);
    let b = marks[0].bounds;
    assert!(b.x <= good.x + 0.5 && b.right() >= ideas.right() - 0.5, "{b:?}");
}

#[gpui::test]
fn draw_select_move_resize_and_undo(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);

    use_tool(&workbench, cx, PdfTool::Rectangle);
    let (a, b) = (PointD::new(100.0, 450.0), PointD::new(250.0, 550.0));
    let (from, to) = (at(&doc, cx, 0, a), at(&doc, cx, 0, b));
    drag(cx, from, to);
    let rect = annotations(&doc, cx, 0).pop().expect("rectangle added");
    assert_eq!(rect.kind, AnnotationKind::Rectangle);
    assert!((rect.bounds.x - 100.0).abs() < 1.5 && (rect.bounds.width - 150.0).abs() < 1.5, "{:?}", rect.bounds);

    // Move it by dragging from inside.
    use_tool(&workbench, cx, PdfTool::Select);
    let inside = at(&doc, cx, 0, PointD::new(175.0, 500.0));
    let moved_to = at(&doc, cx, 0, PointD::new(215.0, 530.0));
    drag(cx, inside, moved_to);
    let moved = annotations(&doc, cx, 0).pop().unwrap();
    assert!((moved.bounds.x - 140.0).abs() < 1.5 && (moved.bounds.y - 480.0).abs() < 1.5, "{:?}", moved.bounds);
    assert_eq!(doc.read_with(cx, |d, _| d.session.selected_id()), Some(rect.id));

    // Resize from the bottom-right handle (3 px outside the outline).
    let zoom = doc.read_with(cx, |d, _| d.zoom);
    let corner = at(&doc, cx, 0, PointD::new(moved.bounds.right() + 3.0 / zoom, moved.bounds.bottom() + 3.0 / zoom));
    let target = at(&doc, cx, 0, PointD::new(moved.bounds.right() + 50.0, moved.bounds.bottom() + 20.0));
    drag(cx, corner, target);
    let resized = annotations(&doc, cx, 0).pop().unwrap();
    assert!((resized.bounds.width - 200.0).abs() < 2.5 && (resized.bounds.height - 120.0).abs() < 2.5, "{:?}", resized.bounds);

    // Undo resize, move and add.
    for _ in 0..3 {
        cx.simulate_keystrokes("cmd-z");
    }
    cx.run_until_parked();
    assert!(annotations(&doc, cx, 0).is_empty());
    assert!(!doc.read_with(cx, |d, _| d.session.is_dirty()));
}

#[gpui::test]
fn ink_on_a_rotated_page_lands_where_it_was_drawn(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    doc.update(cx, |d, cx| d.edit(cx, |s| s.rotate_page(0, 90)));
    cx.run_until_parked();
    use_tool(&workbench, cx, PdfTool::Ink);
    let (a, b) = (PointD::new(300.0, 200.0), PointD::new(350.0, 260.0));
    let (from, to) = (at(&doc, cx, 0, a), at(&doc, cx, 0, b));
    drag(cx, from, to);
    let ink = annotations(&doc, cx, 0).pop().expect("ink added");
    assert_eq!(ink.kind, AnnotationKind::Ink);
    assert!(ink.points.first().unwrap().distance(a) < 1.5, "{:?}", ink.points.first());
    assert!(ink.points.last().unwrap().distance(b) < 1.5, "{:?}", ink.points.last());
    // And the display transform really is rotated: page x runs down the screen.
    doc.read_with(cx, |d, _| {
        let page = &d.doc().pages[0];
        let p = layout::to_display(page, a);
        assert!((p.x - (page.height - a.y)).abs() < 1e-6 && (p.y - a.x).abs() < 1e-6);
    });
}

#[gpui::test]
fn text_tool_types_an_annotation(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Text);
    let spot = at(&doc, cx, 0, PointD::new(80.0, 600.0));
    cx.simulate_click(spot, Modifiers::none());
    cx.run_until_parked();
    assert!(doc.read_with(cx, |d, _| d.inline_text.is_some()), "inline editor opens");
    cx.simulate_input("Approved by QA");
    let typed = doc.read_with(cx, |d, cx| d.inline_text.as_ref().map(|i| i.input.read(cx).text().to_string()));
    let focused = cx.update(|window, cx| doc.read(cx).inline_text.as_ref().map(|i| i.input.read(cx).is_focused(window)));
    assert_eq!(typed.as_deref(), Some("Approved by QA"), "focused: {focused:?}");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let text = annotations(&doc, cx, 0).pop().expect("text added");
    assert_eq!(text.kind, AnnotationKind::Text);
    assert_eq!(text.text, "Approved by QA");
    assert!((text.bounds.x - 80.0).abs() < 1.5 && (text.bounds.y - 600.0).abs() < 1.5);
}

#[gpui::test]
fn select_tool_copies_dragged_text(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Select);
    doc.update(cx, |d, cx| d.words(0, cx));
    cx.run_until_parked();
    let words = doc.update(cx, |d, cx| d.words(0, cx)).unwrap();
    let a = words.iter().find(|w| w.text == "Lasting").unwrap().bounds;
    let b = words.iter().find(|w| w.text == "impact.").unwrap().bounds;
    let (from, to) = (at(&doc, cx, 0, PointD::new(a.x + 1.0, a.y + a.height / 2.0)), at(&doc, cx, 0, PointD::new(b.right() - 1.0, b.y + b.height / 2.0)));
    drag(cx, from, to);
    cx.simulate_keystrokes("cmd-c");
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("Lasting impact."));
}

#[gpui::test]
fn command_scroll_zooms_around_the_pointer(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    let anchor_page = PointD::new(300.0, 300.0);
    let before = at(&doc, cx, 0, anchor_page);
    let zoom = doc.read_with(cx, |d, _| d.zoom);
    cx.simulate_event(ScrollWheelEvent { position: before, delta: ScrollDelta::Pixels(point(px(0.), px(40.))), modifiers: Modifiers::command(), ..Default::default() });
    cx.run_until_parked();
    let after_zoom = doc.read_with(cx, |d, _| d.zoom);
    assert!(after_zoom > zoom * 1.3, "{zoom} → {after_zoom}");
    let after = at(&doc, cx, 0, anchor_page);
    assert!((after.x - before.x).abs() < px(2.) && (after.y - before.y).abs() < px(2.), "{before:?} → {after:?}");
}

#[gpui::test]
fn workspace_round_trips_through_a_file(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Ellipse);
    let (from, to) = (at(&doc, cx, 0, PointD::new(100.0, 100.0)), at(&doc, cx, 0, PointD::new(200.0, 160.0)));
    drag(cx, from, to);
    let workspace = doc.read_with(cx, |d, _| d.doc().clone());
    let path = std::env::temp_dir().join(format!("refr-roundtrip-{}.pdfspace", std::process::id()));
    crate::storage::write_atomic(&path, refr_core::workspace_json::save(&workspace).as_bytes()).unwrap();
    let loaded = crate::storage::load(&engine(), &path).unwrap();
    assert_eq!(loaded, *workspace);
    std::fs::remove_file(path).ok();
}

#[gpui::test]
fn note_tool_asks_for_the_comment_text(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Note);
    let spot = at(&doc, cx, 0, PointD::new(400.0, 300.0));
    cx.simulate_click(spot, Modifiers::none());
    cx.run_until_parked();
    assert!(workbench.read_with(cx, |w, _| w.dialog.is_some()), "comment dialog opens");
    cx.simulate_input("Check these figures");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let note = annotations(&doc, cx, 0).pop().expect("note added");
    assert_eq!(note.kind, AnnotationKind::Note);
    assert_eq!(note.text, "Check these figures");
    assert!(note.bounds.contains(PointD::new(400.0, 300.0)));
    assert_eq!(workbench.read_with(cx, |w, _| w.right), Some(crate::workbench::Panel::Comments));
}

#[gpui::test]
fn crop_tool_crops_the_page(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Crop);
    let (from, to) = (at(&doc, cx, 0, PointD::new(48.0, 331.0)), at(&doc, cx, 0, PointD::new(547.0, 663.0)));
    drag(cx, from, to);
    let crop = doc.read_with(cx, |d, _| d.doc().pages[0].crop).expect("page cropped");
    assert!((crop.x - 48.0).abs() < 1.5 && (crop.width - 499.0).abs() < 2.0, "{crop:?}");
    doc.read_with(cx, |d, _| assert!((d.doc().pages[0].display_height() - crop.height).abs() < 1e-6));
}

#[gpui::test]
fn keyboard_shortcuts_reach_the_viewport(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);
    let doc = doc(&workbench, cx);
    use_tool(&workbench, cx, PdfTool::Rectangle);
    let (from, to) = (at(&doc, cx, 0, PointD::new(100.0, 450.0)), at(&doc, cx, 0, PointD::new(200.0, 500.0)));
    drag(cx, from, to);
    assert_eq!(annotations(&doc, cx, 0).len(), 1);
    // The new annotation is selected; Delete removes it.
    cx.simulate_keystrokes("delete");
    cx.run_until_parked();
    assert!(annotations(&doc, cx, 0).is_empty());
    cx.simulate_keystrokes("d");
    assert_eq!(doc.read_with(cx, |d, _| d.session.tool()), PdfTool::Ink);
    cx.simulate_keystrokes("h");
    assert_eq!(doc.read_with(cx, |d, _| d.session.tool()), PdfTool::Hand);
    cx.simulate_keystrokes("pagedown");
    cx.run_until_parked();
    assert_eq!(doc.read_with(cx, |d, _| d.session.current_page()), 1);
}
