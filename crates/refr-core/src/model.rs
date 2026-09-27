//! Immutable workspace records.
//!
//! Field names and value encodings follow PdfSpace's `.pdfspace` JSON (PascalCase names,
//! numeric enums, base64 source bytes), so workspaces move between the two applications.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use uuid::Uuid;

use crate::geometry::{PointD, RectD};

pub const DEFAULT_COLOR: u32 = 0xFF1473E6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AnnotationKind {
    Highlight = 0,
    Underline = 1,
    Strikeout = 2,
    Ink = 3,
    Rectangle = 4,
    Ellipse = 5,
    Line = 6,
    Arrow = 7,
    Text = 8,
    Note = 9,
    Signature = 10,
    Stamp = 11,
    Check = 12,
}

impl AnnotationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Highlight => "highlight",
            Self::Underline => "underline",
            Self::Strikeout => "strikeout",
            Self::Ink => "ink",
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
            Self::Arrow => "arrow",
            Self::Text => "text",
            Self::Note => "comment",
            Self::Signature => "signature",
            Self::Stamp => "stamp",
            Self::Check => "check mark",
        }
    }

    /// Text markup follows the words it was made from and is not moved or resized freely.
    pub fn is_text_markup(self) -> bool {
        matches!(self, Self::Highlight | Self::Underline | Self::Strikeout)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommentReply {
    pub id: Uuid,
    pub author: String,
    pub text: String,
    pub created: DateTime<Utc>,
}

/// Coordinates are PDF points in the source page's top-left logical coordinate system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Annotation {
    pub id: Uuid,
    pub kind: AnnotationKind,
    pub bounds: RectD,
    pub points: Vec<PointD>,
    pub color: u32,
    pub stroke_width: f64,
    pub font_size: f64,
    pub text: String,
    pub author: String,
    pub created: DateTime<Utc>,
    pub resolved: bool,
    pub replies: Vec<CommentReply>,
}

impl Default for Annotation {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: AnnotationKind::Highlight,
            bounds: RectD::default(),
            points: Vec::new(),
            color: DEFAULT_COLOR,
            stroke_width: 2.0,
            font_size: 14.0,
            text: String::new(),
            author: "You".into(),
            created: Utc::now(),
            resolved: false,
            replies: Vec::new(),
        }
    }
}

impl Annotation {
    pub fn new(kind: AnnotationKind, bounds: RectD) -> Self {
        Self { kind, bounds, ..Default::default() }
    }

    pub fn moved(&self, delta: PointD) -> Annotation {
        Annotation {
            bounds: self.bounds.translate(delta),
            points: self.points.iter().map(|p| *p + delta).collect(),
            ..self.clone()
        }
    }

    /// Scales the annotation so its bounds become `target`, keeping path points proportional.
    pub fn resized(&self, target: RectD) -> Annotation {
        let from = self.bounds;
        let sx = if from.width.abs() < 1e-9 { 1.0 } else { target.width / from.width };
        let sy = if from.height.abs() < 1e-9 { 1.0 } else { target.height / from.height };
        Annotation {
            bounds: target,
            points: self
                .points
                .iter()
                .map(|p| PointD::new(target.x + (p.x - from.x) * sx, target.y + (p.y - from.y) * sy))
                .collect(),
            ..self.clone()
        }
    }

    /// A copy with fresh identifiers for the annotation and its replies.
    pub fn with_new_ids(&self) -> Annotation {
        Annotation {
            id: Uuid::new_v4(),
            replies: self.replies.iter().map(|r| CommentReply { id: Uuid::new_v4(), ..r.clone() }).collect(),
            ..self.clone()
        }
    }
}

/// Source bytes are immutable and shared across history snapshots; original files are never overwritten.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PdfSource {
    pub id: Uuid,
    pub name: String,
    #[serde(with = "base64_bytes")]
    pub bytes: Arc<[u8]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PdfPageState {
    pub id: Uuid,
    pub source_id: Option<Uuid>,
    /// One-based page number in the source document.
    pub source_page: u32,
    pub width: f64,
    pub height: f64,
    pub rotation: u32,
    pub crop: Option<RectD>,
    pub bookmark: String,
    pub annotations: Vec<Annotation>,
}

impl Default for PdfPageState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            source_id: None,
            source_page: 1,
            width: 595.0,
            height: 842.0,
            rotation: 0,
            crop: None,
            bookmark: String::new(),
            annotations: Vec::new(),
        }
    }
}

impl PdfPageState {
    pub fn visible_box(&self) -> RectD {
        self.crop.unwrap_or(RectD::new(0.0, 0.0, self.width, self.height))
    }

    pub fn display_width(&self) -> f64 {
        let b = self.visible_box();
        if self.rotation % 180 == 0 { b.width } else { b.height }
    }

    pub fn display_height(&self) -> f64 {
        let b = self.visible_box();
        if self.rotation % 180 == 0 { b.height } else { b.width }
    }

    pub fn annotation(&self, id: Uuid) -> Option<&Annotation> {
        self.annotations.iter().find(|a| a.id == id)
    }

    /// A copy with fresh identifiers for the page and everything on it.
    pub fn with_new_ids(&self) -> PdfPageState {
        PdfPageState {
            id: Uuid::new_v4(),
            annotations: self.annotations.iter().map(Annotation::with_new_ids).collect(),
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PdfWorkspace {
    pub format_version: u32,
    pub title: String,
    pub author: String,
    pub sources: Vec<PdfSource>,
    pub pages: Vec<PdfPageState>,
}

impl Default for PdfWorkspace {
    fn default() -> Self {
        Self {
            format_version: 1,
            title: "Untitled.pdf".into(),
            author: String::new(),
            sources: Vec::new(),
            pages: vec![PdfPageState::default()],
        }
    }
}

impl PdfWorkspace {
    pub fn update_page(&self, id: Uuid, update: impl FnOnce(&PdfPageState) -> PdfPageState) -> PdfWorkspace {
        let mut next = self.clone();
        if let Some(page) = next.pages.iter_mut().find(|p| p.id == id) {
            *page = update(page);
        }
        next
    }

    pub fn annotation_count(&self) -> usize {
        self.pages.iter().map(|p| p.annotations.len()).sum()
    }

    pub fn source(&self, id: Uuid) -> Option<&PdfSource> {
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn page_index(&self, id: Uuid) -> Option<usize> {
        self.pages.iter().position(|p| p.id == id)
    }
}

mod base64_bytes {
    use std::sync::Arc;

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Arc<[u8]>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Arc<[u8]>, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        STANDARD.decode(text.as_bytes()).map(Arc::from).map_err(serde::de::Error::custom)
    }
}
