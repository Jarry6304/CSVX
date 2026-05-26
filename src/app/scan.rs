use crate::reader::Format;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// List every supported file (csv / xlsx / xlsm) directly inside `dir`.
/// Non-recursive: monthly reports drop into a flat input directory.
pub fn list_input_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if Format::from_path(p).is_some() {
            out.push(p.to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}
