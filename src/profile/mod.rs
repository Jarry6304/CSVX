pub mod learn;
pub mod match_rules;
pub mod signature;

use crate::error::{ProfileError, ReshapeError};
use crate::grid::Grid;
use crate::reader::Format;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub name: String,

    #[serde(rename = "match")]
    pub match_rules: MatchRules,

    #[serde(default = "default_encoding")]
    pub encoding: String,

    pub format: Format,

    #[serde(default)]
    pub sheet: Option<u32>,

    #[serde(default = "default_header_rows")]
    pub header_rows: usize,

    #[serde(default = "default_data_start_row")]
    pub data_start_row: usize,

    #[serde(default = "default_true")]
    pub skip_blank_id: bool,

    #[serde(default)]
    pub fill_merged: bool,

    pub id_cols: Vec<IdCol>,
    pub unpivot: Unpivot,
    pub value_rules: ValueRules,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatchRules {
    #[serde(default)]
    pub filename: Option<String>,

    #[serde(default)]
    pub header_contains: Vec<String>,

    #[serde(default)]
    pub header_signature: Option<String>,

    #[serde(default = "default_priority")]
    pub priority: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdCol {
    pub name: String,
    /// 1-indexed column position (matches Excel column letters: A=1).
    pub col: usize,
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default = "default_id_type", rename = "type")]
    pub kind: String,
}

