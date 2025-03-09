use crate::{
    allocators::TaskAllocatorWithPlans,
    group_by::GroupBy,
    heuristics::Heuristic,
    interval::Interval,
    tasks::{Task, TaskIdx},
};
use derive_more::{Deref, DerefMut};
use jiff::{tz::TimeZone, RoundMode, Timestamp, Unit, ZonedRound};
use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
};

#[derive(Deref, DerefMut)]
pub struct Schedule {
    #[deref]
    #[deref_mut]
    inner: Vec<Vec<Interval>>,
    pub tasks: Vec<Task>,
    pub allocator: TaskAllocatorWithPlans,
    pub interval: Interval,
    pub current_time: Timestamp,
    pub heuristics: Vec<Heuristic>,
}

impl Schedule {
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

    pub fn schedule_task(&mut self, task_idx: TaskIdx, interval: Interval) {
        self[task_idx].push(interval);
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
                *score *= heuristic(self, self.current_time, task_idx);
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

        let interval = self.allocator.allocate(self, self.current_time, idx);

        self.current_time = interval.end;

        Some((idx, interval))
    }

    pub fn get_last_task(&self) -> Option<TaskIdx> {
        self.iter()
            .enumerate()
            .max_by_key(|(_, intervals)| {
                intervals
                    .iter()
                    .max_by_key(|interval| interval.end)
                    .map(|interval| interval.end)
            })
            .map(|(task_idx, _)| task_idx)
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
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut all_intervals = Vec::new();
        for (task_idx, intervals) in self.iter().enumerate() {
            for interval in intervals {
                all_intervals.push((task_idx, interval.clone()));
            }
        }

        all_intervals.sort_by_key(|(_, interval)| interval.start);

        let mut all_intervals_grouped = all_intervals
            .into_iter()
            .group_by(|(_, interval)| {
                interval
                    .start
                    .to_zoned(TimeZone::system())
                    .round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))
                    .expect("Failed to round timestamp")
            })
            .into_iter()
            .collect::<Vec<_>>();

        all_intervals_grouped.sort_by_key(|(day, _)| day.clone());

        for (day, mut task_intervals) in all_intervals_grouped {
            writeln!(f, "{}:", day.strftime("%F"))?;
            task_intervals.sort_by_key(|(_, interval)| interval.start);
            for (task_idx, interval) in task_intervals {
                let task = &self.tasks[task_idx];
                writeln!(
                    f,
                    "    {}: {} - {}",
                    task.description,
                    interval.start.to_zoned(TimeZone::system()).strftime("%R"),
                    interval.end.to_zoned(TimeZone::system()).strftime("%R"),
                )?;
            }
        }

        Ok(())
    }
}
