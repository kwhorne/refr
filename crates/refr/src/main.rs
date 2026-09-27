//! Refr: a local-first PDF workspace for macOS, built on GPUI and PDFium.

mod about;
mod actions;
mod assets;
mod document;
mod glyphs;
mod panels;
mod shell;
mod storage;
mod text_input;
mod theme;
mod viewport;
mod workbench;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use gpui::{
    App, Application, Bounds, KeyBinding, Menu, MenuItem, OsAction, PromptLevel, SystemMenuType, TitlebarOptions, WindowBounds,
    WindowOptions, point, prelude::*, px, size,
};
use refr_pdf::Engine;

use actions::*;
use workbench::Workbench;

/// Viewport shortcuts, except while typing in a text field inside the viewport
/// (the inline annotation editor), where single letters must reach the field.
const VIEWPORT: &str = "Viewport && !TextInput";

fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-n", NewBlank, None),
        KeyBinding::new("cmd-s", SaveWorkspace, None),
        KeyBinding::new("alt-cmd-s", SaveWorkspaceAs, None),
        KeyBinding::new("cmd-shift-s", ExportPdf, None),
        KeyBinding::new("cmd-p", Print, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-shift-]", NextTab, None),
        KeyBinding::new("cmd-shift-[", PreviousTab, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("ctrl-y", Redo, None),
        KeyBinding::new("cmd-0", FitPage, None),
        KeyBinding::new("cmd-1", ActualSize, None),
        KeyBinding::new("cmd-2", FitWidth, None),
        KeyBinding::new("cmd-=", ZoomIn, None),
        KeyBinding::new("cmd-+", ZoomIn, None),
        KeyBinding::new("cmd--", ZoomOut, None),
        KeyBinding::new("cmd-shift-h", ShowHome, None),
        KeyBinding::new("cmd-/", ShowHelp, None),
        KeyBinding::new("escape", CloseModal, Some("Modal")),
        // Viewport
        KeyBinding::new("pageup", PreviousPage, Some(VIEWPORT)),
        KeyBinding::new("pagedown", NextPage, Some(VIEWPORT)),
        KeyBinding::new("home", FirstPage, Some(VIEWPORT)),
        KeyBinding::new("end", LastPage, Some(VIEWPORT)),
        KeyBinding::new("v", ToolSelect, Some(VIEWPORT)),
        KeyBinding::new("h", ToolHand, Some(VIEWPORT)),
        KeyBinding::new("t", ToolText, Some(VIEWPORT)),
        KeyBinding::new("d", ToolInk, Some(VIEWPORT)),
        KeyBinding::new("backspace", DeleteSelection, Some(VIEWPORT)),
        KeyBinding::new("delete", DeleteSelection, Some(VIEWPORT)),
        KeyBinding::new("left", NudgeLeft, Some(VIEWPORT)),
        KeyBinding::new("right", NudgeRight, Some(VIEWPORT)),
        KeyBinding::new("up", NudgeUp, Some(VIEWPORT)),
        KeyBinding::new("down", NudgeDown, Some(VIEWPORT)),
        KeyBinding::new("shift-left", NudgeLeftFar, Some(VIEWPORT)),
        KeyBinding::new("shift-right", NudgeRightFar, Some(VIEWPORT)),
        KeyBinding::new("shift-up", NudgeUpFar, Some(VIEWPORT)),
        KeyBinding::new("shift-down", NudgeDownFar, Some(VIEWPORT)),
        KeyBinding::new("escape", CancelGesture, Some(VIEWPORT)),
        KeyBinding::new("cmd-c", Copy, Some(VIEWPORT)),
        // Text input
        KeyBinding::new("backspace", Backspace, Some("TextInput")),
        KeyBinding::new("delete", Delete, Some("TextInput")),
        KeyBinding::new("left", Left, Some("TextInput")),
        KeyBinding::new("right", Right, Some("TextInput")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
        KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
        KeyBinding::new("cmd-a", SelectAll, Some("TextInput")),
        KeyBinding::new("home", Home, Some("TextInput")),
        KeyBinding::new("end", End, Some("TextInput")),
        KeyBinding::new("cmd-left", Home, Some("TextInput")),
        KeyBinding::new("cmd-right", End, Some("TextInput")),
        KeyBinding::new("enter", Confirm, Some("TextInput")),
        KeyBinding::new("escape", Cancel, Some("TextInput")),
        KeyBinding::new("cmd-c", Copy, Some("TextInput")),
        KeyBinding::new("cmd-x", Cut, Some("TextInput")),
        KeyBinding::new("cmd-v", Paste, Some("TextInput")),
    ]
}

