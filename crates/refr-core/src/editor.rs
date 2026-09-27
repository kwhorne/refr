//! Transactional editing with bounded undo/redo.
//!
//! Every document change goes through [`EditorSession::execute`], which validates the new
//! snapshot and records one named history entry. Selection, tool and current page are view
//! state and never enter history. Source bytes are shared between snapshots, not copied.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::geometry::{PointD, RectD};
use crate::model::{Annotation, CommentReply, DEFAULT_COLOR, PdfPageState, PdfWorkspace};
use crate::workspace_json::{self, WorkspaceError};

pub const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PdfTool {
    #[default]
    Select,
    Hand,
    Highlight,
    Underline,
    Strikeout,
    Ink,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Text,
    Note,
    Signature,
    Stamp,
    Check,
    Crop,
}

impl PdfTool {
    pub fn is_text_markup(self) -> bool {
        matches!(self, Self::Highlight | Self::Underline | Self::Strikeout)
    }
}

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub label: String,
    pub before: Arc<PdfWorkspace>,
    pub after: Arc<PdfWorkspace>,
}

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error("{0}")]
    Rejected(String),
}

pub struct EditorSession {
    document: Arc<PdfWorkspace>,
    saved: Arc<PdfWorkspace>,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    current_page: usize,
    selected: Option<Uuid>,
    tool: PdfTool,
    revision: u64,
    pub color: u32,
    pub stroke_width: f64,
    pub font_size: f64,
}

impl EditorSession {
    pub fn new(document: PdfWorkspace) -> Result<Self, EditError> {
        workspace_json::validate(&document)?;
        let document = Arc::new(document);
        Ok(Self {
            saved: document.clone(),
            document,
            undo: Vec::new(),
            redo: Vec::new(),
            current_page: 0,
            selected: None,
            tool: PdfTool::Select,
            revision: 0,
            color: DEFAULT_COLOR,
            stroke_width: 2.0,
            font_size: 14.0,
        })
    }

    pub fn document(&self) -> &Arc<PdfWorkspace> {
        &self.document
    }

