use crate::error::ReshapeError;
use crate::profile::{ValueKind, ValueRules};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Cast a raw cell into Option<Decimal> according to value_rules.
/// Returns Ok(None) for missing values; Err for true parse/range failures.
pub fn cast_value(
    raw: &str,
    rules: &ValueRules,
    row: usize,
    col: usize,
) -> Result<Option<Decimal>, ReshapeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || rules.missing_values.iter().any(|m| m == raw || m == trimmed) {
        return Ok(None);
    }
    match rules.kind {
        ValueKind::Decimal | ValueKind::Int => {
            let cleaned = trimmed.replace(',', "");
            let d = Decimal::from_str(&cleaned).map_err(|_| ReshapeError::Cast {
                row,
                col,
                value: raw.to_string(),
                kind: format!("{:?}", rules.kind),
            })?;
            check_range(&d, rules, row, col, raw)?;
            Ok(Some(d))
        }
        ValueKind::String => Ok(None),
    }
}

fn check_range(
    d: &Decimal,
    rules: &ValueRules,
    row: usize,
    col: usize,
    raw: &str,
) -> Result<(), ReshapeError> {
    let v = d.to_f64().unwrap_or(0.0);
    let oor = |min: Option<f64>, max: Option<f64>| ReshapeError::OutOfRange {
        row,
        col,
        value: raw.to_string(),
        min,
        max,
    };
    if let Some(min) = rules.min {
        if v < min {
            return Err(oor(rules.min, rules.max));
        }
    }
    if let Some(max) = rules.max {
        if v > max {
            return Err(oor(rules.min, rules.max));
        }
    }
    if !rules.zero_is_valid && d.is_zero() {
        return Err(ReshapeError::Cast {
            row,
            col,
            value: raw.to_string(),
            kind: "zero_is_valid=false".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> ValueRules {
        ValueRules {
            kind: ValueKind::Decimal,
            decimals: 4,
            missing_values: vec!["".into(), "-".into(), "#N/A".into()],
            zero_is_valid: true,
            min: Some(0.0),
            max: Some(1_000_000.0),
        }
    }

    #[test]
    fn missing_returns_none() {
        assert!(cast_value("", &rules(), 0, 0).unwrap().is_none());
        assert!(cast_value("-", &rules(), 0, 0).unwrap().is_none());
        assert!(cast_value("#N/A", &rules(), 0, 0).unwrap().is_none());
        assert!(cast_value("  ", &rules(), 0, 0).unwrap().is_none());
    }

    #[test]
    fn decimal_parses() {
        let v = cast_value("403.4016", &rules(), 0, 0).unwrap().unwrap();
        assert_eq!(v.to_string(), "403.4016");
    }

    #[test]
    fn zero_allowed() {
        let v = cast_value("0", &rules(), 0, 0).unwrap().unwrap();
        assert!(v.is_zero());
    }

    #[test]
    fn negative_out_of_range() {
        let r = cast_value("-1", &rules(), 0, 0);
        assert!(matches!(r, Err(ReshapeError::OutOfRange { .. })));
    }

    #[test]
    fn comma_thousands_separator_stripped() {
        let v = cast_value("1,234.56", &rules(), 0, 0).unwrap().unwrap();
        assert_eq!(v.to_string(), "1234.56");
    }

    #[test]
    fn unparseable_errors() {
        let r = cast_value("abc", &rules(), 0, 0);
        assert!(matches!(r, Err(ReshapeError::Cast { .. })));
    }
}
