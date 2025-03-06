use crate::{allocators::TaskAllocator, heuristics::Heuristic, interval::Interval};
use derive_more::{Deref, DerefMut};
use jiff::Timestamp;
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    fmt::{Display, Formatter},
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
    tasks: Vec<Task>,
    allocator: TaskAllocator,
    heuristics: Vec<Heuristic>,
}

impl Schedule {
    pub fn new(allocator: TaskAllocator, tasks: Vec<Task>) -> Self {
        Self {
            inner: HashMap::new(),
            tasks,
            allocator,
            heuristics: Vec::new(),
        }
    }

    pub fn add_heuristic(mut self, heuristic: Heuristic) -> Self {
        self.heuristics.push(heuristic);
        self
    }

    pub fn get_task(&self, idx: TaskIdx) -> &Task {
        self.tasks
            .get(idx)
            .expect("Task index should not be out of bounds")
    }
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        todo!()
    }
}

pub struct SchedulerIter<'a> {
    schedule: &'a Schedule,
    interval: Interval,
    current_time: Timestamp,
}

impl<'a> Iterator for SchedulerIter<'a> {
    type Item = (TaskIdx, Interval);
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_time >= self.interval.end() {
            return None;
        }

        let mut heuristic_scores = vec![1.0; self.schedule.tasks.len()];

        for heuristic in &self.schedule.heuristics {
            for (task_idx, score) in heuristic_scores.iter_mut().enumerate() {
                *score *= heuristic(self.schedule, self.current_time, task_idx);
            }
        }

        let idx = heuristic_scores
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Less).reverse())
            .unwrap()
            .0;

        let interval = self
            .schedule
            .allocator
            .allocate(self.schedule, self.current_time, idx);

        self.current_time = interval.end();

        Some((idx, interval))
    }
}

pub fn scheduler_iter(schedule: &Schedule, interval: Interval) -> SchedulerIter {
    SchedulerIter {
        schedule,
        current_time: interval.timestamp,
        interval,
    }
}