    /// Increments on every document change, including undo and redo.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        !Arc::ptr_eq(&self.saved, &self.document)
    }

    pub fn mark_saved(&mut self) {
        self.saved = self.document.clone();
    }

    /// Marks the current document as unsaved, for one that came from a recovery copy
    /// rather than a saved file. No undo step leads back to a clean state.
    pub fn mark_unsaved(&mut self) {
        self.saved = Arc::new((*self.document).clone());
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }

    pub fn current_page(&self) -> usize {
        self.current_page
    }

    pub fn page(&self) -> &PdfPageState {
        &self.document.pages[self.current_page]
    }

    pub fn tool(&self) -> PdfTool {
        self.tool
    }

    pub fn selected_id(&self) -> Option<Uuid> {
        self.selected
    }

    pub fn selected_annotation(&self) -> Option<&Annotation> {
        self.selected.and_then(|id| self.page().annotation(id))
    }

    pub fn set_tool(&mut self, tool: PdfTool) {
        self.tool = tool;
        self.selected = None;
    }

    pub fn navigate(&mut self, index: usize) {
        let index = index.min(self.document.pages.len() - 1);
        if index != self.current_page {
            self.selected = None;
        }
        self.current_page = index;
    }

    /// Selects an annotation on the current page, or clears the selection.
    pub fn select(&mut self, id: Option<Uuid>) {
        self.selected = id.filter(|id| self.page().annotation(*id).is_some());
    }

    /// Navigates to the page holding the annotation, then selects it.
    pub fn reveal(&mut self, id: Uuid) {
        if let Some(index) = self.document.pages.iter().position(|p| p.annotation(id).is_some()) {
            self.current_page = index;
            self.selected = Some(id);
        }
    }

    pub fn execute(&mut self, label: impl Into<String>, command: impl FnOnce(&PdfWorkspace) -> PdfWorkspace) -> Result<(), EditError> {
        let after = command(&self.document);
        if after == *self.document {
            return Ok(());
        }
        workspace_json::validate(&after)?;
        let after = Arc::new(after);
        self.undo.push(HistoryEntry { label: label.into(), before: self.document.clone(), after: after.clone() });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.document = after;
        self.settle();
        Ok(())
    }

    fn settle(&mut self) {
        self.current_page = self.current_page.min(self.document.pages.len() - 1);
        if self.selected.is_some_and(|id| self.page().annotation(id).is_none()) {
            self.selected = None;
        }
        self.revision += 1;
    }

    pub fn undo(&mut self) -> Option<String> {
        let entry = self.undo.pop()?;
        self.document = entry.before.clone();
        let label = entry.label.clone();
        self.redo.push(entry);
        self.settle();
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let entry = self.redo.pop()?;
        self.document = entry.after.clone();
        let label = entry.label.clone();
        self.undo.push(entry);
        self.settle();
        Some(label)
    }

    fn page_at(&self, index: usize) -> Result<&PdfPageState, EditError> {
        self.document.pages.get(index).ok_or_else(|| EditError::Rejected("That page does not exist.".into()))
    }

    pub fn add_annotation(&mut self, annotation: Annotation, page_index: Option<usize>) -> Result<(), EditError> {
        let index = page_index.unwrap_or(self.current_page);
        let page_id = self.page_at(index)?.id;
        let id = annotation.id;
        let label = format!("Add {}", annotation.kind.label());
        self.execute(label, |doc| {
            doc.update_page(page_id, |p| {
                let mut p = p.clone();
                p.annotations.push(annotation);
                p
            })
        })?;
        self.current_page = index;
        self.selected = Some(id);
        Ok(())
    }

    /// Adds annotations to several pages as one history entry (watermarks, page numbers).
    pub fn add_to_pages(&mut self, label: &str, make: impl Fn(usize, &PdfPageState) -> Option<Annotation>) -> Result<(), EditError> {
        self.execute(label, |doc| {
            let mut next = doc.clone();
            for (i, page) in next.pages.iter_mut().enumerate() {
                if let Some(annotation) = make(i, page) {
                    page.annotations.push(annotation);
                }
            }
            next
        })
    }

    /// Replaces an annotation anywhere in the document; the update must keep its identity.
    pub fn update_annotation(&mut self, id: Uuid, label: &str, update: impl FnOnce(&Annotation) -> Annotation) -> Result<(), EditError> {
        let Some(page_index) = self.document.pages.iter().position(|p| p.annotation(id).is_some()) else {
            return Ok(());
        };
        let page = &self.document.pages[page_index];
        let existing = page.annotation(id).expect("found above");
        let replacement = update(existing);
        if replacement == *existing {
            return Ok(());
        }
        if replacement.id != id {
            return Err(EditError::Rejected("An annotation update cannot change its identity.".into()));
        }
        let page_id = page.id;
        self.execute(label, |doc| {
            doc.update_page(page_id, |p| {
                let mut p = p.clone();
                for a in &mut p.annotations {
                    if a.id == id {
                        *a = replacement.clone();
                    }
                }
                p
            })
        })
    }

    pub fn delete_annotation(&mut self, id: Uuid) -> Result<(), EditError> {
        let Some(page) = self.document.pages.iter().find(|p| p.annotation(id).is_some()) else {
            return Ok(());
        };
        let page_id = page.id;
        self.execute("Delete annotation", |doc| {
            doc.update_page(page_id, |p| {
                let mut p = p.clone();
                p.annotations.retain(|a| a.id != id);
                p
            })
        })
    }

    pub fn delete_selection(&mut self) -> Result<(), EditError> {
        match self.selected {
            Some(id) => self.delete_annotation(id),
            None => Ok(()),
        }
    }

    pub fn move_selection(&mut self, delta: PointD) -> Result<(), EditError> {
        match self.selected {
            Some(id) => self.update_annotation(id, "Move annotation", |a| a.moved(delta)),
            None => Ok(()),
        }
    }

    pub fn reply(&mut self, id: Uuid, text: &str) -> Result<(), EditError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        let reply = CommentReply { id: Uuid::new_v4(), author: "You".into(), text: text.into(), created: Utc::now() };
        self.update_annotation(id, "Reply to comment", |a| {
            let mut a = a.clone();
            a.replies.push(reply);
            a
        })
    }

    pub fn set_resolved(&mut self, id: Uuid, resolved: bool) -> Result<(), EditError> {
        self.update_annotation(id, if resolved { "Resolve comment" } else { "Reopen comment" }, |a| Annotation { resolved, ..a.clone() })
    }

    pub fn rotate_page(&mut self, index: usize, degrees: i32) -> Result<(), EditError> {
        if degrees % 90 != 0 {
            return Err(EditError::Rejected("Rotation must be a multiple of 90 degrees.".into()));
        }
        let page = self.page_at(index)?;
        let rotation = (page.rotation as i64 + degrees as i64).rem_euclid(360) as u32;
        let page_id = page.id;
        self.execute("Rotate page", |doc| doc.update_page(page_id, |p| PdfPageState { rotation, ..p.clone() }))
    }

    pub fn crop_page(&mut self, index: usize, crop: Option<RectD>) -> Result<(), EditError> {
        let page = self.page_at(index)?;
        let crop = crop.map(|c| {
            RectD::intersection(c, RectD::new(0.0, 0.0, page.width, page.height)).unwrap_or(c)
        });
        let page_id = page.id;
        let label = if crop.is_some() { "Crop page" } else { "Reset crop" };
        self.execute(label, |doc| doc.update_page(page_id, |p| PdfPageState { crop, ..p.clone() }))
    }

    pub fn bookmark_page(&mut self, index: usize, name: &str) -> Result<(), EditError> {
        let page_id = self.page_at(index)?.id;
        let bookmark = name.trim().to_string();
        self.execute("Edit bookmark", |doc| doc.update_page(page_id, |p| PdfPageState { bookmark, ..p.clone() }))
    }

    pub fn insert_blank(&mut self, index: usize) -> Result<(), EditError> {
        let index = index.min(self.document.pages.len());
        let size = self.document.pages.get(self.current_page).map(|p| (p.width, p.height)).unwrap_or((595.0, 842.0));
        self.execute("Insert blank page", |doc| {
            let mut next = doc.clone();
            next.pages.insert(index, PdfPageState { width: size.0, height: size.1, ..Default::default() });
            next
        })?;
        self.navigate(index);
        Ok(())
    }

    pub fn duplicate_page(&mut self, index: usize) -> Result<(), EditError> {
        let copy = self.page_at(index)?.with_new_ids();
        self.execute("Duplicate page", |doc| {
            let mut next = doc.clone();
            next.pages.insert(index + 1, copy);
            next
        })?;
        self.navigate(index + 1);
        Ok(())
    }

    pub fn delete_pages(&mut self, indices: &[usize]) -> Result<(), EditError> {
        let ids: Vec<Uuid> = indices.iter().filter_map(|&i| self.document.pages.get(i)).map(|p| p.id).collect();
        if ids.len() >= self.document.pages.len() {
            return Err(EditError::Rejected("Keep at least one page in the document.".into()));
        }
        let label = if ids.len() == 1 { "Delete page" } else { "Delete pages" };
        self.execute(label, |doc| {
            let mut next = doc.clone();
            next.pages.retain(|p| !ids.contains(&p.id));
            next
        })
    }

    pub fn move_page(&mut self, source: usize, target: usize) -> Result<(), EditError> {
        self.page_at(source)?;
        let target = target.min(self.document.pages.len() - 1);
        if source == target {
            return Ok(());
        }
        self.execute("Reorder pages", |doc| {
            let mut next = doc.clone();
            let page = next.pages.remove(source);
            next.pages.insert(target, page);
            next
        })?;
        self.navigate(target);
        Ok(())
    }

    pub fn combine(&mut self, other: &PdfWorkspace) -> Result<(), EditError> {
        let combined = workspace_json::append(&self.document, other)?;
        self.execute("Combine PDFs", |_| combined)
    }

    pub fn set_info(&mut self, title: &str, author: &str) -> Result<(), EditError> {
        let (title, author) = (title.trim().to_string(), author.trim().to_string());
        self.execute("Edit document properties", |doc| PdfWorkspace {
            title: if title.is_empty() { doc.title.clone() } else { title },
            author,
            ..doc.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AnnotationKind;

    fn session(pages: usize) -> EditorSession {
        EditorSession::new(PdfWorkspace { pages: (0..pages).map(|_| PdfPageState::default()).collect(), ..Default::default() }).unwrap()
    }

    #[test]
    fn undo_restores_clean_state_by_identity() {
        let mut s = session(2);
        assert!(!s.is_dirty());
        s.add_annotation(Annotation::new(AnnotationKind::Rectangle, RectD::new(10.0, 10.0, 50.0, 50.0)), None).unwrap();
        assert!(s.is_dirty());
        assert_eq!(s.document().annotation_count(), 1);
        assert!(s.selected_annotation().is_some());
        s.undo();
        assert!(!s.is_dirty());
        assert_eq!(s.document().annotation_count(), 0);
        assert!(s.selected_annotation().is_none());
        s.redo();
        assert_eq!(s.document().annotation_count(), 1);
    }

    #[test]
    fn source_bytes_are_shared_between_snapshots() {
        let mut s = session(1);
        let bytes: Arc<[u8]> = Arc::from(&b"%PDF-1.7 fake"[..]);
        let source_id = Uuid::new_v4();
        s.execute("Add source", |doc| PdfWorkspace {
            sources: vec![crate::model::PdfSource { id: source_id, name: "a.pdf".into(), bytes: bytes.clone() }],
            ..doc.clone()
        })
        .unwrap();
        s.rotate_page(0, 90).unwrap();
        assert!(Arc::ptr_eq(&s.document().sources[0].bytes, &bytes));
    }

    #[test]
    fn page_operations() {
        let mut s = session(3);
        let first = s.document().pages[0].id;
        s.move_page(0, 2).unwrap();
        assert_eq!(s.document().pages[2].id, first);
        assert_eq!(s.current_page(), 2);
        s.duplicate_page(2).unwrap();
        assert_eq!(s.document().pages.len(), 4);
        s.delete_pages(&[0, 1]).unwrap();
        assert_eq!(s.document().pages.len(), 2);
        assert!(s.delete_pages(&[0, 1]).is_err());
        s.rotate_page(0, -90).unwrap();
        assert_eq!(s.document().pages[0].rotation, 270);
        s.insert_blank(1).unwrap();
        assert_eq!(s.document().pages.len(), 3);
    }

    #[test]
    fn history_is_bounded() {
        let mut s = session(1);
        for i in 0..(HISTORY_LIMIT + 10) {
            s.bookmark_page(0, &format!("b{i}")).unwrap();
        }
        let mut count = 0;
        while s.undo().is_some() {
            count += 1;
        }
        assert_eq!(count, HISTORY_LIMIT);
    }

    #[test]
    fn invalid_changes_are_rejected_without_history() {
        let mut s = session(1);
        let result = s.execute("Break", |doc| PdfWorkspace { pages: vec![], ..doc.clone() });
        assert!(result.is_err());
        assert!(!s.can_undo());
    }
}
