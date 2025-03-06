use crate::{interval::Interval, schedule::TaskIdx, Schedule};
use jiff::{Span, Timestamp};

pub struct TaskAllocator {
    pub granularity: Span,
    pub idle_intervals: Vec<Interval>,
}

// allocates intervals for tasks with max length of `granularity`. avoids placing tasks on idle
// intervals. if available interval is smaller than `granularity`, the task will be split into
// multiple intervals to fit tight available intervals, with each of them having total length of
// `granularity`.
impl TaskAllocator {
    pub fn allocate(
        &self,
        schedule: &Schedule,
        current_time: Timestamp,
        task_idx: TaskIdx,
    ) -> Interval {
        todo!()
    }
}
