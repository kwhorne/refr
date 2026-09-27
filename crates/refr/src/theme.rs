//! Colors and reusable UI pieces, after PdfSpace's visual language.

use gpui::{
    AnyView, App, ClickEvent, Div, ElementId, Hsla, IntoElement, ParentElement, Render, SharedString, Stateful, Styled,
    Svg, Window, div, prelude::*, px, rgb, rgba, svg,
};

pub const ACCENT: u32 = 0x1473E6;
pub const ACCENT_DARK: u32 = 0x0865CB;
pub const ACCENT_SOFT: u32 = 0xE7F0FF;
pub const TEXT: u32 = 0x2B2B2B;
pub const MUTED: u32 = 0x686868;
pub const FAINT: u32 = 0x858585;
pub const BORDER: u32 = 0xDDDDDD;
pub const PANEL_BORDER: u32 = 0xD8D8D8;
pub const TITLE_BAR: u32 = 0xEFEFEF;
pub const FOOTER: u32 = 0xF7F7F7;
pub const CANVAS: u32 = 0xE8E9EB;
pub const HOVER: u32 = 0xF0F0F0;
pub const ERROR: u32 = 0xB12620;
pub const BRAND: u32 = 0xD93830;

pub const FONT: &str = ".SystemUIFont";
/// Annotation text is painted in Helvetica so it wraps like the exported PDF.
pub const ANNOTATION_FONT: &str = "Helvetica";

pub const PALETTE: [(&str, u32); 6] = [
    ("Blue", 0xFF1473E6),
    ("Yellow", 0xFFFFCA28),
    ("Red", 0xFFD93830),
    ("Green", 0xFF29834B),
    ("Purple", 0xFF9254CC),
    ("Black", 0xFF242424),
];

pub fn argb(color: u32) -> Hsla {
    rgba((color << 8) | (color >> 24)).into()
}

pub fn icon(name: &str) -> Svg {
    svg().path(SharedString::from(format!("icons/{name}.svg"))).size(px(19.)).flex_none().text_color(rgb(0x363636))
}

pub struct Tooltip(SharedString);

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .bg(rgb(0x333333))
            .text_color(rgb(0xFFFFFF))
            .text_xs()
            .rounded_md()
            .shadow_md()
            .child(self.0.clone())
    }
}

pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tooltip(text.clone())).into()
}

/// A flat command button: icon and/or label, hover background, selected state.
pub struct Button {
    id: ElementId,
    icon: Option<&'static str>,
    icon_color: Option<u32>,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
    selected: bool,
    disabled: bool,
    primary: bool,
    full_width: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

pub fn button(id: impl Into<ElementId>) -> Button {
    Button {
        id: id.into(),
        icon: None,
        icon_color: None,
        label: None,
        tooltip: None,
        selected: false,
        disabled: false,
        primary: false,
        full_width: false,
        on_click: None,
    }
}

impl Button {
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn icon_color(mut self, color: u32) -> Self {
        self.icon_color = Some(color);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn build(self) -> Stateful<Div> {
        let icon_only = self.label.is_none();
        let (fg, bg) = if self.primary {
            (0xFFFFFF, Some(ACCENT))
        } else if self.selected {
            (ACCENT_DARK, Some(ACCENT_SOFT))
        } else {
            (0x292929, None)
        };
        let mut el = div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .gap(px(9.))
            .h(px(32.))
            .rounded(px(if self.primary { 16. } else { 6. }))
            .text_size(px(13.))
            .text_color(rgb(fg))
            .when(icon_only, |d| d.w(px(34.)).justify_center())
            .when(!icon_only, |d| d.px(px(if self.primary { 15. } else { 9. })))
            .when(self.full_width, |d| d.w_full())
            .when_some(bg, |d, bg| d.bg(rgb(bg)));
        if self.disabled {
            el = el.opacity(0.4);
        } else {
            el = el.cursor_pointer().hover(|s| s.bg(rgb(if self.primary { ACCENT_DARK } else if self.selected { ACCENT_SOFT } else { HOVER })));
            if let Some(handler) = self.on_click {
                el = el.on_click(handler);
            }
        }
        if let Some(name) = self.icon {
            let color = if self.primary { 0xFFFFFF } else if self.selected { ACCENT_DARK } else { self.icon_color.unwrap_or(0x363636) };
            el = el.child(icon(name).text_color(rgb(color)));
        }
        if let Some(label) = self.label {
            el = el.child(div().truncate().child(label));
        }
        let tip = self.tooltip.or_else(|| icon_only.then(|| SharedString::from(""))).filter(|t| !t.is_empty());
        if let Some(text) = tip {
            el = el.tooltip(tooltip(text));
        }
        el
    }
}

impl IntoElement for Button {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        self.build()
    }
}

pub fn divider_h() -> Div {
    div().h(px(1.)).w_full().my(px(8.)).bg(rgb(0xE4E4E4))
}

pub fn divider_v() -> Div {
    div().w(px(1.)).h(px(22.)).mx(px(6.)).bg(rgb(0xE4E4E4))
}

pub fn heading(text: impl Into<SharedString>) -> Div {
    div().mt(px(17.)).mb(px(7.)).mx(px(8.)).text_size(px(11.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(0x696969)).child(text.into())
}

pub fn description(text: impl Into<SharedString>) -> Div {
    div().mx(px(8.)).mt(px(9.)).mb(px(12.)).text_size(px(12.)).line_height(px(17.)).text_color(rgb(0x757575)).child(text.into())
}

/// The field chrome around a [`crate::text_input::TextInput`].
pub fn field() -> Div {
    div()
        .flex()
        .items_center()
        .h(px(32.))
        .px(px(10.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(0xCCCCCC))
        .bg(rgb(0xFFFFFF))
        .text_size(px(13.))
        .text_color(rgb(TEXT))
}
