use crate::error::ReshapeError;
use crate::profile::AnchorMatch;

/// Locate the anchor column in a promoted header row.
pub fn find(header: &[String], anchor: &str, strategy: AnchorMatch) -> Result<usize, ReshapeError> {
    header
        .iter()
        .position(|h| match strategy {
            AnchorMatch::Exact => h.trim() == anchor,
            AnchorMatch::Contains => h.contains(anchor),
        })
        .ok_or_else(|| ReshapeError::AnchorMissing {
            anchor: anchor.to_string(),
            strategy: format!("{:?}", strategy),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<String> {
        vec!["案件編號", "案件名稱", "掛表日", "1日", "2日", "3日"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[test]
    fn exact_match_skips_decoy() {
        let h = hdr();
        let col = find(&h, "1日", AnchorMatch::Exact).unwrap();
        assert_eq!(col, 3);
    }

    #[test]
    fn exact_would_miss_decoy() {
        let h = hdr();
        let r = find(&h, "掛表日", AnchorMatch::Exact);
        assert_eq!(r.unwrap(), 2);
    }

    #[test]
    fn not_found() {
        let h = hdr();
        let r = find(&h, "999日", AnchorMatch::Exact);
        assert!(matches!(r, Err(ReshapeError::AnchorMissing { .. })));
    }
}
