use solar_monitoring_import::profile;
use std::path::PathBuf;

#[test]
fn loads_shipped_solar_monthly_profile() {
    let dir: PathBuf = ["profiles"].iter().collect();
    let registry = profile::load_dir(&dir).expect("load profiles dir");
    assert!(registry.len() >= 1, "expected at least 1 profile in profiles/");

    let solar = registry
        .iter()
        .find(|lp| lp.profile.name == "solar_monthly")
        .expect("solar_monthly profile present");

    assert_eq!(solar.profile.unpivot.day_cols, 31);
    assert_eq!(solar.profile.unpivot.anchor, "1日");
    assert_eq!(solar.profile.id_cols.len(), 2);
    assert_eq!(
        solar.profile.match_rules.header_signature.as_deref(),
        Some("356bdc2a0ec6")
    );
    assert_eq!(solar.profile.value_rules.decimals, 4);
}
