use jiff::{tz::TimeZone, Span, Timestamp, Unit};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Interval {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl Interval {
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    pub fn from_span(start: Timestamp, span: Span) -> Self {
        Self {
            start,
            end: start + span,
        }
    }

    pub fn move_to(&mut self, start: Timestamp) {
        self.end += start - self.start;
        self.start = start;
    }

    pub fn span(&self) -> Span {
        self.end - self.start
    }

    pub fn hours(&self) -> f32 {
        self.span()
            .total((Unit::Hour, &self.start.to_zoned(TimeZone::system())))
            .expect("Failed to convert span to hours") as f32
    }

    pub fn set_span(&mut self, span: Span) {
        self.end = self.start + span;
    }

    pub fn intercepts(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}
