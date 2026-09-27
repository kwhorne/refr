//! Rendering of the workbench chrome: title bar and tabs, global bar, tool panel,
//! navigation rail, floating tools, status bar, home view and dialog.

use gpui::{
    AnyElement, Context, Div, ExternalPaths, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
    Stateful, Styled, Window, div, prelude::*, px, rgb, rgba,
};
use refr_core::layout::PageLayoutMode;
use refr_core::{AnnotationKind, PdfTool};

use crate::actions::*;
use crate::document::Fit;
use crate::theme::{self, button, description, divider_h, divider_v, heading, icon};
use crate::workbench::{Mode, Panel, Workbench, file_name};

const TRAFFIC_LIGHTS: f32 = 78.;

impl Workbench {
    fn title_bar(&mut self, width: f32, cx: &mut Context<Self>) -> Div {
        let tab_width = if width >= 1000. { 222. } else { 155. };
        let mut tabs = div().id("tabs").flex().flex_none().items_end().gap(px(3.)).h_full().pt(px(8.)).overflow_x_scroll();
        for (index, doc) in self.documents.iter().enumerate() {
            let d = doc.read(cx);
            let active = index == self.active && !self.home;
            let title = format!("{}{}", if d.session.is_dirty() { "• " } else { "" }, d.title());
            tabs = tabs.child(
                div()
                    .id(("tab", index))
                    .flex()
                    .flex_none()
                    .items_center()
                    .w(px(tab_width))
                    .h(px(38.))
                    .pl(px(10.))
                    .pr(px(4.))
                    .gap(px(8.))
                    .rounded_t(px(7.))
                    .border_1()
                    .border_b_0()
                    .border_color(rgb(0xD9D9D9))
                    .bg(rgb(if active { 0xFFFFFF } else { 0xEDEDED }))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .on_click(cx.listener(move |this, _, window, cx| this.activate(index, window, cx)))
                    .child(icon("file").text_color(rgb(theme::BRAND)))
                    .child(div().flex_1().truncate().child(title))
                    .child(
                        button(("close-tab", index))
                            .icon("close")
                            .tooltip(format!("Close {}", d.title()))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.close_document(index, window, cx)
                            }))
                            .build()
                            .w(px(26.))
                            .h(px(28.)),
                    ),
            );
        }
        let drag = || {
            div().flex_1().h_full().on_mouse_down(MouseButton::Left, |event, window, _| {
                if event.click_count == 2 {
                    window.titlebar_double_click();
                } else {
                    window.start_window_move();
                }
            })
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(46.))
            .pl(px(TRAFFIC_LIGHTS))
            .bg(rgb(theme::TITLE_BAR))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.))
                    .mr(px(10.))
                    .child(icon("file").size(px(24.)).text_color(rgb(theme::BRAND)))
                    .child(
                        div()
                            .id("brand")
                            .px(px(4.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgba(0x0000000D)))
                            .tooltip(theme::tooltip("About Refr"))
                            .on_click(cx.listener(|this, _, window, cx| this.show_about(window, cx)))
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x383838))
                            .child("Refr"),
                    )
                    .child(button("home").icon("home").tooltip("Home").selected(self.home).on_click(cx.listener(|this, _, _, cx| {
                        this.home = !this.home;
                        cx.notify();
                    }))),
            )
            .child(tabs)
            .child(drag())
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.))
                    .px(px(12.))
                    .child(button("open").icon("plus").tooltip("Open PDF (⌘O)").on_click(cx.listener(|this, _, window, cx| this.open_dialog(false, window, cx))))
                    .child(button("help").icon("help").tooltip("Help and shortcuts").on_click(cx.listener(|this, _, window, cx| this.show_help(window, cx)))),
            )
    }

    fn global_bar(&mut self, width: f32, cx: &mut Context<Self>) -> Div {
        let (can_undo, can_redo, undo_label, redo_label) = self
            .doc()
            .map(|d| {
                let s = &d.read(cx).session;
                (s.can_undo(), s.can_redo(), s.undo_label().map(|l| format!("Undo {l}")), s.redo_label().map(|l| format!("Redo {l}")))
            })
            .unwrap_or_default();
        let mut modes = div().flex().items_end().h_full().gap(px(2.));
        for mode in [Mode::AllTools, Mode::Edit, Mode::Convert, Mode::ESign] {
            let active = self.mode == mode && !self.home;
            modes = modes.child(
                div()
                    .id(mode.title())
                    .flex()
                    .items_center()
                    .h(px(49.))
                    .px(px(14.))
                    .border_b(px(3.))
                    .border_color(if active { rgb(theme::ACCENT) } else { rgba(0) })
                    .text_size(px(13.))
                    .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if mode == Mode::AllTools && this.mode == mode && this.left_open {
                            this.left_open = false;
                            cx.notify();
                        } else {
                            this.set_mode(mode, cx);
                        }
                    }))
                    .child(mode.title()),
            );
        }
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(52.))
            .pl(px(16.))
            .pr(px(14.))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(modes)
            .child(div().flex_1())
            .when(width >= 1100., |d| {
                d.child(
                    theme::field()
                        .w(px(200.))
                        .mr(px(10.))
                        .gap(px(6.))
                        .child(icon("search").size(px(15.)).text_color(rgb(theme::FAINT)))
                        .child(div().flex_1().overflow_hidden().child(self.find_input.clone())),
                )
            })
            .child(button("undo").icon("undo").tooltip(undo_label.unwrap_or_else(|| "Undo".into())).disabled(!can_undo).on_click(cx.listener(|this, _, _, cx| {
                this.with_doc(cx, |d, cx| d.undo(cx));
            })))
            .child(button("redo").icon("redo").tooltip(redo_label.unwrap_or_else(|| "Redo".into())).disabled(!can_redo).on_click(cx.listener(|this, _, _, cx| {
                this.with_doc(cx, |d, cx| d.redo(cx));
            })))
            .child(divider_v())
            .child(button("save").icon("save").tooltip("Save editable workspace (⌘S)").on_click(cx.listener(|this, _, window, cx| this.save_workspace(false, window, cx))))
            .child(button("print").icon("print").tooltip("Print (⌘P)").on_click(cx.listener(|this, _, _, cx| this.print(cx))))
            .child(div().w(px(8.)))
            .child(button("export").icon("share").label("Export PDF").primary().on_click(cx.listener(|this, _, window, cx| this.export_pdf(None, window, cx))))
    }

    fn tool_panel(&mut self, width: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let tool = self.doc().map(|d| d.read(cx).session.tool());
        let item = |id: &'static str, label: &'static str, icon_name: &'static str, color: Option<u32>| {
            let mut b = button(id).icon(icon_name).label(label).full_width();
            if let Some(color) = color {
                b = b.icon_color(color);
            }
            b
        };
        let tool_item = |id: &'static str, label: &'static str, icon_name: &'static str, t: PdfTool, cx: &mut Context<Self>| {
            button(id).icon(icon_name).label(label).full_width().selected(tool == Some(t)).on_click(cx.listener(move |this, _, window, cx| this.use_tool(t, window, cx))).build().h(px(38.))
        };
        let mut items = div().flex().flex_col().gap(px(3.)).px(px(14.)).pb(px(20.));
        match self.mode {
            Mode::AllTools => {
                items = items
                    .child(item("t-edit", "Edit a PDF", "edit", Some(0xFFD93830)).on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Edit, cx))).build().h(px(40.)))
                    .child(item("t-export", "Export a PDF", "export", Some(0xFF278748)).on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Convert, cx))).build().h(px(40.)))
                    .child(item("t-organize", "Organize pages", "pages", Some(0xFF9063C9)).on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Organize, cx))).build().h(px(40.)))
                    .child(item("t-comment", "Add comments", "comment", Some(0xFFE19815)).on_click(cx.listener(|this, _, window, cx| {
                        this.right = Some(Panel::Comments);
                        this.use_tool(PdfTool::Note, window, cx);
                    })).build().h(px(40.)))
                    .child(item("t-sign", "Fill & Sign", "sign", Some(0xFF9257C5)).on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::ESign, cx))).build().h(px(40.)))
                    .child(item("t-create", "Create a PDF", "file", Some(0xFFD93830)).on_click(cx.listener(|this, _, window, cx| this.new_blank(window, cx))).build().h(px(40.)))
                    .child(item("t-combine", "Combine files", "copy", Some(0xFF7854BD)).on_click(cx.listener(|this, _, window, cx| this.open_dialog(true, window, cx))).build().h(px(40.)))
                    .child(item("t-crop", "Crop pages", "crop", Some(0xFF6D59B1)).on_click(cx.listener(|this, _, window, cx| {
                        this.set_mode(Mode::Edit, cx);
                        this.use_tool(PdfTool::Crop, window, cx);
                    })).build().h(px(40.)))
                    .child(divider_h())
                    .child(item("t-save", "Save editable workspace", "save", None).on_click(cx.listener(|this, _, window, cx| this.save_workspace(false, window, cx))).build().h(px(40.)))
                    .child(item("t-find", "Find in document", "search", None).on_click(cx.listener(|this, _, window, cx| {
                        this.right = Some(Panel::Find);
                        this.find_input.read(cx).focus(window);
                        cx.notify();
                    })).build().h(px(40.)))
                    .child(heading("NOT AVAILABLE"))
                    .child(item("t-ocr", "Scan & OCR", "image", None).disabled(true).tooltip("Not supported. No simulated operation is performed.").build().h(px(40.)))
                    .child(item("t-protect", "Protect a PDF", "lock", None).disabled(true).tooltip("Not supported. No simulated security operation is performed.").build().h(px(40.)))
                    .child(item("t-redact", "Redact a PDF", "redact", None).disabled(true).tooltip("Not supported. Cropping and covering are not redaction.").build().h(px(40.)))
                    .child(description("Local-first PDF tools. No account, upload or subscription."));
            }
            Mode::Edit => {
                let (color, stroke, font) = self.doc().map(|d| { let s = &d.read(cx).session; (s.color, s.stroke_width, s.font_size) }).unwrap_or((0, 2.0, 14.0));
                items = items
                    .child(description("Add text, shapes and annotations. The original PDF content is kept intact."))
                    .child(tool_item("e-select", "Select annotation", "select", PdfTool::Select, cx))
                    .child(tool_item("e-text", "Add text", "text", PdfTool::Text, cx))
                    .child(heading("MARK UP TEXT"))
                    .child(tool_item("e-highlight", "Highlight text", "highlight", PdfTool::Highlight, cx))
                    .child(tool_item("e-underline", "Underline text", "underline", PdfTool::Underline, cx))
                    .child(tool_item("e-strike", "Strikethrough text", "strikeout", PdfTool::Strikeout, cx))
                    .child(heading("DRAWING TOOLS"))
                    .child(tool_item("e-ink", "Draw freehand", "pen", PdfTool::Ink, cx))
                    .child(tool_item("e-rect", "Rectangle", "rectangle", PdfTool::Rectangle, cx))
                    .child(tool_item("e-ellipse", "Ellipse", "ellipse", PdfTool::Ellipse, cx))
                    .child(tool_item("e-line", "Line", "line", PdfTool::Line, cx))
                    .child(tool_item("e-arrow", "Arrow", "arrow", PdfTool::Arrow, cx))
                    .child(tool_item("e-note", "Add comment", "comment", PdfTool::Note, cx))
                    .child(tool_item("e-stamp", "Add stamp", "check", PdfTool::Stamp, cx))
                    .child(heading("APPEARANCE"))
                    .child(self.palette(color, cx))
                    .child(self.stepper("Font size", font, 1.0, cx, |d, v| d.session.font_size = v.clamp(4.0, 200.0)))
                    .child(self.stepper("Stroke width", stroke, 0.5, cx, |d, v| d.session.stroke_width = v.clamp(0.5, 30.0)))
                    .child(heading("PAGE CONTENT"))
                    .child(tool_item("e-crop", "Crop page", "crop", PdfTool::Crop, cx))
                    .child(item("e-reset", "Reset page crop", "fit_page", None).on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| {
                            let index = d.session.current_page();
                            d.edit(cx, |s| s.crop_page(index, None));
                        });
                    })).build().h(px(38.)))
                    .child(item("e-watermark", "Add watermark", "text", None).on_click(cx.listener(|this, _, window, cx| this.add_watermark(window, cx))).build().h(px(38.)))
                    .child(item("e-numbers", "Add page numbers", "pages", None).on_click(cx.listener(|this, _, _, cx| this.add_page_numbers(cx))).build().h(px(38.)));
            }
            Mode::Convert => {
                items = items
                    .child(description("Choose an output format. PDF export copies the original pages and adds your annotations as page content."))
                    .child(item("c-pdf", "PDF document", "file", Some(0xFFD93830)).on_click(cx.listener(|this, _, window, cx| this.export_pdf(None, window, cx))).build().h(px(40.)))
                    .child(item("c-png", "PNG image · current page", "image", Some(0xFF29834B)).on_click(cx.listener(|this, _, window, cx| this.export_png(window, cx))).build().h(px(40.)))
                    .child(item("c-text", "Plain text", "text", Some(0xFF1473E6)).on_click(cx.listener(|this, _, window, cx| this.export_text(window, cx))).build().h(px(40.)))
                    .child(item("c-workspace", "Editable workspace", "save", Some(0xFF9254CC)).on_click(cx.listener(|this, _, window, cx| this.save_workspace(true, window, cx))).build().h(px(40.)))
                    .child(item("c-split", "Split into single-page PDFs", "pages", Some(0xFF9254CC)).on_click(cx.listener(|this, _, window, cx| this.split(window, cx))).build().h(px(40.)))
                    .child(divider_h())
                    .child(item("c-extract", "Extract selected pages", "export", None).on_click(cx.listener(|this, _, window, cx| this.extract_pages(window, cx))).build().h(px(40.)))
                    .child(description("Original PDF files are never overwritten. Keep a .pdfspace workspace to retain editable notes, drawings and page organization."));
            }
            Mode::ESign => {
                items = items
                    .child(description("Fill a document with text and check marks, then draw your signature."))
                    .child(tool_item("s-text", "Add text", "text", PdfTool::Text, cx))
                    .child(tool_item("s-check", "Add check mark", "check", PdfTool::Check, cx))
                    .child(tool_item("s-sign", "Draw signature", "sign", PdfTool::Signature, cx))
                    .child(item("s-initials", "Add initials", "text", None).on_click(cx.listener(|this, _, window, cx| this.add_initials(window, cx))).build().h(px(38.)))
                    .child(tool_item("s-stamp", "Add approval stamp", "check", PdfTool::Stamp, cx))
                    .child(divider_h())
                    .child(item("s-export", "Export signed copy", "export", None).on_click(cx.listener(|this, _, window, cx| this.export_pdf(None, window, cx))).build().h(px(38.)))
                    .child(description("Signatures here are visual marks. Refr does not create or validate certificate-based digital signatures, identity verification or audit trails."));
            }
            Mode::Organize => {
                items = items
                    .child(description("Select a page. Drag a thumbnail onto another position to reorder pages."))
                    .child(item("o-rotate", "Rotate clockwise", "rotate", None).on_click(cx.listener(|this, _, _, cx| this.rotate_current(90, cx))).build().h(px(38.)))
                    .child(item("o-rotate-back", "Rotate counterclockwise", "rotate", None).on_click(cx.listener(|this, _, _, cx| this.rotate_current(-90, cx))).build().h(px(38.)))
                    .child(item("o-earlier", "Move page earlier", "up", None).on_click(cx.listener(|this, _, _, cx| this.move_current(-1, cx))).build().h(px(38.)))
                    .child(item("o-later", "Move page later", "down", None).on_click(cx.listener(|this, _, _, cx| this.move_current(1, cx))).build().h(px(38.)))
                    .child(item("o-duplicate", "Duplicate page", "copy", None).on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| {
                            let i = d.session.current_page();
                            d.edit(cx, |s| s.duplicate_page(i));
                        });
                    })).build().h(px(38.)))
                    .child(item("o-delete", "Delete page", "trash", None).on_click(cx.listener(|this, _, window, cx| {
                        if let Some(i) = this.doc().map(|d| d.read(cx).session.current_page()) {
                            this.delete_pages(vec![i], window, cx);
                        }
                    })).build().h(px(38.)))
                    .child(divider_h())
                    .child(item("o-blank", "Insert blank page", "plus", None).on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| {
                            let i = d.session.current_page() + 1;
                            d.edit(cx, |s| s.insert_blank(i));
                        });
                    })).build().h(px(38.)))
                    .child(item("o-insert", "Insert from PDF", "folder", None).on_click(cx.listener(|this, _, window, cx| this.open_dialog(true, window, cx))).build().h(px(38.)))
                    .child(item("o-extract", "Extract pages", "export", None).on_click(cx.listener(|this, _, window, cx| this.extract_pages(window, cx))).build().h(px(38.)))
                    .child(item("o-split", "Split into PDFs", "pages", None).on_click(cx.listener(|this, _, window, cx| this.split(window, cx))).build().h(px(38.)))
                    .child(divider_h())
                    .child(item("o-back", "Back to document", "left", None).on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::AllTools, cx))).build().h(px(38.)));
            }
        }
        div()
            .id("tool-panel")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(if width >= 900. { 256. } else { 215. }))
            .h_full()
            .bg(rgb(0xFFFFFF))
            .border_r_1()
            .border_color(rgb(0xDADADA))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .h(px(57.))
                    .pl(px(21.))
                    .pr(px(12.))
                    .child(div().flex_1().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(self.mode.title()))
                    .child(button("close-left").icon("close").tooltip("Close panel").on_click(cx.listener(|this, _, _, cx| {
                        this.left_open = false;
                        cx.notify();
                    }))),
            )
            .child(div().id("tool-items").flex_1().overflow_y_scroll().child(items))
    }

    pub fn palette(&mut self, selected: u32, cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().items_center().gap(px(5.)).px(px(6.)).py(px(4.));
        for (name, color) in theme::PALETTE {
            row = row.child(
                div()
                    .id(name)
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(selected == color, |d| d.bg(rgb(theme::ACCENT_SOFT)))
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .tooltip(theme::tooltip(format!("{name} annotation color")))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_color(color, cx)))
                    .child(div().size(px(19.)).rounded_full().bg(theme::argb(color)).border_1().border_color(rgba(0x00000033))),
            );
        }
        row
    }

    fn stepper(&mut self, label: &'static str, value: f64, step: f64, cx: &mut Context<Self>, apply: fn(&mut crate::document::DocumentView, f64)) -> Div {
        div()
            .flex()
            .items_center()
            .px(px(8.))
            .py(px(4.))
            .child(div().flex_1().text_size(px(12.)).text_color(rgb(0x656565)).child(label))
            .child(button(SharedString::from(format!("{label}-down"))).icon("zoom_out").tooltip(format!("Decrease {}", label.to_lowercase())).on_click(Self::step_listener(label, -step, apply, cx)))
            .child(div().w(px(40.)).text_center().text_size(px(12.)).child(format!("{value:.1}").trim_end_matches(".0").to_string()))
            .child(button(SharedString::from(format!("{label}-up"))).icon("plus").tooltip(format!("Increase {}", label.to_lowercase())).on_click(Self::step_listener(label, step, apply, cx)))
    }

    /// Changes the tool's font size or stroke width, and the selected annotation's with it.
    fn step_listener(label: &'static str, delta: f64, apply: fn(&mut crate::document::DocumentView, f64), cx: &mut Context<Self>) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static {
        let is_font = label == "Font size";
        cx.listener(move |this: &mut Workbench, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Workbench>| {
            this.with_doc(cx, |d, cx| {
                let current = if is_font { d.session.font_size } else { d.session.stroke_width };
                apply(d, current + delta);
                if let Some(a) = d.session.selected_annotation().cloned() {
                    let (font, stroke) = (d.session.font_size, d.session.stroke_width);
                    d.edit(cx, |s| {
                        s.update_annotation(a.id, if is_font { "Change font size" } else { "Change stroke width" }, |a| {
                            let mut a = a.clone();
                            if is_font {
                                a.font_size = font;
                                if a.kind == AnnotationKind::Text {
                                    let lines = refr_core::text_layout::wrap(&a.text, font, a.bounds.width).len();
                                    a.bounds.height = refr_core::text_layout::height(lines, font);
                                }
                            } else {
                                a.stroke_width = stroke;
                            }
                            a
                        })
                    });
                }
                cx.notify();
            });
        })
    }

    pub fn set_color(&mut self, color: u32, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| {
            d.session.color = color;
            if let Some(a) = d.session.selected_annotation().cloned() {
                d.edit(cx, |s| s.update_annotation(a.id, "Change annotation color", |a| refr_core::Annotation { color, ..a.clone() }));
            }
            cx.notify();
        });
    }

    pub fn rotate_current(&mut self, degrees: i32, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| {
            let i = d.session.current_page();
            d.edit(cx, |s| s.rotate_page(i, degrees));
        });
    }

    pub fn move_current(&mut self, delta: isize, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| {
            let i = d.session.current_page();
            let target = i.saturating_add_signed(delta);
            if target != i && target < d.doc().pages.len() {
                d.edit(cx, |s| s.move_page(i, target));
            }
        });
    }

    fn nav_rail(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let (page, total, zoom, layout) = self.doc().map(|d| { let d = d.read(cx); (d.session.current_page() + 1, d.doc().pages.len(), d.zoom, d.layout_mode) }).unwrap_or((1, 1, 1.0, PageLayoutMode::Continuous));
        if !self.page_input.read(cx).is_focused(window) {
            self.page_input.update(cx, |i, cx| i.set_text(page.to_string(), cx));
        }
        let panel_button = |id: &'static str, icon_name: &'static str, panel: Panel, selected: bool, cx: &mut Context<Self>| {
            button(id).icon(icon_name).tooltip(panel.title()).selected(selected).on_click(cx.listener(move |this, _, _, cx| this.toggle_panel(panel, cx)))
        };
        let next_layout = match layout {
            PageLayoutMode::Continuous => PageLayoutMode::SinglePage,
            PageLayoutMode::SinglePage => PageLayoutMode::TwoPage,
            PageLayoutMode::TwoPage => PageLayoutMode::Continuous,
        };
        let layout_label = match layout {
            PageLayoutMode::Continuous => "Layout: continuous (click for single page)",
            PageLayoutMode::SinglePage => "Layout: single page (click for two pages)",
            PageLayoutMode::TwoPage => "Layout: two pages (click for continuous)",
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .items_center()
            .justify_between()
            .w(px(54.))
            .h_full()
            .py(px(12.))
            .bg(rgb(0xFFFFFF))
            .border_l_1()
            .border_color(rgb(theme::PANEL_BORDER))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(panel_button("p-comments", "comment", Panel::Comments, self.right == Some(Panel::Comments), cx))
                    .child(panel_button("p-bookmarks", "bookmark", Panel::Bookmarks, self.right == Some(Panel::Bookmarks), cx))
                    .child(panel_button("p-thumbs", "pages", Panel::Thumbnails, self.right == Some(Panel::Thumbnails), cx))
                    .child(panel_button("p-props", "info", Panel::Properties, self.right == Some(Panel::Properties), cx))
                    .child(panel_button("p-find", "search", Panel::Find, self.right == Some(Panel::Find), cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(3.))
                    .child(theme::field().w(px(40.)).h(px(28.)).px(px(4.)).text_size(px(12.)).child(self.page_input.clone()))
                    .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child(format!("/ {total}")))
                    .child(button("prev").icon("up").tooltip("Previous page (Page Up)").on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.go_to_page(d.session.current_page().saturating_sub(1), cx));
                    })))
                    .child(button("next").icon("down").tooltip("Next page (Page Down)").on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.go_to_page(d.session.current_page() + 1, cx));
                    })))
                    .child(div().w(px(30.)).h(px(1.)).my(px(6.)).bg(rgb(0xE4E4E4)))
                    .child(button("rotate").icon("rotate").tooltip("Rotate page clockwise").on_click(cx.listener(|this, _, _, cx| this.rotate_current(90, cx))))
                    .child(button("layout").icon("grid").tooltip(layout_label).on_click(cx.listener(move |this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.set_layout_mode(next_layout, cx));
                    })))
                    .child(button("fit").icon("fit_page").tooltip("Fit page (⌘0)").on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.fit(Fit::Page, cx));
                    })))
                    .child(button("fit-width").icon("fit_width").tooltip("Fit width").on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.fit(Fit::Width, cx));
                    })))
                    .child(button("zoom-in").icon("plus").tooltip("Zoom in (⌘+)").on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.zoom_to(d.zoom * 1.2, None, cx));
                    })))
                    .child(button("zoom-out").icon("zoom_out").tooltip("Zoom out (⌘−)").on_click(cx.listener(|this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.zoom_to(d.zoom / 1.2, None, cx));
                    })))
                    .child(
                        div()
                            .id("zoom-label")
                            .px(px(4.))
                            .py(px(3.))
                            .rounded(px(4.))
                            .text_size(px(10.))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .tooltip(theme::tooltip("Set zoom"))
                            .on_click(cx.listener(|this, _, window, cx| this.set_zoom_dialog(window, cx)))
                            .child(format!("{:.0}%", zoom * 100.0)),
                    ),
            )
    }

    fn quick_tools(&mut self, cx: &mut Context<Self>) -> Div {
        let tool = self.doc().map(|d| d.read(cx).session.tool());
        let mut column = div().flex().flex_col().gap(px(3.)).p(px(5.));
        for (t, name, icon_name) in [
            (PdfTool::Select, "Select (V)", "select"),
            (PdfTool::Hand, "Hand (H)", "hand"),
            (PdfTool::Highlight, "Highlight text", "highlight"),
            (PdfTool::Note, "Add a comment", "comment"),
            (PdfTool::Ink, "Draw freehand (D)", "pen"),
            (PdfTool::Text, "Add text (T)", "text"),
            (PdfTool::Signature, "Draw signature", "sign"),
        ] {
            column = column.child(button(name).icon(icon_name).tooltip(name).selected(tool == Some(t)).on_click(cx.listener(move |this, _, window, cx| this.use_tool(t, window, cx))).build().h(px(36.)));
        }
        column = column.child(div().h(px(1.)).mx(px(4.)).my(px(3.)).bg(rgb(0xE4E4E4))).child(button("more-tools").icon("more").tooltip("More drawing tools").on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Edit, cx))));
        div().absolute().top(px(22.)).left(px(15.)).bg(rgb(0xFFFFFF)).border_1().border_color(rgb(0xD4D4D4)).rounded(px(9.)).shadow_md().child(column)
    }

    fn selection_bar(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let doc = self.doc()?.read(cx);
        let has_text = doc.text_selection.is_some();
        let selected = doc.session.selected_annotation().cloned();
        if !has_text && selected.is_none() {
            return None;
        }
        let mut row = div().flex().items_center().gap(px(6.)).px(px(8.)).py(px(5.));
        if has_text && selected.is_none() {
            row = row
                .child(button("sel-copy").icon("copy").label("Copy").on_click(cx.listener(|this, _, _, cx| {
                    this.with_doc(cx, |d, cx| d.copy_selection(cx));
                })))
                .child(divider_v())
                .child(button("sel-highlight").icon("highlight").tooltip("Highlight").on_click(cx.listener(|this, _, _, cx| {
                    this.with_doc(cx, |d, cx| d.markup_selection(AnnotationKind::Highlight, cx));
                })))
                .child(button("sel-underline").icon("underline").tooltip("Underline").on_click(cx.listener(|this, _, _, cx| {
                    this.with_doc(cx, |d, cx| d.markup_selection(AnnotationKind::Underline, cx));
                })))
                .child(button("sel-strike").icon("strikeout").tooltip("Strikethrough").on_click(cx.listener(|this, _, _, cx| {
                    this.with_doc(cx, |d, cx| d.markup_selection(AnnotationKind::Strikeout, cx));
                })));
        } else if let Some(a) = selected {
            let id = a.id;
            row = row.child(self.palette(a.color, cx)).child(divider_v());
            if matches!(a.kind, AnnotationKind::Text | AnnotationKind::Note | AnnotationKind::Stamp) {
                row = row.child(button("sel-edit").icon("edit").tooltip("Edit text").on_click(cx.listener(move |this, _, window, cx| this.edit_annotation_text(id, window, cx))));
            }
            row = row
                .child(button("sel-props").icon("settings").tooltip("Annotation properties").on_click(cx.listener(|this, _, _, cx| {
                    this.right = Some(Panel::Properties);
                    cx.notify();
                })))
                .child(button("sel-delete").icon("trash").tooltip("Delete annotation (Delete)").on_click(cx.listener(|this, _, _, cx| {
                    this.with_doc(cx, |d, cx| d.edit(cx, |s| s.delete_selection()));
                })));
        }
        Some(
            div()
                .absolute()
                .bottom(px(18.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(div().bg(rgb(0xFFFFFF)).border_1().border_color(rgb(0xCCCCCC)).rounded(px(8.)).shadow_lg().child(row)),
        )
    }

    fn footer(&mut self, width: f32, cx: &mut Context<Self>) -> Div {
        let info = self.doc().map(|d| {
            let d = d.read(cx);
            let doc = d.doc();
            format!("{} pages  ·  {} annotations  ·  Local only", doc.pages.len(), doc.annotation_count())
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(26.))
            .px(px(14.))
            .bg(rgb(theme::FOOTER))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .child(div().flex_1().truncate().text_size(px(11.)).text_color(rgb(if self.status.1 { theme::ERROR } else { theme::MUTED })).child(self.status.0.clone()))
            .when(width >= 850., |d| d.children(info.map(|i| div().text_size(px(10.)).text_color(rgb(theme::FAINT)).child(i))))
    }

    fn home_view(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let card = |id: &'static str, title: &'static str, detail: &'static str, icon_name: &'static str, color: u32| {
            div()
                .id(id)
                .flex()
                .flex_col()
                .gap(px(10.))
                .w(px(210.))
                .p(px(18.))
                .rounded(px(10.))
                .border_1()
                .border_color(rgb(theme::BORDER))
                .bg(rgb(0xFFFFFF))
                .cursor_pointer()
                .hover(|s| s.border_color(rgb(theme::ACCENT)).shadow_md())
                .child(icon(icon_name).size(px(28.)).text_color(theme::argb(color)))
                .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(title))
                .child(div().text_size(px(12.)).text_color(rgb(theme::MUTED)).child(detail))
        };
        let mut recent = div().flex().flex_col().gap(px(2.));
        if self.recents.is_empty() {
            recent = recent.child(div().text_size(px(12.)).text_color(rgb(theme::FAINT)).child("Files you open appear here."));
        }
        for (i, path) in self.recents.clone().into_iter().enumerate() {
            let open = path.clone();
            recent = recent.child(
                div()
                    .id(("recent", i))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_path(open.clone(), false, window, cx)))
                    .child(icon(if crate::storage::is_workspace(&path) { "save" } else { "file" }).text_color(rgb(theme::BRAND)))
                    .child(div().text_size(px(13.)).child(file_name(&path)))
                    .child(div().flex_1().truncate().text_size(px(11.)).text_color(rgb(theme::FAINT)).child(path.parent().map(|p| p.display().to_string()).unwrap_or_default())),
            );
        }
        div()
            .id("home")
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(0xF8F8F8))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(24.))
                    .max_w(px(920.))
                    .mx_auto()
                    .px(px(40.))
                    .py(px(48.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(div().text_size(px(28.)).font_weight(FontWeight::SEMIBOLD).child("Welcome to Refr"))
                            .child(div().text_size(px(14.)).text_color(rgb(theme::MUTED)).child("A PDF workspace built for your documents, on your device. No account, no upload.")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(14.))
                            .child(card("home-open", "Open a file", "PDF or .pdfspace workspace", "folder", 0xFF1473E6).on_click(cx.listener(|this, _, window, cx| this.open_dialog(false, window, cx))))
                            .child(card("home-blank", "Create a PDF", "Start from a blank page", "file", 0xFFD93830).on_click(cx.listener(|this, _, window, cx| this.new_blank(window, cx))))
                            .child(card("home-sample", "Open the sample", "A six-page report to try the tools", "star", 0xFF29834B).on_click(cx.listener(|this, _, window, cx| this.open_sample(window, cx))))
                            .child(card("home-combine", "Combine files", "Append PDFs to the current document", "copy", 0xFF7854BD).on_click(cx.listener(|this, _, window, cx| this.open_dialog(true, window, cx)))),
                    )
                    .child(div().flex().flex_col().gap(px(8.)).child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child("Recent files")).child(recent))
                    .child(description("Cropping is not redaction. A drawn signature is a visual mark, not a certificate-based digital signature. OCR, encryption and secure redaction are not available.")),
            )
    }

    fn dialog_view(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let dialog = self.dialog.as_ref()?;
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0x00000033))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .w(px(420.))
                        .p(px(22.))
                        .bg(rgb(0xFFFFFF))
                        .rounded(px(12.))
                        .shadow_lg()
                        .child(div().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(dialog.title.clone()))
                        .child(div().text_size(px(13.)).text_color(rgb(theme::MUTED)).child(dialog.message.clone()))
                        .child(theme::field().h(px(36.)).child(dialog.input.clone()))
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(button("dialog-cancel").label("Cancel").on_click(cx.listener(|this, _, window, cx| this.cancel_dialog(window, cx))))
                                .child(button("dialog-accept").label(dialog.accept.clone()).primary().on_click(cx.listener(|this, _, window, cx| this.accept_dialog(window, cx)))),
                        ),
                ),
        )
    }

    fn center(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(doc) = self.doc().cloned() else { return div().into_any_element() };
        if self.mode == Mode::Organize {
            return self.organizer(cx).into_any_element();
        }
        div()
            .relative()
            .flex_1()
            .h_full()
            .min_w(px(100.))
            .overflow_hidden()
            .child(doc)
            .child(self.quick_tools(cx))
            .children(self.selection_bar(cx))
            .into_any_element()
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width);
        let body: AnyElement = if self.home {
            self.home_view(cx).into_any_element()
        } else {
            let right_width = if width >= 900. { 290. } else if width >= 650. { 240. } else { (width - 200.).max(0.) };
            div()
                .flex()
                .size_full()
                .overflow_hidden()
                .when(self.left_open && width >= 650., |d| d.child(self.tool_panel(width, cx)))
                .child(self.center(cx))
                .when_some(self.right, |d, panel| d.child(self.side_panel(panel, right_width, window, cx)))
                .child(self.nav_rail(window, cx))
                .into_any_element()
        };
        div()
            .id("workbench")
            .key_context("Workbench")
            .track_focus(&self.focus_handle)
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xFFFFFF))
            .font_family(theme::FONT)
            .text_color(rgb(theme::TEXT))
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.flush_recovery(cx);
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &OpenFile, window, cx| this.open_dialog(false, window, cx)))
            .on_action(cx.listener(|this, _: &NewBlank, window, cx| this.new_blank(window, cx)))
            .on_action(cx.listener(|this, _: &OpenSample, window, cx| this.open_sample(window, cx)))
            .on_action(cx.listener(|this, _: &SaveWorkspace, window, cx| this.save_workspace(false, window, cx)))
            .on_action(cx.listener(|this, _: &SaveWorkspaceAs, window, cx| this.save_workspace(true, window, cx)))
            .on_action(cx.listener(|this, _: &ExportPdf, window, cx| this.export_pdf(None, window, cx)))
            .on_action(cx.listener(|this, _: &Print, _, cx| this.print(cx)))
            .on_action(cx.listener(|this, _: &CloseTab, window, cx| this.close_document(this.active, window, cx)))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| {
                let next = (this.active + 1) % this.documents.len().max(1);
                this.activate(next, window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousTab, window, cx| {
                let count = this.documents.len().max(1);
                this.activate((this.active + count - 1) % count, window, cx);
            }))
            .on_action(cx.listener(|this, _: &Find, window, cx| {
                this.right = Some(Panel::Find);
                this.home = false;
                this.find_input.update(cx, |i, cx| i.select_all_text(cx));
                this.find_input.read(cx).focus(window);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Undo, _, cx| {
                this.with_doc(cx, |d, cx| d.undo(cx));
            }))
            .on_action(cx.listener(|this, _: &Redo, _, cx| {
                this.with_doc(cx, |d, cx| d.redo(cx));
            }))
            .on_action(cx.listener(|this, _: &FitPage, _, cx| {
                this.with_doc(cx, |d, cx| d.fit(Fit::Page, cx));
            }))
            .on_action(cx.listener(|this, _: &FitWidth, _, cx| {
                this.with_doc(cx, |d, cx| d.fit(Fit::Width, cx));
            }))
            .on_action(cx.listener(|this, _: &ActualSize, _, cx| {
                this.with_doc(cx, |d, cx| d.zoom_to(1.0, None, cx));
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| {
                this.with_doc(cx, |d, cx| d.zoom_to(d.zoom * 1.2, None, cx));
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| {
                this.with_doc(cx, |d, cx| d.zoom_to(d.zoom / 1.2, None, cx));
            }))
            .on_action(cx.listener(|this, _: &PreviousPage, _, cx| {
                this.with_doc(cx, |d, cx| d.go_to_page(d.session.current_page().saturating_sub(1), cx));
            }))
            .on_action(cx.listener(|this, _: &NextPage, _, cx| {
                this.with_doc(cx, |d, cx| d.go_to_page(d.session.current_page() + 1, cx));
            }))
            .on_action(cx.listener(|this, _: &FirstPage, _, cx| {
                this.with_doc(cx, |d, cx| d.go_to_page(0, cx));
            }))
            .on_action(cx.listener(|this, _: &LastPage, _, cx| {
                this.with_doc(cx, |d, cx| d.go_to_page(usize::MAX, cx));
            }))
            .on_action(cx.listener(|this, _: &ShowHelp, window, cx| this.show_help(window, cx)))
            .on_action(cx.listener(|this, _: &ShowAbout, window, cx| this.show_about(window, cx)))
            .on_action(cx.listener(|this, _: &ShowHome, _, cx| {
                this.home = !this.home;
                cx.notify();
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                for path in paths.paths() {
                    this.open_path(path.clone(), false, window, cx);
                }
            }))
            .child(self.title_bar(width, cx))
            .child(self.global_bar(width, cx))
            .child(div().flex_1().min_h(px(0.)).overflow_hidden().child(body))
            .child(self.footer(width, cx))
            .children(self.dialog_view(cx))
            .children(self.about_view(cx))
    }
}
