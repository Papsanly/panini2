use jiff::{Span, Timestamp};

#[derive(Clone, Debug)]
pub struct Interval {
    pub timestamp: Timestamp,
    pub span: Span,
}

impl Interval {
    pub fn new(timestamp: Timestamp, span: Span) -> Self {
        Self { timestamp, span }
    }

    pub fn end(&self) -> Timestamp {
        self.timestamp + self.span
    }
}
