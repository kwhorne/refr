//! Right-hand panels (comments, bookmarks, thumbnails, properties, find) and the
//! page organizer grid.

use gpui::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render, ScrollStrategy, SharedString,
    Stateful, Styled, StyledImage, Window, div, img, prelude::*, px, rgb, rgba, uniform_list,
};
use refr_core::{AnnotationKind, PdfPageState};

use crate::document::THUMB_WIDTH;
use crate::theme::{self, button, description, heading, icon};
use crate::workbench::{Mode, Panel, Workbench};

const ORGANIZE_CELL: f32 = 195.;
const ORGANIZE_ROW: f32 = 240.;
const THUMB_ROW: f32 = 200.;

#[derive(Clone)]
struct DraggedPage {
    index: usize,
    label: SharedString,
}

impl Render for DraggedPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(12.))
            .py(px(8.))
            .bg(rgb(0xFFFFFF))
            .border_1()
            .border_color(rgb(theme::ACCENT))
            .rounded(px(6.))
            .shadow_lg()
            .text_size(px(12.))
            .child(self.label.clone())
    }
}

/// Size of a page thumbnail fitted into a `max_w` × `max_h` box.
fn fitted(page: &PdfPageState, max_w: f32, max_h: f32) -> (f32, f32) {
    let (w, h) = (page.display_width() as f32, page.display_height() as f32);
    let scale = (max_w / w).min(max_h / h);
    (w * scale, h * scale)
}

fn format_date(date: &chrono::DateTime<chrono::Utc>) -> String {
    date.with_timezone(&chrono::Local).format("%-d %b %Y, %H:%M").to_string()
}

