//! Wide-to-long unpivot row emitter. The actual loop lives in [`super::apply`] for
//! locality with header / anchor lookup; this file holds the row constructor helper.

use crate::dao::row::DailyKwhRow;
use rust_decimal::Decimal;

pub fn make_row(
    caseid: &str,
    plant_name: Option<&str>,
    year_month: &str,
    day: u32,
    value: Option<Decimal>,
) -> DailyKwhRow {
    DailyKwhRow {
        caseid: caseid.to_string(),
        plant_name: plant_name.map(|s| s.to_string()),
        shift_hour: format!("{}{:02}", year_month, day),
        daily_kwh: value,
    }
}
