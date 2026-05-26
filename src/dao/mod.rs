pub mod mssql;
pub mod row;

use crate::config::DatabaseCfg;
use crate::error::DaoError;
use async_trait::async_trait;
use row::DailyKwhRow;

#[async_trait]
pub trait Dao: Send + Sync {
    /// Half-done: ensure target table exists, then batch INSERT rows.
    /// `year_month` is logging context; idempotent replace is out of scope.
    async fn upsert_month(&self, year_month: &str, rows: &[DailyKwhRow]) -> anyhow::Result<u64>;
}

pub fn make_dao(cfg: &DatabaseCfg) -> anyhow::Result<Box<dyn Dao>> {
    if !cfg.is_enabled(&cfg.target) {
        return Err(DaoError::GateClosed(cfg.target.clone()).into());
    }
    match cfg.target.as_str() {
        "mssql" => Ok(Box::new(mssql::SqlServerDao::new(&cfg.conn, &cfg.table)?)),
        other => Err(DaoError::NotImplemented(other.to_string()).into()),
    }
}

/// A do-nothing Dao for `--dry-run`: counts rows but never touches the database.
#[derive(Debug, Default)]
pub struct NoopDao;

#[async_trait]
impl Dao for NoopDao {
    async fn upsert_month(&self, _year_month: &str, _rows: &[DailyKwhRow]) -> anyhow::Result<u64> {
        Ok(0)
    }
}

/// Whitelist table names so the half-done DDL builder never injects unsafe SQL.
pub(crate) fn validate_table_name(name: &str) -> Result<(), DaoError> {
    if name.is_empty() || name.len() > 128 {
        return Err(DaoError::InvalidTable(name.to_string()));
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(DaoError::InvalidTable(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ok() {
        assert!(validate_table_name("dev_kwh_test").is_ok());
        assert!(validate_table_name("ABC123").is_ok());
    }

    #[test]
    fn validate_rejects_unsafe() {
        assert!(validate_table_name("").is_err());
        assert!(validate_table_name("a;b").is_err());
        assert!(validate_table_name("foo bar").is_err());
        assert!(validate_table_name("foo]").is_err());
        assert!(validate_table_name("中文表").is_err());
    }
}
