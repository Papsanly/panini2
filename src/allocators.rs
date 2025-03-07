use crate::{interval::Interval, schedule::TaskIdx, Schedule};
use jiff::{Span, Timestamp, ToSpan, Unit};

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
        let mut allocated_interval = Interval::new(current_time, self.granularity);

        let task = &schedule.tasks[task_idx];
        let work_hours = task.volume - schedule.get_total_task_hours(task_idx);
        let work_span = ((work_hours * 3600.0) as i32).seconds();

        if work_hours <= self.granularity.total(Unit::Hour).unwrap() as f32 {
            allocated_interval.span = work_span;
        }

        if current_time + work_span >= schedule.interval.end() {
            allocated_interval.span = schedule.interval.end() - current_time;
        }

        for idle_interval in &self.idle_intervals {
            if !allocated_interval.intercepts(idle_interval) {
                continue;
            } else if allocated_interval.timestamp >= idle_interval.timestamp {
                allocated_interval.timestamp = idle_interval.end();
            } else {
                allocated_interval.span = idle_interval.timestamp - allocated_interval.timestamp
            }
        }

        allocated_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{get_test_schedule, interval::Interval};
    use jiff::ToSpan;

    #[test]
    fn test_task_allocator() {
        let schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.timestamp;

        let allocator = TaskAllocator {
            granularity: 1.hour(),
            idle_intervals: vec![
                Interval::new(current_time, 2.hours()),
                Interval::new(current_time + 2.hours().minutes(50), 20.minutes()),
                Interval::new(current_time + 4.hours(), 1.hours()),
            ],
        };

        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(current_time + 2.hours(), 50.minutes()),
        );

        let current_time = schedule.interval.end() - 40.minutes();
        let task_idx = 1;
        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);
        assert_eq!(
            allocated_interval,
            Interval::new(current_time, 40.minutes())
        );

        let current_time = schedule.interval.timestamp + 2.hours();
        let task_idx = 5;
        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(current_time, 30.minutes())
        );

        let current_time = schedule.interval.timestamp + 4.hours().minutes(30);
        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(schedule.interval.timestamp + 5.hours(), 30.minutes())
        );
    }
}
