use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReshapeError {
    #[error("anchor '{anchor}' not found via {strategy:?}")]
    AnchorMissing { anchor: String, strategy: String },

    #[error("invalid calendar date {y:04}-{m:02}-{d:02} at col {col}")]
    InvalidDate { y: i32, m: u32, d: u32, col: usize },

    #[error("value at row {row} col {col} out of range: '{value}' not in [{min:?},{max:?}]")]
    OutOfRange {
        row: usize,
        col: usize,
        value: String,
        min: Option<f64>,
        max: Option<f64>,
    },

    #[error("cast failed at row {row} col {col}: '{value}' -> {kind:?}")]
    Cast {
        row: usize,
        col: usize,
        value: String,
        kind: String,
    },

    #[error("malformed profile: {0}")]
    BadProfile(String),

    #[error("malformed year_month '{0}' (expected YYYYMM)")]
    BadYearMonth(String),
}

#[derive(Error, Debug)]
pub enum DaoError {
    #[error("dao gate closed for target '{0}' (check [database.enabled] in config)")]
    GateClosed(String),

    #[error("dao target '{0}' is not implemented")]
    NotImplemented(String),

    #[error("invalid table name '{0}': only [A-Za-z0-9_] allowed (1-128 chars)")]
    InvalidTable(String),

    #[error("db error: {0}")]
    Db(String),
}

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("no profile matched (filename={file})")]
    NoMatch { file: String },

    #[error("multiple profiles matched (file={file}, names={names:?}); set priority to disambiguate")]
    AmbiguousMatch { file: String, names: Vec<String> },

    #[error("header signature drift on '{profile}': expected {expected}, got {actual}")]
    SignatureDrift {
        profile: String,
        expected: String,
        actual: String,
    },

    #[error("io error reading profile '{0}': {1}")]
    Io(String, String),

    #[error("json error in profile '{0}': {1}")]
    Json(String, String),
}