impl Workbench {
    pub fn side_panel(&mut self, panel: Panel, width: f32, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let content: AnyElement = match panel {
            Panel::Thumbnails => self.thumbnails(cx).into_any_element(),
            Panel::Comments => self.comments(window, cx).into_any_element(),
            Panel::Bookmarks => self.bookmarks(cx).into_any_element(),
            Panel::Properties => self.properties(cx).into_any_element(),
            Panel::Find => self.find_panel(cx).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(width))
            .h_full()
            .bg(rgb(0xFFFFFF))
            .border_l_1()
            .border_color(rgb(theme::PANEL_BORDER))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .h(px(55.))
                    .pl(px(18.))
                    .pr(px(10.))
                    .child(div().flex_1().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child(panel.title()))
                    .child(button("close-right").icon("close").tooltip(format!("Close {}", panel.title())).on_click(cx.listener(|this, _, _, cx| {
                        this.right = None;
                        cx.notify();
                    }))),
            )
            .child(div().flex_1().min_h(px(0.)).child(content))
    }

    fn scroll_column(id: &'static str) -> Stateful<Div> {
        div().id(id).size_full().overflow_y_scroll().flex().flex_col().gap(px(10.)).px(px(15.)).pt(px(5.)).pb(px(24.))
    }

    fn thumbnails(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(doc) = self.doc().cloned() else { return div().into_any_element() };
        let count = doc.read(cx).doc().pages.len();
        uniform_list(
            "thumbnails",
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                let Some(doc) = this.doc().cloned() else { return Vec::new() };
                let current = doc.read(cx).session.current_page();
                range
                    .map(|index| {
                        let page = doc.read(cx).doc().pages[index].clone();
                        let (image, failed) = if page.source_id.is_some() { doc.update(cx, |d, cx| d.thumbnail(index, cx)) } else { (None, false) };
                        let (w, h) = fitted(&page, THUMB_WIDTH as f32, 160.);
                        let selected = index == current;
                        div()
                            .id(("thumb", index))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(6.))
                            .h(px(THUMB_ROW))
                            .w_full()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.with_doc(cx, |d, cx| d.go_to_page(index, cx));
                            }))
                            .child(
                                div()
                                    .w(px(w + 6.))
                                    .h(px(h + 6.))
                                    .p(px(2.))
                                    .rounded(px(4.))
                                    .border_2()
                                    .border_color(if selected { rgb(theme::ACCENT) } else { rgba(0) })
                                    .child(thumb_image(image, failed, w, h)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(if selected { theme::ACCENT_DARK } else { theme::MUTED }))
                                    .child(if page.bookmark.is_empty() { format!("{}", index + 1) } else { format!("{} · {}", index + 1, page.bookmark) }),
                            )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.thumb_scroll.clone())
        .size_full()
        .into_any_element()
    }

    fn comments(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(doc) = self.doc().cloned() else { return Self::scroll_column("comments") };
        let (workspace, selected) = {
            let d = doc.read(cx);
            (d.doc().clone(), d.session.selected_id())
        };
        let hide_resolved = self.hide_resolved;
        let mut column = Self::scroll_column("comments").child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(button("add-comment").icon("comment").label("Add comment").on_click(cx.listener(|this, _, window, cx| this.use_tool(refr_core::PdfTool::Note, window, cx))))
                .child(div().flex_1())
                .child(button("hide-resolved").label(if hide_resolved { "Show resolved" } else { "Hide resolved" }).on_click(cx.listener(|this, _, _, cx| {
                    this.hide_resolved = !this.hide_resolved;
                    cx.notify();
                }))),
        );
        let mut any = false;
        for (page_index, page) in workspace.pages.iter().enumerate() {
            for a in &page.annotations {
                if hide_resolved && a.resolved {
                    continue;
                }
                any = true;
                let id = a.id;
                let active = selected == Some(id);
                let kind_label = match a.kind {
                    AnnotationKind::Note => "Comment".to_string(),
                    other => {
                        let label = other.label();
                        let mut chars = label.chars();
                        chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
                    }
                };
                let mut card = div()
                    .id(SharedString::from(format!("card-{id}")))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .p(px(12.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgb(if active { theme::ACCENT } else { theme::BORDER }))
                    .bg(rgb(if a.resolved { 0xF7F7F7 } else { 0xFFFFFF }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.reply_to = Some(id);
                        this.with_doc(cx, |d, cx| {
                            d.session.reveal(id);
                            if let Some(a) = d.doc().pages.iter().find_map(|p| p.annotation(id)).cloned() {
                                let page = d.session.current_page();
                                d.reveal_rect(page, a.bounds, cx);
                                d.session.select(Some(id));
                            }
                        });
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().size(px(24.)).rounded_full().bg(theme::argb(a.color)).flex().items_center().justify_center().text_color(rgb(0xFFFFFF)).text_size(px(11.)).child(a.author.chars().next().unwrap_or('?').to_string()))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child(format!("{} · {kind_label}", a.author)))
                                    .child(div().text_size(px(10.)).text_color(rgb(theme::FAINT)).child(format!("Page {} · {}", page_index + 1, format_date(&a.created)))),
                            )
                            .when(a.resolved, |d| d.child(div().text_size(px(10.)).text_color(rgb(0x29834B)).child("Resolved"))),
                    );
                if !a.text.is_empty() {
                    card = card.child(div().text_size(px(13.)).child(a.text.clone()));
                }
                for reply in &a.replies {
                    card = card.child(
                        div()
                            .ml(px(12.))
                            .pl(px(10.))
                            .border_l_2()
                            .border_color(rgb(0xE4E4E4))
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).child(format!("{} · {}", reply.author, format_date(&reply.created))))
                            .child(div().text_size(px(12.)).child(reply.text.clone())),
                    );
                }
                if active {
                    let resolved = a.resolved;
                    card = card
                        .child(theme::field().child(self.reply_input.clone()))
                        .child(
                            div()
                                .flex()
                                .gap(px(4.))
                                .child(button(SharedString::from(format!("resolve-{id}"))).icon("check").label(if resolved { "Reopen" } else { "Resolve" }).on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.with_doc(cx, |d, cx| d.edit(cx, |s| s.set_resolved(id, !resolved)));
                                })))
                                .child(button(SharedString::from(format!("delete-{id}"))).icon("trash").label("Delete").on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.with_doc(cx, |d, cx| d.edit(cx, |s| s.delete_annotation(id)));
                                }))),
                        );
                }
                column = column.child(card);
            }
        }
        if !any {
            column = column.child(description("No comments or marks yet. Use Add comment, highlight text, or draw on a page."));
        }
        if self.reply_to.is_some() && !self.reply_input.read(cx).is_focused(window) {
            // Keep the reply field ready without stealing focus from the document.
        }
        column
    }

    fn bookmarks(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(doc) = self.doc().cloned() else { return Self::scroll_column("bookmarks") };
        let (workspace, current) = {
            let d = doc.read(cx);
            (d.doc().clone(), d.session.current_page())
        };
        let mut column = Self::scroll_column("bookmarks").child(
            button("bookmark-current").icon("bookmark").label("Bookmark current page").on_click(cx.listener(move |this, _, window, cx| this.edit_bookmark(current, window, cx))),
        );
        let mut any = false;
        for (index, page) in workspace.pages.iter().enumerate().filter(|(_, p)| !p.bookmark.is_empty()) {
            any = true;
            column = column.child(
                div()
                    .id(("bookmark", index))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(index == current, |d| d.bg(rgb(theme::ACCENT_SOFT)))
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.with_doc(cx, |d, cx| d.go_to_page(index, cx));
                    }))
                    .child(icon("bookmark").size(px(16.)).text_color(rgb(theme::ACCENT)))
                    .child(div().flex_1().truncate().text_size(px(13.)).child(page.bookmark.clone()))
                    .child(div().text_size(px(11.)).text_color(rgb(theme::FAINT)).child(format!("{}", index + 1)))
                    .child(button(("edit-bookmark", index)).icon("edit").tooltip("Rename bookmark").on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.edit_bookmark(index, window, cx);
                    }))),
            );
        }
        if !any {
            column = column.child(description("Workspace bookmarks name pages for quick navigation. They are separate from the PDF's own outline."));
        }
        column
    }

    fn properties(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(doc) = self.doc().cloned() else { return Self::scroll_column("properties") };
        let (workspace, current, selected, path) = {
            let d = doc.read(cx);
            (d.doc().clone(), d.session.current_page(), d.session.selected_annotation().cloned(), d.path.clone())
        };
        let row = |label: &str, value: String| {
            div().flex().gap(px(8.)).text_size(px(12.)).child(div().w(px(80.)).flex_none().text_color(rgb(theme::MUTED)).child(label.to_string())).child(div().flex_1().child(value))
        };
        let page = &workspace.pages[current];
        let mm = |pt: f64| pt / 72.0 * 25.4;
        let source_bytes: usize = workspace.sources.iter().map(|s| s.bytes.len()).sum();
        let mut column = Self::scroll_column("properties")
            .child(heading("DOCUMENT"))
            .child(row("Title", workspace.title.clone()))
            .child(row("Author", if workspace.author.is_empty() { "—".into() } else { workspace.author.clone() }))
            .child(row("Pages", workspace.pages.len().to_string()))
            .child(row("Sources", format!("{} PDF{} · {:.1} MB", workspace.sources.len(), if workspace.sources.len() == 1 { "" } else { "s" }, source_bytes as f64 / 1_048_576.0)))
            .child(row("Workspace", path.map(|p| p.display().to_string()).unwrap_or_else(|| "Not saved".into())))
            .child(button("edit-props").icon("edit").label("Edit title and author").on_click(cx.listener(|this, _, window, cx| this.edit_properties(window, cx))))
            .child(heading(format!("PAGE {}", current + 1)))
            .child(row("Size", format!("{:.0} × {:.0} pt ({:.0} × {:.0} mm)", page.display_width(), page.display_height(), mm(page.display_width()), mm(page.display_height()))))
            .child(row("Rotation", format!("{}°", page.rotation)))
            .child(row("Crop", page.crop.map(|c| format!("{:.0} × {:.0} pt", c.width, c.height)).unwrap_or_else(|| "None".into())))
            .child(row("Bookmark", if page.bookmark.is_empty() { "—".into() } else { page.bookmark.clone() }))
            .child(row("Marks", page.annotations.len().to_string()));
        if let Some(a) = selected {
            let mut kind = a.kind.label().to_string();
            if let Some(first) = kind.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            column = column
                .child(heading("SELECTED ANNOTATION"))
                .child(row("Type", kind))
                .child(row("Author", a.author.clone()))
                .child(row("Created", format_date(&a.created)))
                .child(row("Position", format!("{:.0}, {:.0} pt", a.bounds.x, a.bounds.y)))
                .child(row("Size", format!("{:.0} × {:.0} pt", a.bounds.width, a.bounds.height)))
                .child(self.palette(a.color, cx));
        }
        column.child(description("Coordinates are PDF points (1/72 inch) from the top-left of the unrotated page."))
    }

    fn find_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let (results, active) = self.doc().map(|d| { let d = d.read(cx); (d.search.clone(), d.active_search) }).unwrap_or_default();
        let match_case = self.match_case;
        let mut column = Self::scroll_column("find")
            .child(theme::field().gap(px(6.)).child(icon("search").size(px(15.)).text_color(rgb(theme::FAINT))).child(div().flex_1().overflow_hidden().child(self.find_input.clone())))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(button("match-case").icon(if match_case { "check" } else { "rectangle" }).label("Match case").on_click(cx.listener(|this, _, _, cx| {
                        this.match_case = !this.match_case;
                        let query = this.find_input.read(cx).text().to_string();
                        this.search(&query, cx);
                    })))
                    .child(div().flex_1())
                    .child(div().text_size(px(11.)).text_color(rgb(theme::FAINT)).child(if results.is_empty() { String::new() } else { format!("{} result{}", results.len(), if results.len() == 1 { "" } else { "s" }) })),
            );
        for (i, result) in results.iter().enumerate().take(500) {
            column = column.child(
                div()
                    .id(("result", i))
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .p(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(if active == Some(i) { theme::ACCENT } else { theme::BORDER }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| this.show_search_result(i, cx)))
                    .child(div().text_size(px(10.)).text_color(rgb(theme::FAINT)).child(format!("Page {}{}", result.page_index + 1, if result.annotation.is_some() { " · annotation" } else { "" })))
                    .child(div().text_size(px(12.)).child(result.excerpt.clone())),
            );
        }
        if results.is_empty() {
            column = column.child(description("Search the document's selectable text and the text of your annotations. Press Return to search."));
        }
        column
    }

    pub fn organizer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(doc) = self.doc().cloned() else { return div().into_any_element() };
        let count = doc.read(cx).doc().pages.len();
        let viewport = doc.read(cx).viewport;
        let width = f32::from(viewport.size.width).max(ORGANIZE_CELL);
        let columns = ((width - 40.) / ORGANIZE_CELL).floor().max(1.) as usize;
        let rows = count.div_ceil(columns);
        div()
            .flex_1()
            .h_full()
            .bg(rgb(theme::CANVAS))
            .child(
                uniform_list(
                    "organizer",
                    rows,
                    cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                        let Some(doc) = this.doc().cloned() else { return Vec::new() };
                        let current = doc.read(cx).session.current_page();
                        let pages = doc.read(cx).doc().pages.clone();
                        range
                            .map(|row| {
                                let mut line = div().flex().justify_center().gap(px(0.)).h(px(ORGANIZE_ROW)).w_full();
                                for index in (row * columns)..((row + 1) * columns).min(pages.len()) {
                                    let page = &pages[index];
                                    let (image, failed) = if page.source_id.is_some() { doc.update(cx, |d, cx| d.thumbnail(index, cx)) } else { (None, false) };
                                    let (w, h) = fitted(page, 150., 185.);
                                    let selected = index == current;
                                    let label: SharedString = if page.bookmark.is_empty() { format!("{}", index + 1).into() } else { format!("{} · {}", index + 1, page.bookmark).into() };
                                    line = line.child(
                                        div()
                                            .id(("organize", index))
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .justify_end()
                                            .gap(px(6.))
                                            .w(px(ORGANIZE_CELL))
                                            .h_full()
                                            .pb(px(12.))
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, e: &gpui::ClickEvent, _, cx| {
                                                this.with_doc(cx, |d, cx| d.go_to_page(index, cx));
                                                if e.click_count() >= 2 {
                                                    this.mode = Mode::AllTools;
                                                }
                                                cx.notify();
                                            }))
                                            .on_drag(DraggedPage { index, label: format!("Page {}", index + 1).into() }, |dragged, _, _, cx| cx.new(|_| dragged.clone()))
                                            .drag_over::<DraggedPage>(|s, _, _, _| s.bg(rgba(0x1473E61A)))
                                            .on_drop(cx.listener(move |this, dragged: &DraggedPage, _, cx| {
                                                let from = dragged.index;
                                                this.with_doc(cx, |d, cx| d.edit(cx, |s| s.move_page(from, index)));
                                            }))
                                            .child(
                                                div()
                                                    .p(px(3.))
                                                    .rounded(px(4.))
                                                    .border_2()
                                                    .border_color(if selected { rgb(theme::ACCENT) } else { rgba(0) })
                                                    .child(thumb_image(image, failed, w, h)),
                                            )
                                            .child(div().max_w(px(170.)).truncate().text_size(px(12.)).text_color(rgb(if selected { theme::ACCENT_DARK } else { 0x444444 })).child(label)),
                                    );
                                }
                                line
                            })
                            .collect()
                    }),
                )
                .track_scroll(self.organize_scroll.clone())
                .size_full()
                .pt(px(12.)),
            )
            .into_any_element()
    }

    pub fn reveal_thumbnail(&mut self, index: usize) {
        self.thumb_scroll.scroll_to_item(index, ScrollStrategy::Center);
    }
}

fn thumb_image(image: Option<std::sync::Arc<gpui::RenderImage>>, failed: bool, w: f32, h: f32) -> AnyElement {
    match image {
        Some(image) => img(image).w(px(w)).h(px(h)).object_fit(ObjectFit::Fill).shadow_sm().into_any_element(),
        None => div()
            .w(px(w))
            .h(px(h))
            .bg(rgb(0xFFFFFF))
            .shadow_sm()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.))
            .text_color(rgb(0xAAAAAA))
            .child(if failed { "!" } else { "" })
            .into_any_element(),
    }
}
