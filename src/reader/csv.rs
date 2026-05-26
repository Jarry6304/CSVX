use super::FormatReader;
use crate::grid::Grid;
use std::path::Path;

#[derive(Debug, Default)]
pub struct CsvReader;

impl FormatReader for CsvReader {
    fn read(&self, _path: &Path, text: Option<&str>, _sheet: Option<u32>) -> anyhow::Result<Grid> {
        let text = text.ok_or_else(|| {
            anyhow::anyhow!("CsvReader requires decoded text (Decoder must run first)")
        })?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(text.as_bytes());

        let mut grid: Grid = Vec::new();
        for record in rdr.records() {
            let rec = record?;
            grid.push(rec.iter().map(|s| s.to_string()).collect());
        }
        Ok(grid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_simple_csv() {
        let r = CsvReader;
        let grid = r
            .read(Path::new("dummy.csv"), Some("a,b,c\n1,2,3\n"), None)
            .unwrap();
        assert_eq!(grid, vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["1".to_string(), "2".to_string(), "3".to_string()],
        ]);
    }

    #[test]
    fn flexible_rows() {
        let r = CsvReader;
        let grid = r
            .read(Path::new("d.csv"), Some("a,b\n1,2,3\n"), None)
            .unwrap();
        assert_eq!(grid[0].len(), 2);
        assert_eq!(grid[1].len(), 3);
    }
}
