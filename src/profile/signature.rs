use sha1::{Digest, Sha1};

/// Compute a stable signature over a header row: trim each cell, join with '|', sha1, hex,
/// truncated to 12 hex chars to match the spec's `header_signature` examples ("356bdc2a0ec6").
pub fn compute(header: &[String]) -> String {
    let normalized: Vec<&str> = header.iter().map(|s| s.trim()).collect();
    let joined = normalized.join("|");
    let mut h = Sha1::new();
    h.update(joined.as_bytes());
    let digest = h.finalize();
    let full = hex::encode(digest);
    full[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_signature() {
        let h: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let s1 = compute(&h);
        let s2 = compute(&h);
        assert_eq!(s1, s2);
        assert_eq!(s1.len(), 12);
    }

    #[test]
    fn ignores_surrounding_whitespace() {
        let a: Vec<String> = vec!["a ".into(), " b".into()];
        let b: Vec<String> = vec!["a".into(), "b".into()];
        assert_eq!(compute(&a), compute(&b));
    }

    #[test]
    fn different_for_different_headers() {
        let a: Vec<String> = vec!["a".into(), "b".into()];
        let b: Vec<String> = vec!["a".into(), "c".into()];
        assert_ne!(compute(&a), compute(&b));
    }
}
