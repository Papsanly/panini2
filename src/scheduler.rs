use crate::{
    allocators::{Plans, TaskAllocatorWithPlans},
    group_by::GroupBy,
    heuristics::Heuristic,
    interval::Interval,
    tasks::{Task, TaskIdx, Tasks},
};
use derive_more::{Deref, DerefMut};
use indexmap::IndexMap;
use jiff::{civil::DateTime, tz::TimeZone, RoundMode, Span, Timestamp, Unit, ZonedRound};
use serde::Deserialize;
use std::{cmp::Ordering, collections::BTreeMap, error::Error};

#[derive(Debug, Deserialize)]
pub struct SchedulerConfig {
    tasks: Vec<Vec<String>>,
    plans: IndexMap<String, IndexMap<String, String>>,
    granularity: String,
    start: String,
    end: String,
}

impl TryFrom<SchedulerConfig> for Scheduler {
    type Error = Box<dyn Error>;

    fn try_from(value: SchedulerConfig) -> Result<Self, Self::Error> {
        let interval = Interval::new(
            DateTime::strptime("%F %R", value.start)?
                .to_zoned(TimeZone::system())?
                .timestamp(),
            DateTime::strptime("%F %R", value.end)?
                .to_zoned(TimeZone::system())?
                .timestamp(),
        );

        let allocator = TaskAllocatorWithPlans {
            granularity: value.granularity.parse::<Span>()?,
            plans: Plans::try_from((&interval, value.plans))?.into(),
        };

        Ok(Self::new(
            allocator,
            Tasks::try_from(value.tasks)?.into(),
            interval,
        ))
    }
}

#[derive(Deref, DerefMut, Deserialize)]
#[serde(try_from = "SchedulerConfig")]
pub struct Scheduler {
    #[deref]
    #[deref_mut]
    inner: Vec<Vec<Interval>>,
    pub tasks: Vec<Task>,
    pub allocator: TaskAllocatorWithPlans,
    pub interval: Interval,
    pub current_time: Timestamp,
    pub heuristics: Vec<Heuristic>,
}

pub type Schedule = BTreeMap<String, BTreeMap<String, String>>;

impl From<&Scheduler> for Schedule {
    fn from(scheduler: &Scheduler) -> Self {
        let mut all_intervals = Vec::new();
        for (task_idx, intervals) in scheduler.inner.iter().enumerate() {
            for interval in intervals {
                all_intervals.push((scheduler.tasks[task_idx].description.clone(), interval));
            }
        }

        for (interval, description) in &scheduler.allocator.plans {
            all_intervals.push((description.clone(), interval));
        }

        all_intervals
            .into_iter()
            .group_by(|(_, interval)| {
                interval
                    .start
                    .to_zoned(TimeZone::system())
                    .round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))
                    .expect("Failed to round timestamp")
            })
            .into_iter()
            .map(|(day, intervals)| {
                (
                    day.strftime("%F").to_string(),
                    intervals
                        .into_iter()
                        .map(|(description, interval)| {
                            (
                                format!(
                                    "{} - {}",
                                    interval.start.to_zoned(TimeZone::system()).strftime("%R"),
                                    {
                                        let res = interval
                                            .end
                                            .to_zoned(TimeZone::system())
                                            .strftime("%R")
                                            .to_string();
                                        if res == "00:00" {
                                            "24:00".to_string()
                                        } else {
                                            res
                                        }
                                    }
                                ),
                                description,
                            )
                        })
                        .collect(),
                )
            })
            .collect()
    }
}

impl Scheduler {
    pub fn new(allocator: TaskAllocatorWithPlans, tasks: Vec<Task>, interval: Interval) -> Self {
        Self {
            inner: vec![Vec::new(); tasks.len()],
            tasks,
            allocator,
            current_time: interval.start,
            interval,
            heuristics: Vec::new(),
        }
    }

    pub fn schedule(&mut self) {
        while let Some((task_idx, task_interval)) = self.next() {
            self.schedule_task(task_idx, task_interval);
        }
    }

    pub fn schedule_task(&mut self, task_idx: TaskIdx, interval: Interval) {
        let Some(last_task) = self.iter().position(|intervals| {
            let Some(last_interval) = intervals.iter().max_by_key(|i| i.end) else {
                return false;
            };
            last_interval.end == interval.start
        }) else {
            self[task_idx].push(interval);
            return;
        };

        if last_task != task_idx {
            self[task_idx].push(interval);
            return;
        }

        let last_interval = self[task_idx]
            .iter_mut()
            .max_by_key(|interval| interval.end)
            .expect("Failed to find last interval");

        last_interval.end = interval.end;
    }

    // works by iterating over the tasks and applying heuristics to them. the task with the highest
    // heuristic score will be selected for scheduling. the heuristic scores are multiplied
    // together. allocator will allocate the interval for the task to be scheduled on.
    pub fn next(&mut self) -> Option<(TaskIdx, Interval)> {
        if self.current_time >= self.interval.end {
            return None;
        }

        let mut heuristic_scores = vec![1.0; self.tasks.len()];

        for heuristic in &self.heuristics {
            for (task_idx, score) in heuristic_scores.iter_mut().enumerate() {
                *score *= heuristic(self, task_idx);
            }
        }

        if heuristic_scores.iter().sum::<f32>() == 0.0 {
            return None;
        }

        let idx = heuristic_scores
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Less).reverse())
            .expect("Failed to find task with max heuristic score")
            .0;

        let interval = self.allocator.allocate(self, idx);

        self.current_time = interval.end;

        Some((idx, interval))
    }

    pub fn get_last_task(&self) -> Option<TaskIdx> {
        self.iter()
            .enumerate()
            .max_by_key(|(_, intervals)| intervals.iter().map(|interval| interval.end).max())
            .and_then(|(task_idx, intervals)| {
                if intervals.is_empty() {
                    None
                } else {
                    Some(task_idx)
                }
            })
    }

    pub fn add_heuristic(mut self, heuristic: Heuristic) -> Self {
        self.heuristics.push(heuristic);
        self
    }

    pub fn get_total_task_hours(&self, task_idx: TaskIdx) -> f32 {
        self.inner
            .get(task_idx)
            .map(|intervals| {
                intervals
                    .iter()
                    .map(|interval| interval.hours())
                    .sum::<f32>()
            })
            .unwrap_or(0.0)
    }

    pub fn get_planned_hours(&self, interval: Interval) -> f32 {
        self.allocator
            .plans
            .keys()
            .filter(|plan| plan.intercepts(&interval))
            .map(|plan| plan.hours())
            .sum::<f32>()
    }

    pub fn get_missed_deadlines_tasks(&self) -> Vec<TaskIdx> {
        self.tasks
            .iter()
            .enumerate()
            .filter(|(idx, task)| task.volume - self.get_total_task_hours(*idx) != 0.0)
            .map(|(idx, _)| idx)
            .collect()
    }
}
