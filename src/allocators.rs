use crate::{
    chrono::{from_chrono, to_chrono},
    interval::Interval,
    tasks::TaskIdx,
    Scheduler,
};
use croner::Cron;
use derive_more::{Deref, DerefMut, Into};
use jiff::{civil::DateTime, tz::TimeZone, RoundMode, Span, ToSpan, Unit, ZonedRound};
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
};

pub struct TaskAllocatorWithPlans {
    pub granularity: Span,
    pub plans: BTreeMap<Interval, String>,
}

// allocates intervals for tasks with max length of `granularity`. avoids placing tasks on planed
// intervals. if available interval is smaller than `granularity`, the task will reduce the interval
// to fit it to available interval
impl TaskAllocatorWithPlans {
    pub fn allocate(&self, scheduler: &Scheduler, task_idx: TaskIdx) -> Interval {
        let mut allocated_interval = Interval::new(
            scheduler.current_time,
            scheduler.current_time + self.granularity,
        );

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

        if scheduler.current_time + work_span >= scheduler.interval.end {
            allocated_interval.end = scheduler.interval.end;
        }

        for plan_interval in self.plans.keys() {
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

#[derive(Into, Deref, DerefMut)]
pub struct Plans(BTreeMap<Interval, String>);

impl Plans {
    pub fn insert_with_overriding(&mut self, interval: Interval, description: String) {
        let contained_intervals: Vec<_> = self
            .keys()
            .filter(|&k| interval.contains(k))
            .cloned()
            .collect();

        for k in contained_intervals {
            self.remove(&k);
        }

        let is_contained_in_intervals: Vec<_> = self
            .iter()
            .filter(|(k, _)| k.contains(&interval))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (k, v) in is_contained_in_intervals {
            self.remove(&k);
            if k.start != interval.start {
                self.insert(Interval::new(k.start, interval.start), v.clone());
            }
            if k.end != interval.end {
                self.insert(Interval::new(interval.end, k.end), v);
            }
        }

        let partially_intercepted_intervals: Vec<_> = self
            .iter()
            .filter(|(k, _)| interval.partially_intercepts(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (k, v) in partially_intercepted_intervals {
            self.remove(&k);
            if k.start < interval.start {
                self.insert(Interval::new(k.start, interval.start), v.clone());
            }
            if k.end > interval.end {
                self.insert(Interval::new(interval.end, k.end), v);
            }
        }

        self.insert(interval, description);
    }
}

impl TryFrom<(&Interval, Vec<(String, HashMap<String, String>)>)> for Plans {
    type Error = Box<dyn Error>;

    fn try_from(
        (interval, value): (&Interval, Vec<(String, HashMap<String, String>)>),
    ) -> Result<Self, Self::Error> {
        let mut plans = Plans(BTreeMap::new());
        for (cron_part, day_plans) in value {
            let cron_string = "0 0 ".to_string() + &cron_part;
            let cron = Cron::new(&cron_string).parse()?;

            for (time, description) in day_plans {
                for datetime in cron
                    .iter_from(to_chrono(
                        interval
                            .start
                            .to_zoned(TimeZone::system())
                            .round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))
                            .unwrap()
                            .timestamp(),
                    ))
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
                    plans.insert_with_overriding(plan_interval, description.clone());
                }
            }
        }

        Ok(plans)
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

        scheduler.allocator = TaskAllocatorWithPlans {
            granularity: 1.hour(),
            plans: BTreeMap::from([
                (
                    Interval::from_span(scheduler.current_time, 2.hours()),
                    "".into(),
                ),
                (
                    Interval::from_span(
                        scheduler.current_time + 2.hours().minutes(50),
                        20.minutes(),
                    ),
                    "".into(),
                ),
                (
                    Interval::from_span(scheduler.current_time + 4.hours(), 1.hour()),
                    "".into(),
                ),
            ]),
        };
        let allocator = &scheduler.allocator;

        let allocated_interval = allocator.allocate(&scheduler, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::from_span(scheduler.current_time + 2.hours(), 50.minutes()),
        );

        scheduler.current_time = scheduler.interval.end - 40.minutes();
        let task_idx = 1;
        let allocated_interval = allocator.allocate(&scheduler, task_idx);
        assert_eq!(
            allocated_interval,
            Interval::from_span(scheduler.current_time, 40.minutes())
        );

        scheduler.current_time = scheduler.interval.start + 2.hours();
        let task_idx = 5;
        let allocated_interval = allocator.allocate(&scheduler, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::from_span(scheduler.current_time, 30.minutes())
        );

        scheduler.current_time = scheduler.interval.start + 4.hours().minutes(30);
        let allocated_interval = allocator.allocate(&scheduler, task_idx);

        assert_eq!(
            allocated_interval,
            Interval::new(
                scheduler.interval.start + 5.hours(),
                scheduler.interval.start + 5.hours().minutes(30)
            )
        );
    }
}
