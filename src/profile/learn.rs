use super::*;
use crate::grid::Grid;
use crate::reader::Format;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct StructureDraft {
    pub header_row: usize,
    pub anchor_col: usize,
    pub anchor_value: String,
    pub day_cols: usize,
    pub data_start_row: usize,
    pub signature: String,
    pub header_contains: Vec<String>,
}

/// Build a Profile from a structure template (and optional sample for value rules).
/// Returns a Profile draft. Semantic fields (`id_cols`, `year_month_from`, `encoding`)
/// are best-guess and may need a human/LLM pass to confirm.
pub fn learn(
    structure_grid: &Grid,
    sample_grid: Option<&Grid>,
    name: &str,
    format: Format,
) -> Profile {
    let draft = learn_structure(structure_grid);
    let value_rules = match sample_grid {
        Some(g) => learn_values(g, &draft),
        None => default_value_rules(),
    };

    Profile {
        name: name.to_string(),
        match_rules: MatchRules {
            filename: Some(format!("*.{}", format_ext(format))),
            header_contains: draft.header_contains.clone(),
            header_signature: Some(draft.signature.clone()),
            priority: 0,
        },
        encoding: default_learn_encoding(format),
        format,
        sheet: Some(1),
        header_rows: 1,
        data_start_row: draft.data_start_row,
        skip_blank_id: true,
        fill_merged: false,
        id_cols: vec![
            IdCol {
                name: "caseid".into(),
                col: 1,
                header: structure_grid
                    .get(draft.header_row.saturating_sub(1))
                    .and_then(|r| r.first())
                    .cloned(),
                kind: "string".into(),
            },
            IdCol {
                name: "plant_name".into(),
                col: 2,
                header: structure_grid
                    .get(draft.header_row.saturating_sub(1))
                    .and_then(|r| r.get(1))
                    .cloned(),
                kind: "string".into(),
            },
        ],
        unpivot: Unpivot {
            anchor: draft.anchor_value.clone(),
            anchor_match: AnchorMatch::Exact,
            day_cols: draft.day_cols,
            validate_calendar_day: true,
            var_name: "shift_hour".into(),
            value_name: "daily_kwh".into(),
            year_month_from: "filename:0..6".into(),
            anchor_col_observed: Some(draft.anchor_col + 1),
        },
        value_rules,
    }
}

/// Pure-structure inference: locate header row, anchor "1日", count day columns,
/// find data start (first non-empty A-column row past header), compute signature.
pub fn learn_structure(grid: &Grid) -> StructureDraft {
    let mut hdr_idx = 0usize;
    for (i, row) in grid.iter().enumerate() {
        if row.iter().any(|c| is_day_token(c.trim())) {
            hdr_idx = i;
            break;
        }
    }

    let empty_row: Vec<String> = Vec::new();
    let hdr_row = grid.get(hdr_idx).unwrap_or(&empty_row);

    let anchor_col = hdr_row
        .iter()
        .position(|c| c.trim() == "1日")
        .unwrap_or(0);

    let day_cols = hdr_row
        .iter()
        .skip(anchor_col)
        .take_while(|c| is_day_token(c.trim()))
        .count();

    let mut data_start_row = hdr_idx + 2;
    for r in (hdr_idx + 1)..grid.len() {
        let id_cell = grid[r].first().map(|s| s.trim()).unwrap_or("");
        if !id_cell.is_empty() {
            data_start_row = r + 1;
            break;
        }
    }

    let signature = super::signature::compute(hdr_row);

    let mut header_contains: Vec<String> = Vec::new();
    if let Some(first) = hdr_row.iter().find(|c| !c.trim().is_empty()) {
        header_contains.push(first.trim().to_string());
    }
    header_contains.push("1日".to_string());
    if day_cols > 0 {
        header_contains.push(format!("{}日", day_cols));
    }

    StructureDraft {
        header_row: hdr_idx + 1,
        anchor_col,
        anchor_value: "1日".to_string(),
        day_cols,
        data_start_row,
        signature,
        header_contains,
    }
}

/// Inspect numeric cells in the day_cols block to learn decimals/min/max and pick up unknown tokens.
pub fn learn_values(sample: &Grid, draft: &StructureDraft) -> ValueRules {
    let mut decimals: u32 = 0;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut unknown_tokens: BTreeSet<String> = BTreeSet::new();

    let start = draft.data_start_row.saturating_sub(1);
    for row in sample.iter().skip(start) {
        let id = row.first().map(|s| s.trim()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        for c in draft.anchor_col..draft.anchor_col + draft.day_cols {
            let raw = row.get(c).map(|s| s.trim()).unwrap_or("");
            if raw.is_empty() {
                continue;
            }
            match raw.replace(',', "").parse::<f64>() {
                Ok(v) => {
                    if v < min {
                        min = v;
                    }
                    if v > max {
                        max = v;
                    }
                    if let Some(idx) = raw.find('.') {
                        let d = (raw.len() - idx - 1) as u32;
                        if d > decimals {
                            decimals = d;
                        }
                    }
                }
                Err(_) => {
                    unknown_tokens.insert(raw.to_string());
                }
            }
        }
    }

    let mut missing_values: Vec<String> = vec![
        "".into(),
        "-".into(),
        "#DIV/0!".into(),
        "#N/A".into(),
        "#VALUE!".into(),
    ];
    for tok in unknown_tokens {
        if !missing_values.contains(&tok) {
            missing_values.push(tok);
        }
    }

    ValueRules {
        kind: ValueKind::Decimal,
        decimals,
        missing_values,
        zero_is_valid: true,
        min: Some(if min.is_finite() { min.min(0.0) } else { 0.0 }),
        max: Some(if max.is_finite() {
            (max * 2.0).max(1000.0)
        } else {
            1_000_000.0
        }),
    }
}

pub fn default_value_rules() -> ValueRules {
    ValueRules {
        kind: ValueKind::Decimal,
        decimals: 4,
        missing_values: vec![
            "".into(),
            "-".into(),
            "#DIV/0!".into(),
            "#N/A".into(),
            "#VALUE!".into(),
        ],
        zero_is_valid: true,
        min: Some(0.0),
        max: Some(1_000_000.0),
    }
}

fn is_day_token(s: &str) -> bool {
    s.strip_suffix("日")
        .and_then(|n| n.parse::<u32>().ok())
        .is_some()
}

fn default_learn_encoding(format: Format) -> String {
    match format {
        Format::Csv => "utf-8".into(),
        Format::Xlsx => "big5".into(),
    }
}

fn format_ext(format: Format) -> &'static str {
    match format {
        Format::Csv => "csv",
        Format::Xlsx => "xlsx",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[&str]]) -> Grid {
        rows.iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[test]
    fn structure_solar_layout() {
        let mut header = vec!["caseid", "plant", "extra"];
        for d in 1..=31 {
            let s: &'static str = Box::leak(format!("{}日", d).into_boxed_str());
            header.push(s);
        }
        let header_slice: &[&str] = &header;

        let g = grid(&[
            header_slice,
            &["", "", "Total", "100"],
            &["A001", "PlantA", "x", "10.5"],
            &["A002", "PlantB", "x", "20.123"],
        ]);
        let draft = learn_structure(&g);
        assert_eq!(draft.anchor_col, 3);
        assert_eq!(draft.day_cols, 31);
        assert_eq!(draft.data_start_row, 3);
    }

    #[test]
    fn is_day_token_works() {
        assert!(is_day_token("1日"));
        assert!(is_day_token("31日"));
        assert!(!is_day_token("掛表日"));
        assert!(!is_day_token("日"));
    }
}
