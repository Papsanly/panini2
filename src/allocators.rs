use crate::{interval::Interval, schedule::TaskIdx, Schedule};
use derive_more::Into;
use jiff::{Span, Timestamp, ToSpan, Unit};
use std::{collections::HashMap, error::Error, str::FromStr};

pub struct TaskAllocatorWithPlans {
    pub granularity: Span,
    pub plans: HashMap<Interval, String>,
}

// allocates intervals for tasks with max length of `granularity`. avoids placing tasks on planed
// intervals. if available interval is smaller than `granularity`, the task will reduce the interval
// to fit it to available interval
impl TaskAllocatorWithPlans {
    pub fn allocate(
        &self,
        schedule: &Schedule,
        current_time: Timestamp,
        task_idx: TaskIdx,
    ) -> Interval {
        let mut allocated_interval = Interval::new(current_time, current_time + self.granularity);

        let task = &schedule.tasks[task_idx];
        let work_hours = task.volume - schedule.get_total_task_hours(task_idx);
        let work_span = ((work_hours * 3600.0) as i32).seconds();

        if work_hours
            <= self
                .granularity
                .total(Unit::Hour)
                .expect("Failed to get hours from granularity") as f32
        {
            allocated_interval.set_span(work_span);
        }

        if current_time + work_span >= schedule.interval.end {
            allocated_interval.end = schedule.interval.end;
        }

        let mut plan_intervals: Vec<_> = self.plans.keys().collect();
        plan_intervals.sort_by_key(|interval| interval.start);
        for plan_interval in plan_intervals {
            if !allocated_interval.intercepts(plan_interval) {
                continue;
            } else if allocated_interval.start >= plan_interval.start {
                allocated_interval.move_to(plan_interval.end);
            } else {
                allocated_interval.end = plan_interval.start
            }
        }

        allocated_interval
    }
}

#[derive(Into)]
pub struct Plans(HashMap<Interval, String>);

impl FromStr for Plans {
    type Err = Box<dyn Error>;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{get_test_schedule, interval::Interval};
    use jiff::ToSpan;

    #[test]
    fn test_task_allocator() {
        let mut schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.start;

        schedule.allocator = TaskAllocatorWithPlans {
            granularity: 1.hour(),
            plans: HashMap::from([
                (
                    Interval::new(current_time, current_time + 2.hours()),
                    "".into(),
                ),
                (
                    Interval::new(
                        current_time + 2.hours().minutes(50),
                        current_time + 3.hours().minutes(10),
                    ),
                    "".into(),
                ),
                (
                    Interval::new(current_time + 4.hours(), current_time + 5.hours()),
                    "".into(),
                ),
            ]),
        };
        let allocator = &schedule.allocator;

        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(
                current_time + 2.hours(),
                current_time + 2.hours().minutes(50)
            ),
        );

        let current_time = schedule.interval.end - 40.minutes();
        let task_idx = 1;
        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);
        assert_eq!(
            allocated_interval,
            Interval::new(current_time, current_time + 40.minutes())
        );

        let current_time = schedule.interval.start + 2.hours();
        let task_idx = 5;
        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(current_time, current_time + 30.minutes())
        );

        let current_time = schedule.interval.start + 4.hours().minutes(30);
        let allocated_interval = allocator.allocate(&schedule, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(
                schedule.interval.start + 5.hours(),
                schedule.interval.start + 5.hours().minutes(30)
            )
        );
    }
}
