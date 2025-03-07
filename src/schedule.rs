use crate::{allocators::TaskAllocator, heuristics::Heuristic, interval::Interval};
use derive_more::{Deref, DerefMut};
use jiff::{tz::TimeZone, Timestamp, Unit};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    fmt::{Display, Formatter},
    str::FromStr,
};

pub struct Task {
    pub description: String,
    pub deadline: Timestamp,
    pub priority: f32,
    pub volume: f32,
    pub dependencies: Vec<TaskIdx>,
}

pub type TaskIdx = usize;

#[derive(Deref, DerefMut)]
pub struct Schedule {
    #[deref]
    #[deref_mut]
    inner: HashMap<TaskIdx, Vec<Interval>>,
    pub tasks: Vec<Task>,
    allocator: TaskAllocator,
    pub interval: Interval,
    pub current_time: Timestamp,
    heuristics: Vec<Heuristic>,
}

impl Schedule {
    pub fn new(allocator: TaskAllocator, tasks: Vec<Task>, interval: Interval) -> Self {
        Self {
            inner: HashMap::new(),
            tasks,
            allocator,
            current_time: interval.timestamp,
            interval,
            heuristics: Vec::new(),
        }
    }

    // works by iterating over the tasks and applying heuristics to them. the task with the highest
    // heuristic score will be selected for scheduling. the heuristic scores are multiplied
    // together. allocator will allocate the interval for the task to be scheduled on.
    pub fn next(&mut self) -> Option<(TaskIdx, Interval)> {
        if self.current_time >= self.interval.end() {
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
            .unwrap()
            .0;

        let interval = self.allocator.allocate(self, self.current_time, idx);

        self.current_time = interval.end();

        Some((idx, interval))
    }

    pub fn add_heuristic(mut self, heuristic: Heuristic) -> Self {
        self.heuristics.push(heuristic);
        self
    }

    pub fn get_total_task_hours(&self, task_idx: TaskIdx) -> f32 {
        self.inner
            .get(&task_idx)
            .map(|intervals| {
                intervals
                    .iter()
                    .map(|interval| {
                        interval
                            .span
                            .total((Unit::Hour, &interval.timestamp.to_zoned(TimeZone::system())))
                            .unwrap()
                    })
                    .sum::<f64>()
            })
            .unwrap_or(0.0) as f32
    }

    pub fn get_idle_hours(&self, interval: Interval) -> f32 {
        self.allocator
            .idle_intervals
            .iter()
            .filter(|idle_interval| idle_interval.intercepts(&interval))
            .map(|idle_interval| {
                idle_interval
                    .span
                    .total((
                        Unit::Hour,
                        &idle_interval.timestamp.to_zoned(TimeZone::system()),
                    ))
                    .unwrap()
            })
            .sum::<f64>() as f32
    }
}

#[derive(Debug)]
pub struct ScheduleParsingError;

impl FromStr for Schedule {
    type Err = ScheduleParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut all_intervals = Vec::new();
        for (task_idx, intervals) in self.iter() {
            for interval in intervals {
                all_intervals.push((task_idx, interval.clone()));
            }
        }

        all_intervals.sort_by_key(|(_, interval)| interval.timestamp);

        for (task_idx, interval) in all_intervals {
            let task = &self.tasks[*task_idx];
            writeln!(
                f,
                "Task {}: {} - {} ({} {})",
                task.description,
                interval.timestamp,
                interval.end(),
                interval
                    .span
                    .total((Unit::Hour, &interval.timestamp.to_zoned(TimeZone::system())))
                    .unwrap(),
                if interval.span.total(Unit::Hour).unwrap() == 1.0 {
                    "hour"
                } else {
                    "hours"
                }
            )?;
        }

        Ok(())
    }
}
