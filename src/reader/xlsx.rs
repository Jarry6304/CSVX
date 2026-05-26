use super::FormatReader;
use crate::grid::Grid;
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

#[derive(Debug, Default)]
pub struct XlsxReader;

impl FormatReader for XlsxReader {
    fn read(&self, path: &Path, _text: Option<&str>, sheet: Option<u32>) -> anyhow::Result<Grid> {
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| anyhow::anyhow!("open_workbook({}): {e}", path.display()))?;

        let names = workbook.sheet_names();
        if names.is_empty() {
            anyhow::bail!("workbook has no sheets");
        }
        let idx = sheet.map(|s| s.saturating_sub(1) as usize).unwrap_or(0);
        let name = names
            .get(idx)
            .ok_or_else(|| anyhow::anyhow!("sheet index {} out of range (have {})", idx, names.len()))?
            .clone();

        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| anyhow::anyhow!("worksheet_range({name}): {e}"))?;

        let mut grid: Grid = Vec::with_capacity(range.height());
        for row in range.rows() {
            grid.push(row.iter().map(cell_to_string).collect());
        }
        Ok(grid)
    }
}

fn cell_to_string(c: &Data) -> String {
    match c {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => format_float(*f),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#{:?}", e),
    }
}

fn format_float(f: f64) -> String {
    // Avoid scientific notation; trim trailing zeros but keep integer floats as integers.
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        // 10 fractional digits is enough for typical kWh decimals; trim trailing zeros.
        let s = format!("{:.10}", f);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_float_integer() {
        assert_eq!(format_float(403.0), "403");
    }

    #[test]
    fn format_float_decimal() {
        assert_eq!(format_float(403.4016), "403.4016");
    }

    #[test]
    fn cell_string() {
        assert_eq!(cell_to_string(&Data::String("hi".into())), "hi");
        assert_eq!(cell_to_string(&Data::Empty), "");
        assert_eq!(cell_to_string(&Data::Float(1.5)), "1.5");
    }
}
