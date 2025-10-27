use anyhow::{Result, bail};
use time::format_description::well_known::Iso8601;
use time::macros::format_description;
pub use time::{Date, OffsetDateTime};

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn format_human(dt: OffsetDateTime) -> String {
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    dt.format(&format).unwrap_or_else(|_| dt.to_string())
}

pub fn format_iso(dt: OffsetDateTime) -> String {
    dt.format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| dt.to_string())
}

pub fn parse_date(input: &str) -> Result<OffsetDateTime> {
    if let Ok(dt) = OffsetDateTime::parse(input, &Iso8601::DEFAULT) {
        return Ok(dt);
    }
    let human_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    if let Ok(dt) = OffsetDateTime::parse(input, &human_format) {
        return Ok(dt);
    }
    let date_format = format_description!("[year]-[month]-[day]");
    if let Ok(date) = Date::parse(input, &date_format) {
        return Ok(date.midnight().assume_utc());
    }
    bail!("Unable to parse date: {input}")
}
