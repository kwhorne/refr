//! The multi-document shell: tabs, tool modes, panels, file commands and recovery.
//! Rendering of the chrome is in `shell.rs`; side panels are in `panels.rs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    AppContext, Context, Entity, EntityId, FocusHandle, Focusable, PathPromptOptions, PromptLevel, SharedString, Subscription, Task,
    UniformListScrollHandle, Window,
};
use refr_core::{Annotation, AnnotationKind, EditorSession, PdfTool, PdfWorkspace, PointD, RectD, Uuid, page_range, text_layout, workspace_json};
use refr_pdf::Engine;

use crate::document::{DocumentEvent, DocumentView};
use crate::storage;
use crate::text_input::{InputEvent, TextInput};

pub const MAX_DOCUMENTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    AllTools,
    Edit,
    Convert,
    ESign,
    Organize,
}

impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Mode::AllTools => "All tools",
            Mode::Edit => "Edit",
            Mode::Convert => "Convert",
            Mode::ESign => "E-Sign",
            Mode::Organize => "Organize pages",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Comments,
    Bookmarks,
    Thumbnails,
    Properties,
    Find,
}

impl Panel {
    pub fn title(self) -> &'static str {
        match self {
            Panel::Comments => "Comments",
            Panel::Bookmarks => "Bookmarks",
            Panel::Thumbnails => "Page thumbnails",
            Panel::Properties => "Properties",
            Panel::Find => "Find",
        }
    }
}

type DialogAction = Box<dyn FnOnce(&mut Workbench, String, &mut Window, &mut Context<Workbench>)>;

pub struct Dialog {
    pub title: SharedString,
    pub message: SharedString,
    pub input: Entity<TextInput>,
    pub accept: SharedString,
    on_accept: Option<DialogAction>,
    _subscription: Subscription,
}

pub struct Workbench {
    pub engine: Engine,
    pub documents: Vec<Entity<DocumentView>>,
    pub active: usize,
    pub home: bool,
    pub mode: Mode,
    pub left_open: bool,
    pub right: Option<Panel>,
    pub status: (SharedString, bool),
    pub find_input: Entity<TextInput>,
    pub reply_input: Entity<TextInput>,
    pub page_input: Entity<TextInput>,
    pub dialog: Option<Dialog>,
    pub match_case: bool,
    pub hide_resolved: bool,
    pub reply_to: Option<Uuid>,
    pub recents: Vec<PathBuf>,
    pub focus_handle: FocusHandle,
    pub thumb_scroll: UniformListScrollHandle,
    pub organize_scroll: UniformListScrollHandle,
    autosave: Option<Task<()>>,
    subscriptions: Vec<(EntityId, [Subscription; 2])>,
    _inputs: Vec<Subscription>,
}

