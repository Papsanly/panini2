use crate::{
    chrono::{from_chrono, to_chrono},
    interval::Interval,
    tasks::TaskIdx,
    Scheduler,
};
use croner::Cron;
use derive_more::Into;
use jiff::{civil::DateTime, tz::TimeZone, RoundMode, Span, Timestamp, ToSpan, Unit, ZonedRound};
use std::{collections::HashMap, error::Error};

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
        scheduler: &Scheduler,
        current_time: Timestamp,
        task_idx: TaskIdx,
    ) -> Interval {
        let mut allocated_interval = Interval::new(current_time, current_time + self.granularity);

        let task = &scheduler.tasks[task_idx];
        let work_hours = task.volume - scheduler.get_total_task_hours(task_idx);
        let work_span = ((work_hours * 3600.0) as i32).seconds();

        if work_hours
            <= self
                .granularity
                .total(Unit::Hour)
                .expect("Failed to get hours from granularity") as f32
        {
            allocated_interval.set_span(work_span);
        }

        if current_time + work_span >= scheduler.interval.end {
            allocated_interval.end = scheduler.interval.end;
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

impl TryFrom<(&Interval, HashMap<String, HashMap<String, String>>)> for Plans {
    type Error = Box<dyn Error>;

    fn try_from(
        (interval, value): (&Interval, HashMap<String, HashMap<String, String>>),
    ) -> Result<Self, Self::Error> {
        let mut plans = HashMap::new();
        for (cron_part, day_plans) in value {
            let cron_string = "0 0 ".to_string() + &cron_part;
            let cron = Cron::new(&cron_string).parse()?;

            for (time, description) in day_plans {
                for datetime in cron
                    .iter_from(to_chrono(interval.start))
                    .take_while(|dt| from_chrono(*dt) < interval.end)
                {
                    let date = from_chrono(datetime);
                    let [start, end]: [&str; 2] = time
                        .split('-')
                        .map(|v| v.trim())
                        .collect::<Vec<_>>()
                        .try_into()
                        .map_err(|e: Vec<_>| {
                            format!(
                                "Expected 2 elements separated by '\\', got {}: {:?}",
                                e.len(),
                                e
                            )
                        })?;
                    let start =
                        DateTime::strptime("%F %R", format!("{} {}", date.strftime("%F"), start))?
                            .to_zoned(TimeZone::system())
                            .unwrap()
                            .timestamp();

                    let end = if end.starts_with("24") {
                        (&date.to_zoned(TimeZone::system()) + 1.day())
                            .round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))
                            .unwrap()
                            .timestamp()
                    } else {
                        DateTime::strptime("%F %R", format!("{} {}", date.strftime("%F"), end))?
                            .to_zoned(TimeZone::system())
                            .unwrap()
                            .timestamp()
                    };

                    let plan_interval = Interval::new(start, end);
                    plans.insert(plan_interval, description.clone());
                }
            }
        }

        Ok(Plans(plans))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interval::Interval, tests::get_test_scheduler};
    use jiff::ToSpan;

    #[test]
    fn test_task_allocator() {
        let mut scheduler = get_test_scheduler();
        let task_idx = 0;
        let current_time = scheduler.interval.start;

        scheduler.allocator = TaskAllocatorWithPlans {
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
        let allocator = &scheduler.allocator;

        let allocated_interval = allocator.allocate(&scheduler, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(
                current_time + 2.hours(),
                current_time + 2.hours().minutes(50)
            ),
        );

        let current_time = scheduler.interval.end - 40.minutes();
        let task_idx = 1;
        let allocated_interval = allocator.allocate(&scheduler, current_time, task_idx);
        assert_eq!(
            allocated_interval,
            Interval::new(current_time, current_time + 40.minutes())
        );

        let current_time = scheduler.interval.start + 2.hours();
        let task_idx = 5;
        let allocated_interval = allocator.allocate(&scheduler, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(current_time, current_time + 30.minutes())
        );

        let current_time = scheduler.interval.start + 4.hours().minutes(30);
        let allocated_interval = allocator.allocate(&scheduler, current_time, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(
                scheduler.interval.start + 5.hours(),
                scheduler.interval.start + 5.hours().minutes(30)
            )
        );
    }
}
