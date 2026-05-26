use super::{row::DailyKwhRow, validate_table_name, Dao};
use anyhow::Context;
use async_trait::async_trait;
use tiberius::{Client, Config, ToSql};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

const INSERT_CHUNK: usize = 500;

type MssqlClient = Client<Compat<TcpStream>>;

#[derive(Debug)]
pub struct SqlServerDao {
    ado: String,
    table: String,
}

impl SqlServerDao {
    pub fn new(conn_str: &str, table: &str) -> anyhow::Result<Self> {
        validate_table_name(table)?;
        // Eager parse to fail fast on a bad ADO string.
        let _ = Config::from_ado_string(conn_str)
            .with_context(|| "parsing connection string")?;
        Ok(Self {
            ado: conn_str.to_string(),
            table: table.to_string(),
        })
    }

    async fn connect(&self) -> anyhow::Result<MssqlClient> {
        let cfg = Config::from_ado_string(&self.ado)?;
        let tcp = TcpStream::connect(cfg.get_addr()).await?;
        tcp.set_nodelay(true)?;
        let client = Client::connect(cfg, tcp.compat_write()).await?;
        Ok(client)
    }

    async fn ensure_table(&self, client: &mut MssqlClient) -> anyhow::Result<()> {
        let table_param: &str = &self.table;
        let rs = client
            .query(
                "SELECT CAST(COUNT(*) AS INT) FROM sys.tables WHERE name = @P1",
                &[&table_param],
            )
            .await?;
        let row = rs.into_row().await?;
        let count: i32 = row.and_then(|r| r.get(0)).unwrap_or(0);
        if count > 0 {
            return Ok(());
        }
        let ddl = format!(
            "CREATE TABLE [{name}] (
                caseid     NVARCHAR(64)  NOT NULL,
                plant_name NVARCHAR(256) NULL,
                shift_hour CHAR(8)       NOT NULL,
                daily_kwh  DECIMAL(18,4) NULL
            )",
            name = self.table
        );
        client.execute(&ddl, &[]).await?;
        tracing::info!(table = %self.table, "created dev table");
        Ok(())
    }

    async fn insert_chunk(
        &self,
        client: &mut MssqlClient,
        chunk: &[DailyKwhRow],
    ) -> anyhow::Result<u64> {
        if chunk.is_empty() {
            return Ok(0);
        }
        let mut sql = format!(
            "INSERT INTO [{}] (caseid, plant_name, shift_hour, daily_kwh) VALUES ",
            self.table
        );
        for i in 0..chunk.len() {
            if i > 0 {
                sql.push(',');
            }
            let base = i * 4;
            use std::fmt::Write as _;
            let _ = write!(
                sql,
                "(@P{},@P{},@P{},@P{})",
                base + 1,
                base + 2,
                base + 3,
                base + 4
            );
        }
        let mut params: Vec<&dyn ToSql> = Vec::with_capacity(chunk.len() * 4);
        for r in chunk {
            params.push(&r.caseid);
            params.push(&r.plant_name);
            params.push(&r.shift_hour);
            params.push(&r.daily_kwh);
        }
        let result = client.execute(&sql, &params).await?;
        Ok(result.total())
    }
}

#[async_trait]
impl Dao for SqlServerDao {
    async fn upsert_month(&self, year_month: &str, rows: &[DailyKwhRow]) -> anyhow::Result<u64> {
        let mut client = self.connect().await?;
        self.ensure_table(&mut client).await?;
        let mut total = 0u64;
        for chunk in rows.chunks(INSERT_CHUNK) {
            total += self.insert_chunk(&mut client, chunk).await?;
        }
        tracing::info!(table = %self.table, year_month = %year_month, rows = total, "insert done");
        Ok(total)
    }
}
