use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyKwhRow {
    pub caseid: String,
    pub plant_name: Option<String>,
    /// "YYYYMMDD".
    pub shift_hour: String,
    pub daily_kwh: Option<Decimal>,
}
