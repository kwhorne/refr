//! Files on disk: opening documents, atomic writes, the recovery copy and recent files.

use std::path::{Path, PathBuf};

use refr_core::PdfWorkspace;
use refr_core::workspace_json;
use refr_pdf::Engine;

pub const WORKSPACE_EXTENSION: &str = "pdfspace";

/// `~/Library/Application Support/Refr`, or `REFR_DATA_DIR` when set (tests use this).
pub fn data_dir() -> PathBuf {
    let dir = std::env::var_os("REFR_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("Refr"));
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn recovery_path() -> PathBuf {
    data_dir().join("recovery.pdfspace")
}

fn recents_path() -> PathBuf {
    data_dir().join("recent.json")
}

/// Writes through a temporary file and rename, so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

pub fn is_workspace(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case(WORKSPACE_EXTENSION))
}

pub fn is_openable(path: &Path) -> bool {
    is_workspace(path) || path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

/// Reads a PDF or `.pdfspace` workspace.
pub fn load(engine: &Engine, path: &Path) -> Result<PdfWorkspace, String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.len() > (workspace_json::MAXIMUM_SOURCE_BYTES * 2) as u64 {
        return Err("This file is too large.".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Untitled.pdf");
    if is_workspace(path) || bytes.first() == Some(&b'{') {
        let text = String::from_utf8(bytes).map_err(|_| "This workspace is not valid UTF-8 text.".to_string())?;
        workspace_json::load(&text).map_err(|e| e.to_string())
    } else {
        engine.open(bytes, name).map_err(|e| e.to_string())
    }
}

pub fn read_recovery() -> Option<PdfWorkspace> {
    let text = std::fs::read_to_string(recovery_path()).ok()?;
    workspace_json::load(&text).ok()
}

pub fn write_recovery(json: &str) -> std::io::Result<()> {
    write_atomic(&recovery_path(), json.as_bytes())
}

pub fn recents() -> Vec<PathBuf> {
    std::fs::read_to_string(recents_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<PathBuf>>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.is_file())
        .collect()
}

pub fn remember(path: &Path) -> Vec<PathBuf> {
    let mut list = recents();
    list.retain(|p| p != path);
    list.insert(0, path.to_path_buf());
    list.truncate(10);
    if let Ok(json) = serde_json::to_string(&list) {
        write_atomic(&recents_path(), json.as_bytes()).ok();
    }
    list
}

pub fn documents_dir() -> PathBuf {
    dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(std::env::temp_dir)
}

pub fn stem(title: &str) -> String {
    Path::new(title).file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("document").to_string()
}
