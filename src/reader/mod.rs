pub mod csv;
pub mod xlsx;

use crate::grid::Grid;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Xlsx,
}

impl Format {
    pub fn from_path(p: &Path) -> Option<Format> {
        let ext = p.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "csv" => Some(Format::Csv),
            "xlsx" | "xlsm" => Some(Format::Xlsx),
            _ => None,
        }
    }

    pub fn needs_text_decode(self) -> bool {
        matches!(self, Format::Csv)
    }
}

pub trait FormatReader: Send + Sync {
    /// `text` is required for CSV (already decoded by Decoder) and ignored for XLSX.
    /// `sheet` is 1-indexed (1 = first sheet) and used by XLSX only.
    fn read(&self, path: &Path, text: Option<&str>, sheet: Option<u32>) -> anyhow::Result<Grid>;
}

pub fn make_reader(format: Format) -> Box<dyn FormatReader> {
    match format {
        Format::Csv => Box::new(csv::CsvReader::default()),
        Format::Xlsx => Box::new(xlsx::XlsxReader::default()),
    }
}