impl Focusable for Workbench {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Workbench {
    pub fn new(engine: Engine, initial: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let find_input = cx.new(|cx| TextInput::new("Find in document", cx));
        let reply_input = cx.new(|cx| TextInput::new("Reply…", cx));
        let page_input = cx.new(|cx| TextInput::new("", cx));
        let inputs = vec![
            cx.subscribe_in(&find_input, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Confirm(query) => {
                    let query = query.clone();
                    this.search(&query, cx);
                }
                InputEvent::Cancel => this.focus_document(window, cx),
                _ => {}
            }),
            cx.subscribe_in(&reply_input, window, |this, input, event: &InputEvent, _, cx| {
                if let InputEvent::Confirm(text) = event
                    && let Some(id) = this.reply_to
                {
                    let text = text.clone();
                    this.with_doc(cx, |doc, cx| doc.edit(cx, |s| s.reply(id, &text)));
                    input.update(cx, |i, cx| i.set_text("", cx));
                }
            }),
            cx.subscribe_in(&page_input, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Confirm(text) => {
                    if let Ok(n) = text.trim().parse::<usize>() {
                        this.with_doc(cx, |doc, cx| doc.go_to_page(n.saturating_sub(1), cx));
                    }
                    this.focus_document(window, cx);
                }
                InputEvent::Cancel | InputEvent::Blur => cx.notify(),
                _ => {}
            }),
        ];
        let mut this = Self {
            engine,
            documents: Vec::new(),
            active: 0,
            home: false,
            mode: Mode::AllTools,
            left_open: true,
            right: None,
            status: ("All files stay on your device.".into(), false),
            find_input,
            reply_input,
            page_input,
            dialog: None,
            match_case: false,
            hide_resolved: false,
            reply_to: None,
            recents: storage::recents(),
            focus_handle: cx.focus_handle(),
            thumb_scroll: UniformListScrollHandle::new(),
            organize_scroll: UniformListScrollHandle::new(),
            autosave: None,
            subscriptions: Vec::new(),
            _inputs: inputs,
        };
        if initial.is_empty() {
            this.open_sample(window, cx);
        }
        for path in initial {
            this.open_path(path, false, window, cx);
        }
        this.offer_recovery(window, cx);
        this
    }

    // ---- Status -------------------------------------------------------------------------

    pub fn set_status(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.status = (text.into(), false);
        cx.notify();
    }

    pub fn error(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.status = (text.into(), true);
        cx.notify();
    }

    // ---- Documents ----------------------------------------------------------------------

    pub fn doc(&self) -> Option<&Entity<DocumentView>> {
        self.documents.get(self.active)
    }

    pub fn with_doc<R>(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut DocumentView, &mut Context<DocumentView>) -> R) -> Option<R> {
        let doc = self.doc()?.clone();
        Some(doc.update(cx, f))
    }

    pub fn workspace(&self, cx: &gpui::App) -> Option<Arc<PdfWorkspace>> {
        self.doc().map(|d| d.read(cx).doc().clone())
    }

    pub fn add_document(&mut self, workspace: PdfWorkspace, path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        if self.documents.len() >= MAX_DOCUMENTS {
            self.error("Close a document before opening another. Refr keeps up to eight documents open.", cx);
            return;
        }
        let session = match EditorSession::new(workspace) {
            Ok(session) => session,
            Err(error) => return self.error(error.to_string(), cx),
        };
        let engine = self.engine.clone();
        let doc = cx.new(|cx| {
            let mut doc = DocumentView::new(session, engine, cx);
            doc.path = path.filter(|p| storage::is_workspace(p));
            doc
        });
        let subscriptions = [cx.subscribe_in(&doc, window, Self::on_document_event), cx.observe(&doc, |_, _, cx| cx.notify())];
        self.subscriptions.push((doc.entity_id(), subscriptions));
        self.documents.push(doc);
        self.activate(self.documents.len() - 1, window, cx);
    }

    pub fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.doc().cloned() {
            previous.update(cx, |d, cx| d.finish_text(true, cx));
        }
        self.active = index.min(self.documents.len().saturating_sub(1));
        self.home = false;
        self.reply_to = None;
        if self.mode == Mode::Organize {
            self.mode = Mode::AllTools;
        }
        self.focus_document(window, cx);
        cx.notify();
    }

    pub fn focus_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(doc) = self.doc() {
            window.focus(&doc.read(cx).focus_handle);
        }
        cx.notify();
    }

    pub fn close_document(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.documents.get(index).cloned() else { return };
        if doc.read(cx).session.is_dirty() {
            let answer = window.prompt(
                PromptLevel::Warning,
                "Close document?",
                Some("Unsaved workspace changes will be closed. Save an editable .pdfspace workspace to keep them."),
                &["Close Document", "Cancel"],
                cx,
            );
            cx.spawn_in(window, async move |this, cx| {
                if answer.await == Ok(0) {
                    this.update_in(cx, |this, window, cx| this.remove_document(&doc, window, cx)).ok();
                }
            })
            .detach();
        } else {
            self.remove_document(&doc, window, cx);
        }
    }

    fn remove_document(&mut self, doc: &Entity<DocumentView>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.documents.iter().position(|d| d == doc) else { return };
        doc.update(cx, |d, cx| d.release_images(cx));
        self.documents.remove(index);
        self.subscriptions.retain(|(id, _)| *id != doc.entity_id());
        if self.documents.is_empty() {
            self.add_document(PdfWorkspace::default(), None, window, cx);
        } else {
            let next = if index <= self.active { self.active.saturating_sub(1) } else { self.active };
            self.activate(next.min(self.documents.len() - 1), window, cx);
        }
    }

    fn on_document_event(&mut self, doc: &Entity<DocumentView>, event: &DocumentEvent, window: &mut Window, cx: &mut Context<Self>) {
        let is_active = self.doc() == Some(doc);
        match event {
            DocumentEvent::Changed => {
                if is_active {
                    self.schedule_recovery(cx);
                }
                cx.notify();
            }
            DocumentEvent::Status(text, error) => {
                self.status = (text.clone().into(), *error);
                cx.notify();
            }
            DocumentEvent::NoteRequested { page, point } => {
                let (page, point) = (*page, *point);
                let doc = doc.clone();
                self.prompt("Add a comment", "Share a thought, question or suggested change.", "", "Post", window, cx, move |this, text, _, cx| {
                    if text.trim().is_empty() {
                        return;
                    }
                    doc.update(cx, |d, cx| {
                        let a = Annotation { color: d.session.color, text: text.trim().into(), ..Annotation::new(AnnotationKind::Note, RectD::new(point.x - 11.0, point.y - 11.0, 23.0, 23.0)) };
                        d.edit(cx, |s| s.add_annotation(a, Some(page)));
                    });
                    this.right = Some(Panel::Comments);
                    cx.notify();
                });
            }
            DocumentEvent::EditNote(id) => self.edit_annotation_text(*id, window, cx),
        }
    }

    pub fn edit_annotation_text(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.doc().cloned() else { return };
        let Some(a) = doc.read(cx).doc().pages.iter().find_map(|p| p.annotation(id)).cloned() else { return };
        match a.kind {
            AnnotationKind::Text => {
                let page = doc.read(cx).doc().pages.iter().position(|p| p.annotation(id).is_some()).unwrap_or(0);
                doc.update(cx, |d, cx| d.start_text(page, PointD::new(a.bounds.x, a.bounds.y), Some(id), window, cx));
            }
            _ => {
                let (title, label) = if a.kind == AnnotationKind::Stamp { ("Edit stamp", "Stamp text") } else { ("Edit comment", "Change the comment text.") };
                self.prompt(title, label, &a.text, "Save", window, cx, move |_, text, _, cx| {
                    doc.update(cx, |d, cx| d.edit(cx, |s| s.update_annotation(id, "Edit comment", |a| Annotation { text: text.trim().into(), ..a.clone() })));
                });
            }
        }
    }

    // ---- Dialog ---------------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn prompt(
        &mut self,
        title: &str,
        message: &str,
        initial: &str,
        accept: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        on_accept: impl FnOnce(&mut Workbench, String, &mut Window, &mut Context<Workbench>) + 'static,
    ) {
        let input = cx.new(|cx| {
            let mut input = TextInput::new("", cx);
            input.set_text(initial.to_string(), cx);
            input.select_all_text(cx);
            input
        });
        let subscription = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::Confirm(_) => this.accept_dialog(window, cx),
            InputEvent::Cancel => this.cancel_dialog(window, cx),
            _ => {}
        });
        input.read(cx).focus(window);
        self.dialog = Some(Dialog { title: title.to_string().into(), message: message.to_string().into(), input, accept: accept.to_string().into(), on_accept: Some(Box::new(on_accept)), _subscription: subscription });
        cx.notify();
    }

    pub fn accept_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let text = dialog.input.read(cx).text().to_string();
        if let Some(action) = dialog.on_accept.take() {
            action(self, text, window, cx);
        }
        if self.dialog.is_none() {
            self.focus_document(window, cx);
        }
        cx.notify();
    }

    pub fn cancel_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        self.focus_document(window, cx);
    }

    // ---- Recovery -------------------------------------------------------------------------

    fn schedule_recovery(&mut self, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace(cx) else { return };
        let background = cx.background_executor().clone();
        self.autosave = Some(cx.spawn(async move |this, cx| {
            background.timer(Duration::from_millis(1200)).await;
            let result = background.spawn(async move { storage::write_recovery(&workspace_json::save(&workspace)) }).await;
            this.update(cx, |this, cx| match result {
                Ok(()) => this.set_status("Recovery copy saved on this device. Save a workspace for a permanent copy.", cx),
                Err(error) => this.error(format!("Recovery could not be saved: {error}. Save your workspace now."), cx),
            })
            .ok();
        }));
    }

    fn offer_recovery(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let task = cx.background_executor().spawn(async { storage::read_recovery() });
        cx.spawn_in(window, async move |this, cx| {
            let Some(workspace) = task.await else { return };
            let Ok(answer) = this.update_in(cx, |_, window, cx| {
                window.prompt(
                    PromptLevel::Info,
                    "Restore your previous workspace?",
                    Some("A recovery copy is available on this device. Your source PDF has not been changed."),
                    &["Restore", "Not Now"],
                    cx,
                )
            }) else {
                return;
            };
            if answer.await == Ok(0) {
                this.update_in(cx, |this, window, cx| this.add_document(workspace, None, window, cx)).ok();
            }
        })
        .detach();
    }

    // ---- Opening --------------------------------------------------------------------------

    pub fn open_dialog(&mut self, combine: bool, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(if combine { "Combine".into() } else { "Open".into() }),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            this.update_in(cx, |this, window, cx| {
                for path in paths {
                    this.open_path(path, combine, window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn open_path(&mut self, path: PathBuf, combine: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !storage::is_openable(&path) {
            return self.error(format!("{} is not a PDF or .pdfspace file.", path.display()), cx);
        }
        let engine = self.engine.clone();
        let load_path = path.clone();
        let task = cx.background_executor().spawn(async move { storage::load(&engine, &load_path) });
        self.set_status(format!("Opening {}…", path.file_name().and_then(|n| n.to_str()).unwrap_or("file")), cx);
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(workspace) => {
                    this.recents = storage::remember(&path);
                    if combine {
                        let pages = workspace.pages.len();
                        this.with_doc(cx, |d, cx| d.edit(cx, |s| s.combine(&workspace)));
                        this.set_status(format!("Added {pages} page{} from {}.", if pages == 1 { "" } else { "s" }, workspace.title), cx);
                    } else {
                        let title = workspace.title.clone();
                        this.add_document(workspace, Some(path.clone()), window, cx);
                        this.set_status(format!("Opened {title}. All files stay on your device."), cx);
                    }
                }
                Err(error) => this.error(error, cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn open_sample(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.engine.sample() {
            Ok(sample) => self.add_document(sample, None, window, cx),
            Err(error) => self.error(format!("The sample document could not be created: {error}"), cx),
        }
    }

    pub fn new_blank(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_document(PdfWorkspace::default(), None, window, cx);
        self.set_status("Created a blank PDF. Add text, drawings and pages, then export.", cx);
    }

    // ---- Saving and exporting ----------------------------------------------------------------

    /// Asks where to save, then produces and writes the bytes off the main thread.
    fn save_as(
        &mut self,
        suggested: String,
        produce: impl FnOnce() -> Result<Vec<u8>, String> + Send + 'static,
        done: impl FnOnce(&mut Workbench, PathBuf, &mut Context<Workbench>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = cx.prompt_for_new_path(&storage::documents_dir(), Some(&suggested));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = target.await else { return };
            this.update(cx, |this, cx| this.set_status(format!("Saving {}…", path.display()), cx)).ok();
            let write_path = path.clone();
            let result = cx.background_executor().spawn(async move { produce().and_then(|bytes| storage::write_atomic(&write_path, &bytes).map_err(|e| e.to_string())) }).await;
            this.update(cx, |this, cx| match result {
                Ok(()) => done(this, path, cx),
                Err(error) => this.error(error, cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn save_workspace(&mut self, force_prompt: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.doc().cloned() else { return };
        doc.update(cx, |d, cx| d.finish_text(true, cx));
        let workspace = doc.read(cx).doc().clone();
        let existing = doc.read(cx).path.clone().filter(|_| !force_prompt);
        let mark_saved = {
            let doc = doc.clone();
            move |this: &mut Workbench, path: PathBuf, cx: &mut Context<Workbench>| {
                doc.update(cx, |d, cx| {
                    d.session.mark_saved();
                    d.path = Some(path.clone());
                    cx.notify();
                });
                this.recents = storage::remember(&path);
                this.set_status(format!("Saved editable workspace to {}. It contains the full original PDF.", path.display()), cx);
            }
        };
        if let Some(path) = existing {
            let json = workspace_json::save(&workspace);
            match storage::write_atomic(&path, json.as_bytes()) {
                Ok(()) => mark_saved(self, path, cx),
                Err(error) => self.error(error.to_string(), cx),
            }
            return;
        }
        let suggested = format!("{}.{}", storage::stem(&workspace.title), storage::WORKSPACE_EXTENSION);
        self.save_as(suggested, move || Ok(workspace_json::save(&workspace).into_bytes()), mark_saved, window, cx);
    }

    pub fn export_pdf(&mut self, pages: Option<Vec<usize>>, window: &mut Window, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| d.finish_text(true, cx));
        let Some(workspace) = self.workspace(cx) else { return };
        let engine = self.engine.clone();
        let suffix = if pages.is_some() { "-extract" } else { "" };
        let suggested = format!("{}{suffix}.pdf", storage::stem(&workspace.title));
        self.save_as(
            suggested,
            move || engine.export_pdf(&workspace, pages).map_err(|e| e.to_string()),
            |this, path, cx| this.set_status(format!("Exported PDF to {}. Keep the .pdfspace workspace to edit annotations later.", path.display()), cx),
            window,
            cx,
        );
    }

    pub fn export_png(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.doc() else { return };
        let (workspace, page) = { let d = doc.read(cx); (d.doc().clone(), d.session.current_page()) };
        let engine = self.engine.clone();
        let suggested = format!("{}-page-{}.png", storage::stem(&workspace.title), page + 1);
        self.save_as(suggested, move || engine.export_png(&workspace, page, 2.0).map_err(|e| e.to_string()), |this, path, cx| this.set_status(format!("Saved page image to {}.", path.display()), cx), window, cx);
    }

    pub fn export_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace(cx) else { return };
        let engine = self.engine.clone();
        let suggested = format!("{}.txt", storage::stem(&workspace.title));
        self.save_as(suggested, move || engine.extract_text(&workspace).map(String::into_bytes).map_err(|e| e.to_string()), |this, path, cx| this.set_status(format!("Saved extracted text to {}.", path.display()), cx), window, cx);
    }

    pub fn split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace(cx) else { return };
        let engine = self.engine.clone();
        let suggested = format!("{}-pages.zip", storage::stem(&workspace.title));
        self.save_as(suggested, move || engine.split(&workspace).map_err(|e| e.to_string()), |this, path, cx| this.set_status(format!("Saved single-page PDFs to {}.", path.display()), cx), window, cx);
    }

    pub fn extract_pages(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace(cx) else { return };
        let count = workspace.pages.len();
        let current = self.doc().map(|d| d.read(cx).session.current_page() + 1).unwrap_or(1);
        self.prompt("Extract pages", &format!("Pages to extract, for example 1, 3-5 (1–{count})."), &current.to_string(), "Extract", window, cx, move |this, text, window, cx| {
            match page_range::parse(&text, count) {
                Ok(pages) => this.export_pdf(Some(pages), window, cx),
                Err(error) => this.error(error, cx),
            }
        });
    }

    pub fn print(&mut self, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| d.finish_text(true, cx));
        let Some(workspace) = self.workspace(cx) else { return };
        let engine = self.engine.clone();
        let dir = std::env::temp_dir().join("Refr");
        let path = dir.join(format!("{}-print.pdf", storage::stem(&workspace.title)));
        let task = cx.background_executor().spawn({
            let path = path.clone();
            async move {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let bytes = engine.export_pdf(&workspace, None).map_err(|e| e.to_string())?;
                std::fs::write(&path, bytes).map_err(|e| e.to_string())
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| match result {
                Ok(()) => {
                    cx.open_with_system(&path);
                    this.set_status("Opened a printable PDF in your PDF viewer. Print from there with ⌘P.", cx);
                }
                Err(error) => this.error(error, cx),
            })
            .ok();
        })
        .detach();
    }

    // ---- Document tools ---------------------------------------------------------------------

    pub fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        let Some(doc) = self.doc().cloned() else { return };
        let workspace = doc.read(cx).doc().clone();
        let (engine, query, match_case) = (self.engine.clone(), query.trim().to_string(), self.match_case);
        self.right = Some(Panel::Find);
        if query.is_empty() {
            doc.update(cx, |d, cx| {
                d.search.clear();
                d.active_search = None;
                cx.notify();
            });
            return;
        }
        let search_query = query.clone();
        let task = cx.background_executor().spawn(async move { engine.find(&workspace, &search_query, match_case) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| match result {
                Ok(results) => {
                    let count = results.len();
                    doc.update(cx, |d, cx| {
                        d.search = results;
                        d.active_search = None;
                        if count > 0 {
                            d.active_search = Some(0);
                            let first = d.search[0].clone();
                            d.reveal_rect(first.page_index, first.bounds[0], cx);
                        }
                        cx.notify();
                    });
                    this.set_status(if count == 0 { format!("No matches for “{query}”.") } else { format!("{count} match{} for “{query}”.", if count == 1 { "" } else { "es" }) }, cx);
                }
                Err(error) => this.error(error.to_string(), cx),
            })
            .ok();
        })
        .detach();
    }

    pub fn show_search_result(&mut self, index: usize, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| {
            let Some(result) = d.search.get(index).cloned() else { return };
            d.active_search = Some(index);
            d.reveal_rect(result.page_index, result.bounds[0], cx);
            if let Some(id) = result.annotation {
                d.session.reveal(id);
            }
        });
    }

    pub fn use_tool(&mut self, tool: PdfTool, window: &mut Window, cx: &mut Context<Self>) {
        if self.mode == Mode::Organize {
            self.mode = Mode::Edit;
        }
        self.home = false;
        self.with_doc(cx, |d, cx| d.set_tool(tool, cx));
        self.focus_document(window, cx);
    }

    pub fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        self.mode = mode;
        self.home = false;
        self.left_open = true;
        cx.notify();
    }

    pub fn toggle_panel(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.right = if self.right == Some(panel) { None } else { Some(panel) };
        if self.right == Some(Panel::Thumbnails)
            && let Some(page) = self.doc().map(|d| d.read(cx).session.current_page())
        {
            self.reveal_thumbnail(page);
        }
        cx.notify();
    }

    pub fn add_watermark(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt("Add watermark", "Text placed across the middle of every page.", "DRAFT", "Add", window, cx, |this, text, _, cx| {
            let text = text.trim().to_string();
            if text.is_empty() {
                return;
            }
            this.with_doc(cx, |d, cx| {
                d.edit(cx, |s| {
                    s.add_to_pages("Add watermark", |_, page| {
                        let size = (page.width / (text.chars().count().max(1) as f64 * 0.62)).clamp(24.0, 110.0);
                        let width = text_layout::measure(&text, size) + 4.0;
                        let height = text_layout::height(1, size);
                        let b = page.visible_box();
                        let bounds = RectD::new(b.x + (b.width - width) / 2.0, b.y + (b.height - height) / 2.0, width, height);
                        Some(Annotation { color: 0x55999999, font_size: size, text: text.clone(), ..Annotation::new(AnnotationKind::Text, bounds) })
                    })
                })
            });
            this.set_status("Added a watermark to every page. It is a visual mark, not a security feature.", cx);
        });
    }

    pub fn add_page_numbers(&mut self, cx: &mut Context<Self>) {
        self.with_doc(cx, |d, cx| {
            let total = d.doc().pages.len();
            d.edit(cx, |s| {
                s.add_to_pages("Add page numbers", |i, page| {
                    let text = format!("{} / {total}", i + 1);
                    let size = 10.0;
                    let width = text_layout::measure(&text, size) + 4.0;
                    let b = page.visible_box();
                    let bounds = RectD::new(b.x + (b.width - width) / 2.0, b.bottom() - 30.0, width, text_layout::height(1, size));
                    Some(Annotation { color: 0xFF555555, font_size: size, text, ..Annotation::new(AnnotationKind::Text, bounds) })
                })
            });
        });
        self.set_status("Added page numbers.", cx);
    }

    pub fn add_initials(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt("Add initials", "Your initials, placed on the current page. Drag them into position.", "", "Add", window, cx, |this, text, window, cx| {
            let text = text.trim().to_uppercase();
            if text.is_empty() {
                return;
            }
            this.with_doc(cx, |d, cx| {
                let page = d.session.page().clone();
                let size = 18.0;
                let width = text_layout::measure(&text, size) + 6.0;
                let b = page.visible_box();
                let bounds = RectD::new(b.right() - width - 48.0, b.bottom() - 90.0, width, text_layout::height(1, size));
                let a = Annotation { color: d.session.color, font_size: size, text, ..Annotation::new(AnnotationKind::Text, bounds) };
                let index = d.session.current_page();
                d.edit(cx, |s| s.add_annotation(a, Some(index)));
                d.set_tool(PdfTool::Select, cx);
            });
            this.focus_document(window, cx);
        });
    }

    pub fn edit_bookmark(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace(cx) else { return };
        let current = workspace.pages.get(index).map(|p| p.bookmark.clone()).unwrap_or_default();
        let initial = if current.is_empty() { format!("Page {}", index + 1) } else { current };
        self.prompt("Bookmark page", "Name this page. Leave empty to remove the bookmark.", &initial, "Save", window, cx, move |this, text, _, cx| {
            this.with_doc(cx, |d, cx| d.edit(cx, |s| s.bookmark_page(index, &text)));
        });
    }

    pub fn set_zoom_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.doc().map(|d| (d.read(cx).zoom * 100.0).round()).unwrap_or(100.0);
        self.prompt("Zoom", "Zoom level in percent (10–800).", &current.to_string(), "Zoom", window, cx, |this, text, _, cx| {
            if let Ok(value) = text.trim().trim_end_matches('%').parse::<f64>() {
                this.with_doc(cx, |d, cx| d.zoom_to(value / 100.0, None, cx));
            }
        });
    }

    pub fn edit_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace(cx) else { return };
        let author = workspace.author.clone();
        self.prompt("Document title", "The title shown on the tab and used for exported file names.", &workspace.title, "Next", window, cx, move |this, title, window, cx| {
            this.prompt("Author", "Stored in the workspace.", &author, "Save", window, cx, move |this, author, _, cx| {
                this.with_doc(cx, |d, cx| d.edit(cx, |s| s.set_info(&title, &author)));
            });
        });
    }

    pub fn delete_pages(&mut self, pages: Vec<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let count = pages.len();
        let answer = window.prompt(
            PromptLevel::Warning,
            if count == 1 { "Delete this page?" } else { "Delete these pages?" },
            Some("You can undo this with ⌘Z."),
            &["Delete", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                this.update(cx, |this, cx| {
                    this.with_doc(cx, |d, cx| d.edit(cx, |s| s.delete_pages(&pages)));
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn show_help(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let detail = "⌘O open · ⌘N new blank PDF · ⌘S save workspace · ⇧⌘S export PDF · ⌘P print\n\
            ⌘F find · ⌘Z / ⇧⌘Z undo and redo · ⌘C copy selected text\n\
            ⌘0 fit page · ⌘1 actual size · ⌘+ / ⌘− zoom · ⌘-scroll zooms at the pointer\n\
            V select · H hand · T text · D draw · Page Up / Page Down change page\n\
            Delete removes the selected annotation · arrow keys nudge it · Esc cancels\n\n\
            Cropping is not redaction. A drawn signature is a visual mark, not a certificate-based digital signature.";
        let _ = window.prompt(PromptLevel::Info, "Keyboard and pointer guide", Some(detail), &["OK"], cx);
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string()
}
