pub type Grid = Vec<Vec<String>>;

pub fn cell<'a>(grid: &'a Grid, row: usize, col: usize) -> Option<&'a str> {
    grid.get(row).and_then(|r| r.get(col)).map(|s| s.as_str())
}

pub fn cell_trim<'a>(grid: &'a Grid, row: usize, col: usize) -> &'a str {
    cell(grid, row, col).map(str::trim).unwrap_or("")
}
