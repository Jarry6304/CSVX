pub mod anchor;
pub mod calendar;
pub mod cast;
pub mod header;
pub mod unpivot;

use crate::dao::row::DailyKwhRow;
use crate::error::ReshapeError;
use crate::grid::Grid;
use crate::profile::Profile;

/// Apply a Profile to a Grid for a given year_month ("YYYYMM"), producing one row per
/// (case × valid calendar day). Pure function — no I/O.
pub fn apply(grid: &Grid, profile: &Profile, year_month: &str) -> Result<Vec<DailyKwhRow>, ReshapeError> {
    if year_month.len() != 6 || year_month.chars().any(|c| !c.is_ascii_digit()) {
        return Err(ReshapeError::BadYearMonth(year_month.to_string()));
    }

    let header = header::promote(grid, profile.header_rows, profile.fill_merged);
    let anchor_col = anchor::find(&header, &profile.unpivot.anchor, profile.unpivot.anchor_match)?;
    let caseid_col = profile.caseid_col_index()?;
    let plant_name_col = profile.plant_name_col_index();

    let data_start = profile.data_start_row.saturating_sub(1); // 1-indexed → 0-indexed
    let mut out: Vec<DailyKwhRow> = Vec::new();

    for (rno, row) in grid.iter().enumerate().skip(data_start) {
        if profile.skip_blank_id {
            let cell = row.get(caseid_col).map(|s| s.trim()).unwrap_or("");
            if cell.is_empty() {
                continue;
            }
        }
        let caseid = row.get(caseid_col).cloned().unwrap_or_default();
        let plant_name = plant_name_col
            .and_then(|c| row.get(c).cloned())
            .filter(|s| !s.trim().is_empty());

        for d in 0..profile.unpivot.day_cols {
            let day = (d + 1) as u32;
            if profile.unpivot.validate_calendar_day && !calendar::is_valid(year_month, day) {
                continue;
            }
            let cell_col = anchor_col + d;
            let raw = row.get(cell_col).map(String::as_str).unwrap_or("");
            let value = cast::cast_value(raw, &profile.value_rules, rno, cell_col)?;
            out.push(unpivot::make_row(
                &caseid,
                plant_name.as_deref(),
                year_month,
                day,
                value,
            ));
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::*;
    use crate::reader::Format;

    fn solar_profile() -> Profile {
        Profile {
            name: "test".into(),
            match_rules: MatchRules {
                filename: None,
                header_contains: vec![],
                header_signature: None,
                priority: 0,
            },
            encoding: "utf-8".into(),
            format: Format::Csv,
            sheet: None,
            header_rows: 1,
            data_start_row: 3,
            skip_blank_id: true,
            fill_merged: false,
            id_cols: vec![
                IdCol {
                    name: "caseid".into(),
                    col: 1,
                    header: None,
                    kind: "string".into(),
                },
                IdCol {
                    name: "plant_name".into(),
                    col: 2,
                    header: None,
                    kind: "string".into(),
                },
            ],
            unpivot: Unpivot {
                anchor: "1日".into(),
                anchor_match: AnchorMatch::Exact,
                day_cols: 31,
                validate_calendar_day: true,
                var_name: "shift_hour".into(),
                value_name: "daily_kwh".into(),
                year_month_from: "filename:0..6".into(),
                anchor_col_observed: None,
            },
            value_rules: ValueRules {
                kind: ValueKind::Decimal,
                decimals: 4,
                missing_values: vec!["".into(), "-".into()],
                zero_is_valid: true,
                min: Some(0.0),
                max: Some(1_000_000.0),
            },
        }
    }

    fn build_grid_31() -> Grid {
        let mut header: Vec<String> = vec!["案件編號".into(), "案件名稱".into()];
        for d in 1..=31 {
            header.push(format!("{}日", d));
        }
        let mut totals: Vec<String> = vec!["".into(), "".into()];
        for _ in 1..=31 {
            totals.push("99".into());
        }
        let mut data: Vec<String> = vec!["A001".into(), "PlantA".into()];
        for d in 1..=31 {
            data.push(format!("{}.5", d));
        }
        vec![header, totals, data]
    }

    #[test]
    fn april_drops_day_31() {
        let g = build_grid_31();
        let rows = apply(&g, &solar_profile(), "202604").unwrap();
        assert_eq!(rows.len(), 30);
        assert_eq!(rows.first().unwrap().shift_hour, "20260401");
        assert_eq!(rows.last().unwrap().shift_hour, "20260430");
    }

    #[test]
    fn march_keeps_day_31() {
        let g = build_grid_31();
        let rows = apply(&g, &solar_profile(), "202603").unwrap();
        assert_eq!(rows.len(), 31);
        assert_eq!(rows.last().unwrap().shift_hour, "20260331");
    }

    #[test]
    fn skip_blank_id() {
        let mut g = build_grid_31();
        // Add a blank-id row
        let mut blank: Vec<String> = vec!["".into(), "".into()];
        for _ in 1..=31 {
            blank.push("".into());
        }
        g.push(blank);
        let rows = apply(&g, &solar_profile(), "202603").unwrap();
        assert_eq!(rows.len(), 31);
    }
}