impl IdCol {
    /// 0-indexed Vec position.
    pub fn index(&self) -> usize {
        self.col.saturating_sub(1)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Unpivot {
    pub anchor: String,

    #[serde(default)]
    pub anchor_match: AnchorMatch,

    pub day_cols: usize,

    #[serde(default)]
    pub validate_calendar_day: bool,

    #[serde(default = "default_var_name")]
    pub var_name: String,

    #[serde(default = "default_value_name")]
    pub value_name: String,

    /// "filename:0..6" or "constant:202604".
    pub year_month_from: String,

    #[serde(default)]
    pub anchor_col_observed: Option<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnchorMatch {
    #[default]
    Exact,
    Contains,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValueRules {
    #[serde(rename = "type")]
    pub kind: ValueKind,

    #[serde(default)]
    pub decimals: u32,

    #[serde(default, rename = "missingValues", alias = "missing_values")]
    pub missing_values: Vec<String>,

    #[serde(default = "default_true")]
    pub zero_is_valid: bool,

    #[serde(default)]
    pub min: Option<f64>,

    #[serde(default)]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValueKind {
    Decimal,
    Int,
    String,
}

fn default_encoding() -> String {
    "utf-8".into()
}
fn default_header_rows() -> usize {
    1
}
fn default_data_start_row() -> usize {
    2
}
fn default_priority() -> i32 {
    0
}
fn default_true() -> bool {
    true
}
fn default_id_type() -> String {
    "string".into()
}
fn default_var_name() -> String {
    "shift_hour".into()
}
fn default_value_name() -> String {
    "daily_kwh".into()
}

impl Profile {
    /// Parse the year_month directive against a file path or filename.
    pub fn extract_year_month(&self, path: &Path) -> Result<String, ReshapeError> {
        let directive = &self.unpivot.year_month_from;
        if let Some(spec) = directive.strip_prefix("filename:") {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let (start_s, end_s) = spec.split_once("..").ok_or_else(|| {
                ReshapeError::BadProfile(format!("bad year_month_from range '{}'", directive))
            })?;
            let start: usize = start_s
                .parse()
                .map_err(|_| ReshapeError::BadProfile(format!("bad range start: {}", start_s)))?;
            let end: usize = end_s
                .parse()
                .map_err(|_| ReshapeError::BadProfile(format!("bad range end: {}", end_s)))?;
            if end < start {
                return Err(ReshapeError::BadProfile(format!(
                    "year_month_from end < start: {}",
                    directive
                )));
            }
            let ym: String = filename.chars().skip(start).take(end - start).collect();
            if ym.len() != 6 || ym.chars().any(|c| !c.is_ascii_digit()) {
                return Err(ReshapeError::BadYearMonth(ym));
            }
            Ok(ym)
        } else if let Some(c) = directive.strip_prefix("constant:") {
            let ym = c.trim().to_string();
            if ym.len() != 6 || ym.chars().any(|c| !c.is_ascii_digit()) {
                return Err(ReshapeError::BadYearMonth(ym));
            }
            Ok(ym)
        } else {
            Err(ReshapeError::BadProfile(format!(
                "unsupported year_month_from: {}",
                directive
            )))
        }
    }

    pub fn caseid_col_index(&self) -> Result<usize, ReshapeError> {
        Ok(self
            .id_cols
            .first()
            .ok_or_else(|| ReshapeError::BadProfile("id_cols must have at least one entry".into()))?
            .index())
    }

    pub fn plant_name_col_index(&self) -> Option<usize> {
        self.id_cols.get(1).map(|c| c.index())
    }
}

pub struct Registry {
    profiles: Vec<LoadedProfile>,
}

pub struct LoadedProfile {
    pub path: PathBuf,
    pub profile: Profile,
}

#[derive(Debug)]
pub struct MatchResult<'a> {
    pub profile: &'a Profile,
    pub path: &'a Path,
    pub signature_ok: bool,
    pub actual_signature: String,
}

impl Registry {
    pub fn empty() -> Self {
        Self { profiles: Vec::new() }
    }

    pub fn from_profiles(profiles: Vec<LoadedProfile>) -> Self {
        Self { profiles }
    }

    pub fn iter(&self) -> impl Iterator<Item = &LoadedProfile> {
        self.profiles.iter()
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Route a file (by filename + first header row) to a single matching profile.
    /// Returns Err(NoMatch) / Err(AmbiguousMatch) per spec semantics.
    pub fn route<'a>(
        &'a self,
        filename: &str,
        header_row: &[String],
    ) -> Result<MatchResult<'a>, ProfileError> {
        let actual_signature = signature::compute(header_row);
        let mut hits: Vec<&LoadedProfile> = self
            .profiles
            .iter()
            .filter(|lp| match_rules::matches(&lp.profile, filename, header_row))
            .collect();

        if hits.is_empty() {
            return Err(ProfileError::NoMatch {
                file: filename.to_string(),
            });
        }

        if hits.len() > 1 {
            // Disambiguate by priority (higher wins).
            hits.sort_by_key(|lp| std::cmp::Reverse(lp.profile.match_rules.priority));
            if hits[0].profile.match_rules.priority == hits[1].profile.match_rules.priority {
                return Err(ProfileError::AmbiguousMatch {
                    file: filename.to_string(),
                    names: hits.iter().map(|lp| lp.profile.name.clone()).collect(),
                });
            }
        }

        let picked = hits[0];
        let signature_ok = match &picked.profile.match_rules.header_signature {
            Some(expected) => expected == &actual_signature,
            None => true,
        };

        Ok(MatchResult {
            profile: &picked.profile,
            path: &picked.path,
            signature_ok,
            actual_signature,
        })
    }
}

/// Load every *.json file in `dir` as a Profile and build a Registry.
pub fn load_dir(dir: &Path) -> Result<Registry, ProfileError> {
    if !dir.exists() {
        return Ok(Registry::empty());
    }
    let mut profiles = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| ProfileError::Io(dir.display().to_string(), e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| ProfileError::Io(dir.display().to_string(), e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path)
            .map_err(|e| ProfileError::Io(path.display().to_string(), e.to_string()))?;
        let profile: Profile = serde_json::from_slice(&bytes)
            .map_err(|e| ProfileError::Json(path.display().to_string(), e.to_string()))?;
        profiles.push(LoadedProfile {
            path: path.clone(),
            profile,
        });
    }
    Ok(Registry::from_profiles(profiles))
}

/// Convenience: extract first row of a grid, defaulting to empty vec.
pub fn first_row(grid: &Grid) -> Vec<String> {
    grid.first().cloned().unwrap_or_default()
}
