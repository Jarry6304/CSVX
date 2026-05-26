# Profiles

Each `*.json` in this directory is a profile — a declarative description of a single
monthly-report layout. At runtime the engine loads every profile, peeks at each
incoming file's header, and routes the file to whichever profile matches.

Adding a new format = drop a new `*.json` here. The engine is **not** recompiled.

## Authoring

Two ways to create a profile:

1. **`import learn`** — point at a template + sample workbook and the binary will
   draft a JSON for you. Fields it can't infer get sensible defaults and may need
   manual review:
   ```
   import learn \
     --structure templates/foo/structure.xlsx \
     --sample    templates/foo/sample.xlsx \
     --out       profiles/foo.json \
     --name      foo
   ```

2. **Hand-write it** — copy `solar_monthly.json` and adjust. Useful when an LLM
   has already analyzed the template.

## Schema cheat-sheet

| Field | Notes |
|---|---|
| `name` | Unique identifier. |
| `match.filename` | Glob: `*`, `?`. e.g. `*發電量*.xlsx`. |
| `match.header_contains` | Tokens that must all appear in the header row (loose). |
| `match.header_signature` | sha1(headers joined by `|`), first 12 hex chars. Strict drift detector. |
| `match.priority` | Higher wins when multiple profiles match the same file. |
| `encoding` | `utf-8` / `big5` / `auto` (BOM detection). XLSX ignores this. |
| `format` | `csv` / `xlsx`. |
| `sheet` | 1-indexed sheet number (XLSX only). |
| `header_rows` | 1 for single-row headers; ≥2 to enable `fill_merged` ffill+concat. |
| `data_start_row` | 1-indexed first data row (use this to skip total/summary rows). |
| `skip_blank_id` | Skip rows where the first id column is blank. |
| `id_cols` | List of identity columns. `col` is **1-indexed** (Excel A=1, B=2…). |
| `unpivot.anchor` | The first day-column header text, e.g. `"1日"`. |
| `unpivot.anchor_match` | `exact` (default) or `contains`. |
| `unpivot.day_cols` | How many consecutive day columns follow the anchor. |
| `unpivot.validate_calendar_day` | If true, drop days that don't exist (e.g., Apr 31). |
| `unpivot.year_month_from` | `filename:0..6` extracts chars 0..6, or `constant:202604`. |
| `value_rules.type` | `decimal` / `int` / `string`. |
| `value_rules.missingValues` | Tokens treated as NULL. |
| `value_rules.min` / `.max` | Range guard (in unsupervised `_` decimal). |

Anything starting with `_` is a comment and ignored by the loader.
