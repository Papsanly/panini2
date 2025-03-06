use crate::{interval::Interval, schedule::TaskIdx, Schedule};
use jiff::Timestamp;

pub trait TaskAllocator {
    fn allocate(&self, schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx)
        -> Interval;
}

pub struct IdleIntervalAllocator {
    idle_intervals: Vec<Interval>,
}

impl IdleIntervalAllocator {
    pub fn new(idle_intervals: Vec<Interval>) -> Self {
        Self { idle_intervals }
    }
}

impl TaskAllocator for IdleIntervalAllocator {
    fn allocate(
        &self,
        schedule: &Schedule,
        current_time: Timestamp,
        task_idx: TaskIdx,
    ) -> Interval {
        todo!()
    }
}
