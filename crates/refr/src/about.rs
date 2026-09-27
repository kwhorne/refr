//! The About Refr window: version, what the app is, credits and links.

use gpui::{Context, Div, FocusHandle, FontWeight, InteractiveElement, MouseButton, ParentElement, SharedString, Styled, Window, div, px, rgb, rgba};

use crate::actions::CloseModal;
use crate::theme::{self, button, icon};
use crate::workbench::Workbench;

pub const REPOSITORY: &str = "https://github.com/kwhorne/refr";
const PDFSPACE: &str = "https://github.com/wieslawsoltes/PdfSpace";

impl Workbench {
    pub fn show_about(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = cx.focus_handle();
        window.focus(&handle);
        self.about = Some(handle);
        cx.notify();
    }

    pub fn close_about(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.about.take().is_some() {
            self.focus_document(window, cx);
        }
    }

    pub(crate) fn about_view(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let handle: FocusHandle = self.about.clone()?;
        let fact = |label: &'static str, value: SharedString| {
            div()
                .flex()
                .gap(px(10.))
                .text_size(px(12.))
                .child(div().w(px(92.)).flex_none().text_color(rgb(theme::FAINT)).child(label))
                .child(div().flex_1().min_w(px(0.)).line_height(px(17.)).child(value))
        };
        let link = |id: &'static str, label: &'static str, url: &'static str, cx: &mut Context<Self>| {
            button(id).label(label).on_click(cx.listener(move |_, _, _, cx| cx.open_url(url)))
        };
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0x00000033))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_about(window, cx)))
                .child(
                    div()
                        .id("about")
                        .key_context("Modal")
                        .track_focus(&handle)
                        .on_action(cx.listener(|this, _: &CloseModal, window, cx| this.close_about(window, cx)))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .flex()
                        .flex_col()
                        .w(px(440.))
                        .bg(rgb(0xFFFFFF))
                        .rounded(px(14.))
                        .shadow_lg()
                        .overflow_hidden()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(6.))
                                .pt(px(30.))
                                .pb(px(22.))
                                .px(px(28.))
                                .bg(rgb(0xF7F7F7))
                                .border_b_1()
                                .border_color(rgb(theme::BORDER))
                                .child(
                                    div()
                                        .size(px(64.))
                                        .rounded(px(16.))
                                        .bg(rgb(0xFFFFFF))
                                        .border_1()
                                        .border_color(rgb(theme::BORDER))
                                        .shadow_sm()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(icon("file").size(px(38.)).text_color(rgb(theme::BRAND))),
                                )
                                .child(div().mt(px(8.)).text_size(px(22.)).font_weight(FontWeight::SEMIBOLD).child("Refr"))
                                .child(div().text_size(px(12.)).text_color(rgb(theme::MUTED)).child(format!("Version {}", env!("CARGO_PKG_VERSION"))))
                                .child(div().mt(px(6.)).text_size(px(14.)).text_center().child("A local-first PDF workspace for macOS.")),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(14.))
                                .px(px(28.))
                                .py(px(20.))
                                .child(div().text_size(px(13.)).line_height(px(19.)).text_color(rgb(0x444444)).child(
                                    "Read, annotate, comment, sign, organize and export PDFs. Your documents stay on this Mac: \
                                     no account, no upload and no telemetry.",
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .child(fact("Built with", "GPUI for the interface and PDFium for reading, rendering and writing PDFs.".into()))
                                        .child(fact("Based on", "PdfSpace by Wiesław Šoltés, whose .pdfspace workspaces Refr opens and saves.".into()))
                                        .child(fact("License", "MIT. PDFium is BSD-3-Clause and Apache-2.0.".into())),
                                )
                                .child(div().text_size(px(11.)).line_height(px(16.)).text_color(rgb(theme::FAINT)).child(
                                    "Cropping is not redaction, and a drawn signature is a visual mark, not a certificate-based digital signature.",
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .px(px(20.))
                                .pb(px(18.))
                                .child(link("about-github", "Refr on GitHub", REPOSITORY, cx))
                                .child(link("about-pdfspace", "PdfSpace", PDFSPACE, cx))
                                .child(div().flex_1())
                                .child(button("about-close").label("Close").primary().on_click(cx.listener(|this, _, window, cx| this.close_about(window, cx)))),
                        ),
                ),
        )
    }
}