fn menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Refr".into(),
            items: vec![
                MenuItem::action("About Refr", ShowAbout),
                MenuItem::separator(),
                MenuItem::action("Keyboard and Pointer Guide", ShowHelp),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Refr", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Blank PDF", NewBlank),
                MenuItem::action("Open…", OpenFile),
                MenuItem::action("Open Sample Document", OpenSample),
                MenuItem::separator(),
                MenuItem::action("Save Workspace", SaveWorkspace),
                MenuItem::action("Save Workspace As…", SaveWorkspaceAs),
                MenuItem::action("Export PDF…", ExportPdf),
                MenuItem::action("Print…", Print),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action("Find…", Find),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Home", ShowHome),
                MenuItem::separator(),
                MenuItem::action("Fit Page", FitPage),
                MenuItem::action("Fit Width", FitWidth),
                MenuItem::action("Actual Size", ActualSize),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::separator(),
                MenuItem::action("Previous Page", PreviousPage),
                MenuItem::action("Next Page", NextPage),
                MenuItem::separator(),
                MenuItem::action("Next Tab", NextTab),
                MenuItem::action("Previous Tab", PreviousTab),
            ],
        },
    ]
}

fn main() {
    let initial: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).filter(|p| p.is_file()).collect();
    let engine = Engine::locate_library()
        .ok_or_else(|| "libpdfium.dylib was not found. Run scripts/fetch-pdfium.sh, or set REFR_PDFIUM.".to_string())
        .and_then(|path| Engine::start(path).map_err(|e| e.to_string()));

    let application = Application::new().with_assets(assets::Assets);
    let (opened_tx, opened_rx) = std::sync::mpsc::channel::<Vec<String>>();
    application.on_open_urls(move |urls| {
        opened_tx.send(urls).ok();
    });
    application.run(move |cx: &mut App| {
        cx.bind_keys(key_bindings());
        cx.set_menus(menus());
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1360.), px(880.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some("Refr".into()), appears_transparent: true, traffic_light_position: Some(point(px(14.), px(16.))) }),
            window_min_size: Some(size(px(720.), px(480.))),
            ..Default::default()
        };
        match engine {
            Ok(engine) => {
                let window = cx
                    .open_window(options, |window, cx| cx.new(|cx| Workbench::new(engine, initial, storage::data_dir(), window, cx)))
                    .expect("the main window opens");
                // Files opened from Finder while running.
                cx.spawn(async move |cx| {
                    loop {
                        cx.background_executor().timer(std::time::Duration::from_millis(250)).await;
                        while let Ok(urls) = opened_rx.try_recv() {
                            let paths: Vec<PathBuf> = urls.iter().filter_map(|u| u.strip_prefix("file://")).map(|p| PathBuf::from(percent_decode(p))).collect();
                            window
                                .update(cx, |workbench, window, cx| {
                                    for path in paths.clone() {
                                        workbench.open_path(path, false, window, cx);
                                    }
                                })
                                .ok();
                        }
                    }
                })
                .detach();
            }
            Err(message) => {
                let window = cx.open_window(options, |_, cx| cx.new(|_| gpui::Empty)).expect("a window opens");
                window
                    .update(cx, |_, window, cx| {
                        let answer = window.prompt(PromptLevel::Critical, "Refr could not start its PDF engine", Some(&message), &["Quit"], cx);
                        cx.spawn(async move |_, cx| {
                            answer.await.ok();
                            cx.update(|cx| cx.quit()).ok();
                        })
                        .detach();
                    })
                    .ok();
            }
        }
        cx.activate(true);
    });
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&text[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
