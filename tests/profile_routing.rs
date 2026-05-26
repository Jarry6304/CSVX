use solar_monitoring_import::profile::*;
use solar_monitoring_import::reader::Format;
use std::path::PathBuf;

fn make_profile(name: &str, filename_pat: &str, priority: i32, header_contains: Vec<String>) -> LoadedProfile {
    let profile = Profile {
        name: name.into(),
        match_rules: MatchRules {
            filename: Some(filename_pat.into()),
            header_contains,
            header_signature: None,
            priority,
        },
        encoding: "utf-8".into(),
        format: Format::Xlsx,
        sheet: Some(1),
        header_rows: 1,
        data_start_row: 2,
        skip_blank_id: true,
        fill_merged: false,
        id_cols: vec![IdCol {
            name: "caseid".into(),
            col: 1,
            header: None,
            kind: "string".into(),
        }],
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
            missing_values: vec!["".into()],
            zero_is_valid: true,
            min: Some(0.0),
            max: Some(1_000_000.0),
        },
    };
    LoadedProfile {
        path: PathBuf::from(format!("synthetic/{}.json", name)),
        profile,
    }
}

#[test]
fn routes_single_match_by_filename() {
    let reg = Registry::from_profiles(vec![
        make_profile("solar", "*發電量*.xlsx", 10, vec!["1日".into()]),
        make_profile("wind", "*風力*.xlsx", 10, vec!["1日".into()]),
    ]);
    let header: Vec<String> = vec!["案件編號".into(), "1日".into(), "31日".into()];
    let m = reg.route("202604_案場發電量.xlsx", &header).unwrap();
    assert_eq!(m.profile.name, "solar");
}

#[test]
fn no_match_returns_error() {
    let reg = Registry::from_profiles(vec![make_profile("solar", "*發電量*.xlsx", 10, vec![])]);
    let header: Vec<String> = vec![];
    let r = reg.route("invoice.xlsx", &header);
    assert!(r.is_err());
}

#[test]
fn priority_breaks_tie() {
    let reg = Registry::from_profiles(vec![
        make_profile("low", "*.xlsx", 1, vec![]),
        make_profile("high", "*.xlsx", 10, vec![]),
    ]);
    let m = reg.route("anything.xlsx", &[]).unwrap();
    assert_eq!(m.profile.name, "high");
}

#[test]
fn header_contains_filters_loosely() {
    let reg = Registry::from_profiles(vec![
        make_profile("solar", "*.xlsx", 10, vec!["案件編號".into(), "1日".into()]),
    ]);
    let header_with: Vec<String> = vec!["案件編號".into(), "1日".into()];
    let header_without: Vec<String> = vec!["foo".into(), "bar".into()];
    assert!(reg.route("anything.xlsx", &header_with).is_ok());
    assert!(reg.route("anything.xlsx", &header_without).is_err());
}

#[test]
fn ambiguous_match_errors() {
    let reg = Registry::from_profiles(vec![
        make_profile("a", "*.xlsx", 5, vec![]),
        make_profile("b", "*.xlsx", 5, vec![]),
    ]);
    let r = reg.route("any.xlsx", &[]);
    assert!(r.is_err());
}
