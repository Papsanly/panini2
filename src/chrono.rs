use chrono::{DateTime, Utc};
use jiff::Timestamp;

pub fn to_chrono(timestamp: Timestamp) -> DateTime<Utc> {
    let seconds = timestamp.as_second();
    DateTime::from_timestamp(seconds, 0).unwrap()
}

pub fn from_chrono(datetime: DateTime<Utc>) -> Timestamp {
    let seconds = datetime.timestamp();
    Timestamp::from_second(seconds).unwrap()
}
