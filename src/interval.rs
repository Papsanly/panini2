use jiff::{tz::TimeZone, Span, Timestamp};

#[derive(Clone, Debug)]
pub struct Interval {
    pub timestamp: Timestamp,
    pub span: Span,
}

impl PartialEq for Interval {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self
                .span
                .compare((other.span, &self.timestamp.to_zoned(TimeZone::system())))
                .unwrap()
                .is_eq()
    }
}

impl Interval {
    pub fn new(timestamp: Timestamp, span: Span) -> Self {
        Self { timestamp, span }
    }

    pub fn end(&self) -> Timestamp {
        self.timestamp + self.span
    }

    pub fn intercepts(&self, other: &Self) -> bool {
        self.timestamp < other.end() && other.timestamp < self.end()
    }
}
