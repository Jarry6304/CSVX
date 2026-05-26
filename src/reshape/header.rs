use crate::grid::Grid;

/// Build a single promoted header row from the first `header_rows` rows of `grid`.
/// - `header_rows = 0` → empty header
/// - `header_rows = 1` → grid[0] (clone)
/// - `header_rows ≥ 2` → if `fill_merged` is true, ffill each header row's empty cells
///   from the previous non-empty cell, then concatenate all rows column-wise with '/'
///   (skipping empty fragments).
pub fn promote(grid: &Grid, header_rows: usize, fill_merged: bool) -> Vec<String> {
    if grid.is_empty() || header_rows == 0 {
        return Vec::new();
    }
    if header_rows == 1 {
        return grid[0].clone();
    }
    let rows: Vec<Vec<String>> = (0..header_rows)
        .map(|i| {
            let mut row = grid.get(i).cloned().unwrap_or_default();
            if fill_merged {
                ffill(&mut row);
            }
            row
        })
        .collect();
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    (0..width)
        .map(|c| {
            rows.iter()
                .map(|r| r.get(c).cloned().unwrap_or_default())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect()
}

fn ffill(row: &mut [String]) {
    let mut last = String::new();
    for cell in row.iter_mut() {
        if cell.trim().is_empty() {
            *cell = last.clone();
        } else {
            last = cell.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(rows: &[&[&str]]) -> Grid {
        rows.iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    #[test]
    fn single_row_passthrough() {
        let grid = g(&[&["a", "b", "c"], &["1", "2", "3"]]);
        assert_eq!(promote(&grid, 1, false), vec!["a", "b", "c"]);
    }

    #[test]
    fn empty_header_rows() {
        let grid = g(&[&["a", "b"]]);
        assert_eq!(promote(&grid, 0, false), Vec::<String>::new());
    }

    #[test]
    fn two_row_with_ffill() {
        let grid = g(&[&["G1", "", "G2", ""], &["A", "B", "C", "D"]]);
        let h = promote(&grid, 2, true);
        assert_eq!(h, vec!["G1/A", "G1/B", "G2/C", "G2/D"]);
    }

    #[test]
    fn two_row_no_ffill() {
        let grid = g(&[&["G1", "", "G2", ""], &["A", "B", "C", "D"]]);
        let h = promote(&grid, 2, false);
        assert_eq!(h, vec!["G1/A", "B", "G2/C", "D"]);
    }
}
