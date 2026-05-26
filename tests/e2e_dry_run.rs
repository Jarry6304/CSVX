use async_trait::async_trait;
use solar_monitoring_import::config::{Config, DatabaseCfg, LogCfg, ProfileCfg, SourceCfg};
use solar_monitoring_import::dao::{row::DailyKwhRow, Dao};
use solar_monitoring_import::{app, profile};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
struct RecordingDao {
    calls: Mutex<Vec<(String, Vec<DailyKwhRow>)>>,
}

#[async_trait]
impl Dao for RecordingDao {
    async fn upsert_month(&self, ym: &str, rows: &[DailyKwhRow]) -> anyhow::Result<u64> {
        self.calls
            .lock()
            .unwrap()
            .push((ym.to_string(), rows.to_vec()));
        Ok(rows.len() as u64)
    }
}

const PROFILE_JSON: &str = r#"{
  "name": "test_csv",
  "match": { "filename": "*.csv", "header_contains": ["1日"], "priority": 1 },
  "encoding": "utf-8",
  "format": "csv",
  "header_rows": 1,
  "data_start_row": 2,
  "skip_blank_id": true,
  "fill_merged": false,
  "id_cols": [
    { "name": "caseid", "col": 1, "type": "string" },
    { "name": "plant_name", "col": 2, "type": "string" }
  ],
  "unpivot": {
    "anchor": "1日", "anchor_match": "exact", "day_cols": 3,
    "validate_calendar_day": true,
    "var_name": "shift_hour", "value_name": "daily_kwh",
    "year_month_from": "filename:0..6"
  },
  "value_rules": {
    "type": "decimal", "decimals": 2,
    "missingValues": [""],
    "zero_is_valid": true, "min": 0, "max": 10000
  }
}"#;

fn build_cfg(tmp: &std::path::Path) -> Config {
    Config {
        source: SourceCfg {
            input_dir: tmp.join("input"),
            backup_dir: tmp.join("bak"),
            error_dir: tmp.join("error"),
        },
        database: DatabaseCfg {
            target: "mssql".into(),
            conn: "".into(),
            table: "unused".into(),
            enabled: HashMap::new(),
        },
        log: LogCfg {
            dir: tmp.join("log"),
            level: "info".into(),
        },
        profile: ProfileCfg {
            dir: tmp.join("profiles"),
        },
    }
}

#[tokio::test]
async fn dry_run_emits_no_insert_calls() {
    let tmp = tempfile::tempdir().unwrap();
    let pdir = tmp.path().join("profiles");
    let idir = tmp.path().join("input");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::create_dir_all(&idir).unwrap();
    std::fs::write(pdir.join("test.json"), PROFILE_JSON).unwrap();
    let csv = "caseid,plant_name,1日,2日,3日\nA001,PlantA,10.5,20.5,30.5\n";
    std::fs::write(idir.join("202604_x.csv"), csv).unwrap();

    let cfg = build_cfg(tmp.path());
    let registry = profile::load_dir(&pdir).unwrap();
    let dao = RecordingDao::default();

    let summary = app::run(&cfg, &registry, &dao, None, true).await.unwrap();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.rows_total, 3);
    assert_eq!(dao.calls.lock().unwrap().len(), 0); // dry-run = no insert
    // File should NOT be moved on dry-run.
    assert!(idir.join("202604_x.csv").exists());
}

#[tokio::test]
async fn run_calls_dao_and_moves_to_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let pdir = tmp.path().join("profiles");
    let idir = tmp.path().join("input");
    let bdir = tmp.path().join("bak");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::create_dir_all(&idir).unwrap();
    std::fs::write(pdir.join("test.json"), PROFILE_JSON).unwrap();
    let csv = "caseid,plant_name,1日,2日,3日\nA001,PlantA,10.5,20.5,30.5\n";
    std::fs::write(idir.join("202604_x.csv"), csv).unwrap();

    let cfg = build_cfg(tmp.path());
    let registry = profile::load_dir(&pdir).unwrap();
    let dao = RecordingDao::default();

    let summary = app::run(&cfg, &registry, &dao, None, false).await.unwrap();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.rows_total, 3);
    let calls = dao.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let (ym, rows) = &calls[0];
    assert_eq!(ym, "202604");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].caseid, "A001");
    assert_eq!(rows[0].shift_hour, "20260401");
    drop(calls);

    // Original file moved out; a date-suffixed copy lives in bak/.
    assert!(!idir.join("202604_x.csv").exists());
    let backup_count = std::fs::read_dir(&bdir).unwrap().count();
    assert_eq!(backup_count, 1);
}

#[tokio::test]
async fn april_drops_day_31() {
    let tmp = tempfile::tempdir().unwrap();
    let pdir = tmp.path().join("profiles");
    let idir = tmp.path().join("input");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::create_dir_all(&idir).unwrap();

    // Profile with 31 day_cols and calendar validation.
    let profile_31 = r#"{
      "name": "csv31",
      "match": { "filename": "*.csv", "header_contains": ["1日","31日"], "priority": 1 },
      "encoding": "utf-8", "format": "csv",
      "header_rows": 1, "data_start_row": 2,
      "skip_blank_id": true, "fill_merged": false,
      "id_cols": [
        { "name": "caseid", "col": 1, "type": "string" },
        { "name": "plant_name", "col": 2, "type": "string" }
      ],
      "unpivot": {
        "anchor": "1日", "anchor_match": "exact", "day_cols": 31,
        "validate_calendar_day": true,
        "var_name": "shift_hour", "value_name": "daily_kwh",
        "year_month_from": "filename:0..6"
      },
      "value_rules": {
        "type": "decimal", "decimals": 4,
        "missingValues": [""],
        "zero_is_valid": true, "min": 0, "max": 100000
      }
    }"#;
    std::fs::write(pdir.join("p.json"), profile_31).unwrap();

    let mut header_cols: Vec<String> = vec!["caseid".into(), "plant_name".into()];
    let mut data_cols: Vec<String> = vec!["A001".into(), "PlantA".into()];
    for d in 1..=31 {
        header_cols.push(format!("{}日", d));
        data_cols.push(format!("{}.0", d));
    }
    let csv = format!("{}\n{}\n", header_cols.join(","), data_cols.join(","));
    std::fs::write(idir.join("202604_april.csv"), &csv).unwrap();

    let cfg = build_cfg(tmp.path());
    let registry = profile::load_dir(&pdir).unwrap();
    let dao = RecordingDao::default();

    let summary = app::run(&cfg, &registry, &dao, None, true).await.unwrap();
    assert_eq!(summary.rows_total, 30, "April has 30 days, not 31");
}
