//! Icons compiled into the binary, served to GPUI's `svg()` element.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[
            $((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../../../assets/icons/", $name, ".svg")))),*
        ];
    };
}

icons!(
    "menu", "home", "file", "folder", "close", "plus", "search", "save", "print", "share", "info", "settings", "edit",
    "export", "pages", "comment", "pen", "text", "sign", "check", "arrow", "line", "rectangle", "ellipse", "hand",
    "select", "highlight", "underline", "strikeout", "crop", "rotate", "zoom_in", "zoom_out", "fit_page", "fit_width",
    "up", "down", "left", "right", "undo", "redo", "trash", "copy", "bookmark", "lock", "redact", "measure", "image",
    "download", "attachment", "more", "help", "star", "grid",
);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS.iter().find(|(p, _)| *p == path).map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| SharedString::from(*p)).collect())
    }
}
