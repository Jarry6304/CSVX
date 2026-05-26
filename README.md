# CSVX — Template-driven monthly-report importer (Rust)

Wide-table CSV/XLSX (one column per day) → long table in SQL Server, driven by
declarative `profiles/*.json`. The engine doesn't know what a "solar report" is;
adding a new format is a JSON file, not a code change.

## Architecture in one paragraph

`config.toml` + `.env` → `main.rs` wires a `Decoder`, a `FormatReader` (CSV or
XLSX), a `Dao` (SQL Server), and a profile `Registry`. For each file in
`input_dir`: peek the header, route to the matching profile, run `reshape::apply`
to unpivot wide → long, hand the rows to the DAO, and move the file to `bak/`.
Failures move to `error/` with a `.err.txt` chain.

Core modules (`app`, `reshape`, `profile`) never depend on `calamine`,
`tiberius`, or `encoding_rs` — only on traits and plain data.

## Quick start

```sh
cargo build --release
cp config.example.toml config.toml      # fill in <PLACEHOLDER>s
cp .env.example .env                    # set SMI_DB_PASSWORD

mkdir -p input bak error log

# Drop a monthly report into input/ then dry-run:
./target/release/import run --dry-run

# Real run (creates the dev table on first call, inserts rows):
./target/release/import run

# Generate a profile draft from a template workbook:
./target/release/import learn \
  --structure templates/foo/structure.xlsx \
  --sample    templates/foo/sample.xlsx \
  --out       profiles/foo.json \
  --name      foo
```

## DAO status: half-done

The shipped `SqlServerDao` does three things and three only:

1. Check if `[database].table` exists in `sys.tables`.
2. `CREATE TABLE` with a generic `(caseid, plant_name, shift_hour, daily_kwh)`
   schema if missing.
3. Batched `INSERT VALUES (...)` for every row.

There is no `DELETE`-by-month, no staging table, no transaction. Reruns
accumulate rows — that's the contract. A production-grade idempotent replace is
left to whoever has the real DB and schema access.

The table name must be `[A-Za-z0-9_]{1,128}` so the DDL is safe to interpolate.

## Profiles

`profiles/solar_monthly.json` is the calibrated reference (Taiwan-style monthly
report, single-row header + total row, BIG5-named files, 31 day columns with
calendar validation). Hand-write a sibling JSON or use `import learn` to draft
one from a template. See `profiles/README.md` for the schema cheat sheet.

## Layout

```
src/
├── app/         orchestration (scan / pipeline / move-to-bak / move-to-error)
├── cli.rs       clap subcommands: run / learn
├── config/      figment + toml + .env (password override)
├── dao/         trait + SqlServerDao + NoopDao (for dry-run)
├── decoder/     trait + Utf8 / Big5 / BOM-detect
├── error.rs     thiserror types (ReshapeError, DaoError, ProfileError)
├── grid.rs      type Grid = Vec<Vec<String>>
├── log.rs       tracing → console + rolling daily file
├── main.rs      composition root (~60 lines)
├── profile/     types + registry + matching + signature + learn
├── reader/      trait + CsvReader + XlsxReader (calamine, boxed)
└── reshape/     header promote/ffill + anchor + calendar + cast + apply

tests/           integration tests (profile loading/routing, e2e dry-run)
profiles/        the shipped solar_monthly.json + authoring README
```

## Secrets / privacy

`config.toml` is gitignored. The example uses `<PLACEHOLDER>`s for host, DB,
user, and table. `.env` (also gitignored) supplies `SMI_DB_PASSWORD`, which
substitutes `<REPLACED_BY_ENV>` in the connection string at startup.

No real schema, table names, or sample reports live in this repo.

## Tests

```sh
cargo test        # 47 tests: 38 unit + 9 integration
cargo test --release
```

The engine is pinned by synthetic-Grid unit tests (`reshape::tests`, etc.). DAO
live tests are `#[ignore]` by default — wire up `MSSQL_URL` + a dev DB to run
them.

## License

See `LICENSE`.
