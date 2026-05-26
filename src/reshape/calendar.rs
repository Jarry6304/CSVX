use chrono::NaiveDate;

/// Validate that (year, month, day) is a real calendar date.
/// `ym` is "YYYYMM"; `day` is 1..=31. Returns false if the date doesn't exist
/// (e.g., 2024-04-31, 2025-02-29).
pub fn is_valid(ym: &str, day: u32) -> bool {
    let Some((y, m)) = parse_ym(ym) else {
        return false;
    };
    NaiveDate::from_ymd_opt(y, m, day).is_some()
}

pub fn parse_ym(ym: &str) -> Option<(i32, u32)> {
    if ym.len() != 6 {
        return None;
    }
    let y: i32 = ym.get(0..4)?.parse().ok()?;
    let m: u32 = ym.get(4..6)?.parse().ok()?;
    if !(1..=12).contains(&m) {
        return None;
    }
    Some((y, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn april_thirtyone_invalid() {
        assert!(!is_valid("202604", 31));
        assert!(is_valid("202604", 30));
    }

    #[test]
    fn feb_leap_year() {
        assert!(is_valid("202402", 29));
        assert!(!is_valid("202502", 29));
        assert!(is_valid("202402", 28));
    }

    #[test]
    fn march_thirty_one() {
        assert!(is_valid("202603", 31));
    }

    #[test]
    fn bad_ym() {
        assert!(!is_valid("20260", 1));
        assert!(!is_valid("202613", 1));
        assert!(!is_valid("XXXX01", 1));
    }
}
