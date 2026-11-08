pub mod cases;
pub mod events;
pub mod notes;
pub mod query;
pub mod seed;

pub use oc_kit::OcError;

pub fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

pub fn ms(t: time::OffsetDateTime) -> i64 {
    i64::try_from(t.unix_timestamp_nanos() / 1_000_000).unwrap_or(i64::MAX)
}

pub fn from_ms(ms: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
}
