//! `.pdfspace` serialization, validation guardrails and workspace composition.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::model::PdfWorkspace;

pub const MAXIMUM_SOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_PAGES: usize = 4096;
pub const MAXIMUM_ANNOTATIONS: usize = 50_000;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("The workspace could not be read: {0}")]
    Json(#[from] serde_json::Error),
}

fn invalid<T>(message: &'static str) -> Result<T, WorkspaceError> {
    Err(WorkspaceError::Invalid(message))
}

pub fn save(document: &PdfWorkspace) -> String {
    serde_json::to_string(document).expect("workspace records always serialize")
}

pub fn load(json: &str) -> Result<PdfWorkspace, WorkspaceError> {
    if json.len() > MAXIMUM_SOURCE_BYTES * 2 {
        return invalid("This workspace exceeds the 128 MB recovery-file limit.");
    }
    let document: PdfWorkspace = serde_json::from_str(json)?;
    validate(&document)?;
    Ok(document)
}

pub fn is_pdf(bytes: &[u8]) -> bool {
    bytes.len() >= 5 && bytes[..bytes.len().min(1024)].windows(5).any(|w| w == b"%PDF-")
}

pub fn validate(document: &PdfWorkspace) -> Result<(), WorkspaceError> {
    if document.format_version != 1 {
        return invalid("Unsupported workspace format version.");
    }
    if document.title.len() > 1024 || document.author.len() > 4096 {
        return invalid("Invalid document information.");
    }
    if document.pages.is_empty() || document.pages.len() > MAXIMUM_PAGES {
        return invalid("A workspace must contain 1–4096 pages.");
    }
    if document.sources.len() > 4096 {
        return invalid("Invalid source collection.");
    }
    let mut source_ids = HashSet::new();
    let mut source_bytes = 0usize;
    for source in &document.sources {
        if source.name.len() > 1024 || !source_ids.insert(source.id) {
            return invalid("Invalid or duplicate PDF source.");
        }
        source_bytes += source.bytes.len();
        if source_bytes > MAXIMUM_SOURCE_BYTES {
            return invalid("The combined source PDFs exceed 64 MB.");
        }
        if !is_pdf(&source.bytes) {
            return invalid("A source is not a PDF document.");
        }
    }
    let mut page_ids = HashSet::new();
    let mut annotation_ids = HashSet::new();
    let mut annotation_count = 0usize;
    let mut point_count = 0usize;
    for page in &document.pages {
        if !page_ids.insert(page.id) {
            return invalid("Invalid or duplicate page identifier.");
        }
        let dimension_ok = |v: f64| v.is_finite() && v > 0.0 && v <= 100_000.0;
        if !dimension_ok(page.width) || !dimension_ok(page.height) {
            return invalid("Invalid page dimensions.");
        }
        if !matches!(page.rotation, 0 | 90 | 180 | 270) {
            return invalid("Invalid page rotation.");
        }
        if let Some(id) = page.source_id
            && !source_ids.contains(&id)
        {
            return invalid("Missing source document.");
        }
        if page.source_page < 1 || page.source_page > MAXIMUM_PAGES as u32 {
            return invalid("Invalid source page number.");
        }
        if page.bookmark.len() > 4096 {
            return invalid("Invalid bookmark.");
        }
        if let Some(crop) = page.crop
            && (!crop.is_finite()
                || crop.width < 1.0
                || crop.height < 1.0
                || crop.x < 0.0
                || crop.y < 0.0
                || crop.right() > page.width + 0.01
                || crop.bottom() > page.height + 0.01)
        {
            return invalid("Invalid page crop.");
        }
        annotation_count += page.annotations.len();
        if annotation_count > MAXIMUM_ANNOTATIONS {
            return invalid("Too many annotations.");
        }
        for annotation in &page.annotations {
            if !annotation_ids.insert(annotation.id) {
                return invalid("Invalid or duplicate annotation.");
            }
            let b = annotation.bounds;
            if !b.is_finite()
                || b.width < 0.0
                || b.height < 0.0
                || annotation.text.len() > 100_000
                || annotation.author.len() > 4096
                || !annotation.stroke_width.is_finite()
                || annotation.stroke_width <= 0.0
                || annotation.stroke_width > 100.0
                || !annotation.font_size.is_finite()
                || annotation.font_size < 1.0
                || annotation.font_size > 1000.0
                || annotation.points.len() > 100_000
            {
                return invalid("Invalid annotation geometry or appearance.");
            }
            point_count += annotation.points.len();
            if point_count > 2_000_000 || annotation.points.iter().any(|p| !p.is_finite()) {
                return invalid("Invalid or excessive annotation path data.");
            }
            if annotation.replies.len() > 1000
                || annotation.replies.iter().any(|r| r.author.len() > 4096 || r.text.len() > 100_000)
            {
                return invalid("Invalid comment replies.");
            }
        }
    }
    Ok(())
}

/// Combines workspaces without colliding identifiers, even when importing the same workspace twice.
pub fn append(current: &PdfWorkspace, incoming: &PdfWorkspace) -> Result<PdfWorkspace, WorkspaceError> {
    validate(current)?;
    validate(incoming)?;
    let source_map: HashMap<Uuid, Uuid> = incoming.sources.iter().map(|s| (s.id, Uuid::new_v4())).collect();
    let mut result = current.clone();
    result.sources.extend(incoming.sources.iter().map(|s| crate::model::PdfSource { id: source_map[&s.id], ..s.clone() }));
    result.pages.extend(incoming.pages.iter().map(|page| {
        let mut page = page.with_new_ids();
        page.source_id = page.source_id.map(|id| source_map[&id]);
        page
    }));
    validate(&result)?;
    Ok(result)
}

/// A new workspace holding copies of the given pages, keeping only the sources they use.
pub fn extract(document: &PdfWorkspace, indices: &[usize]) -> Result<PdfWorkspace, WorkspaceError> {
    let pages: Vec<_> = indices
        .iter()
        .filter_map(|&i| document.pages.get(i))
        .map(|p| p.with_new_ids())
        .collect();
    if pages.is_empty() {
        return invalid("Select at least one page.");
    }
    let used: HashSet<Uuid> = pages.iter().filter_map(|p| p.source_id).collect();
    let result = PdfWorkspace {
        sources: document.sources.iter().filter(|s| used.contains(&s.id)).cloned().collect(),
        pages,
        ..document.clone()
    };
    validate(&result)?;
    Ok(result)
}
