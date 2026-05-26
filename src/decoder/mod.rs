pub mod encoding;

use std::fmt::Debug;

pub trait Decoder: Send + Sync + Debug {
    /// Decode raw bytes into a UTF-8 String.
    /// Implementations must strip BOM if present.
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<String>;
}

/// Build a decoder from a profile-supplied encoding label.
/// Accepts: "utf-8" / "utf8" / "big5" / "bom" / "auto" (case-insensitive).
pub fn from_label(label: &str) -> Box<dyn Decoder> {
    match label.to_ascii_lowercase().as_str() {
        "big5" => Box::new(encoding::Big5Decoder),
        "bom" | "auto" => Box::new(encoding::BomDecoder),
        _ => Box::new(encoding::Utf8Decoder),
    }
}
