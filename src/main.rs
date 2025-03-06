use derive_more::{Deref, DerefMut};
use jiff::{Span, Timestamp, ToSpan};
use std::{
    cmp::Reverse,
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

trait Normalize {
    fn normalize(self) -> Self;
}

impl Normalize for Vec<f32> {
    // Normalizes the vector so that its elements sum to 1. If the sum is 0, it returns the
    // original vector.
    fn normalize(self) -> Self {
        let total = self.iter().sum::<f32>();
        if total < f32::EPSILON {
            return self;
        }
        self.into_iter().map(|x| x / total).collect()
    }
}

#[derive(Clone, Debug)]
struct Interval {
    timestamp: Timestamp,
    span: Span,
}

impl Interval {
    fn new(timestamp: Timestamp, span: Span) -> Self {
        Self { timestamp, span }
    }

    fn end(&self) -> Timestamp {
        self.timestamp + self.span
    }
}

struct Task {
    description: String,
    deadline: Timestamp,
    granularity: Span,
}

trait TaskAllocator {
    fn allocate(&self, schedule: &Schedule, task_idx: TaskIdx) -> Interval;
}

// this is a task allocator which takes into account idle intervals where tasks cannot be placed
struct IdleIntervalAllocator {
    idle_intervals: Vec<Interval>,
}

impl IdleIntervalAllocator {
    fn new(idle_intervals: Vec<Interval>) -> Self {
        Self { idle_intervals }
    }
}

impl TaskAllocator for IdleIntervalAllocator {
    fn allocate(&self, schedule: &Schedule, task_idx: TaskIdx) -> Interval {
        todo!()
    }
}

trait Heuristic {
    fn evaluate(&self, task: &Task) -> i32;
}

struct DependencyHeuristic {
    dependencies: HashMap<TaskIdx, Vec<TaskIdx>>,
}

impl DependencyHeuristic {
    fn new(dependencies: HashMap<TaskIdx, Vec<TaskIdx>>) -> Self {
        Self { dependencies }
    }
}

impl Heuristic for DependencyHeuristic {
    fn evaluate(&self, task: &Task) -> i32 {
        todo!()
    }
}

struct PriorityHeuristic {
    priorities: HashMap<TaskIdx, f32>,
}

impl PriorityHeuristic {
    fn new(priorities: HashMap<TaskIdx, f32>) -> Self {
        Self { priorities }
    }
}

impl Heuristic for PriorityHeuristic {
    fn evaluate(&self, task: &Task) -> i32 {
        todo!()
    }
}

struct DeadlineHeuristic;

impl Heuristic for DeadlineHeuristic {
    fn evaluate(&self, task: &Task) -> i32 {
        todo!()
    }
}

struct VolumeHeuristic {
    volumes: HashMap<TaskIdx, f32>,
}

impl VolumeHeuristic {
    fn new(volumes: HashMap<TaskIdx, f32>) -> Self {
        Self { volumes }
    }
}

impl Heuristic for VolumeHeuristic {
    fn evaluate(&self, task: &Task) -> i32 {
        todo!()
    }
}

type TaskIdx = usize;

#[derive(Deref, DerefMut)]
struct Schedule {
    #[deref]
    #[deref_mut]
    inner: HashMap<TaskIdx, Vec<Interval>>,
    tasks: Vec<Task>,
    allocator: Box<dyn TaskAllocator>,
    heuristics: Vec<Box<dyn Heuristic>>,
}

impl Schedule {
    fn new(allocator: impl TaskAllocator + 'static) -> Self {
        Self {
            inner: HashMap::new(),
            tasks: Vec::new(),
            allocator: Box::new(allocator),
            heuristics: Vec::new(),
        }
    }

    fn add_heuristic(mut self, heuristic: impl Heuristic + 'static) -> Self {
        self.heuristics.push(Box::new(heuristic));
        self
    }
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        todo!()
    }
}

struct SchedulerIter<'a> {
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

        let idx = self
            .schedule
            .tasks
            .iter()
            .enumerate()
            .min_by_key(|(_, task)| {
                Reverse(
                    self.schedule
                        .heuristics
                        .iter()
                        .map(|heuristic| heuristic.evaluate(task))
                        .reduce(|a, b| a * b)
                        .expect("at least one heuristic must be present"),
                )
            })?
            .0;

        Some((idx, self.schedule.allocator.allocate(self.schedule, idx)))
    }
}

fn scheduler_iter(schedule: &Schedule, interval: Interval) -> SchedulerIter {
    SchedulerIter {
        schedule,
        current_time: interval.timestamp,
        interval,
    }
}

fn main() {
    let (mut schedule, interval) = get_test_schedule();
    let scheduled_tasks: Vec<_> = scheduler_iter(&schedule, interval).collect();
    for (task_idx, task_interval) in scheduled_tasks {
        schedule.entry(task_idx).or_default().push(task_interval);
    }
    println!("{schedule}");
}

fn get_test_schedule() -> (Schedule, Interval) {
    let task1_chain = vec![
        Task {
            description: "Task 1".to_string(),
            deadline: "2025-03-05T12:00Z".parse().unwrap(),
            granularity: 1.hours(),
        },
        Task {
            description: "Task 2".to_string(),
            deadline: "2025-03-05T17:00Z".parse().unwrap(),
            granularity: 1.hours().minutes(30),
        },
    ];

    let task2_chain = vec![
        Task {
            description: "Task 3".to_string(),
            deadline: "2025-03-05T13:00Z".parse().unwrap(),
            granularity: 30.minutes(),
        },
        Task {
            description: "Task 4".to_string(),
            deadline: "2025-03-05T18:00Z".parse().unwrap(),
            granularity: 2.hours(),
        },
    ];

    let allocator = IdleIntervalAllocator::new(vec![
        Interval::new("2025-03-05T00:00Z".parse().unwrap(), 9.hours()),
        Interval::new("2025-03-05T13:00Z".parse().unwrap(), 2.hours()),
        Interval::new("2025-03-05T22:00Z".parse().unwrap(), 2.hours()),
    ]);

    let mut schedule = Schedule::new(allocator);

    let mut dependencies = HashMap::new();

    for task_chain in [task1_chain, task2_chain] {
        let mut prev_task_idx = None;
        for (idx, task) in task_chain.into_iter().enumerate() {
            schedule.tasks.push(task);
            if let Some(prev_task_idx) = prev_task_idx {
                dependencies.insert(idx, vec![prev_task_idx]);
            }
            prev_task_idx = Some(idx);
        }
    }

    schedule = schedule
        .add_heuristic(DependencyHeuristic::new(dependencies))
        .add_heuristic(DeadlineHeuristic)
        .add_heuristic(PriorityHeuristic::new(HashMap::from([
            (0, 1.0),
            (1, 1.0),
            (2, 2.0),
            (3, 1.0),
        ])))
        .add_heuristic(VolumeHeuristic::new(HashMap::from([
            (0, 1.0),
            (1, 0.5),
            (2, 0.5),
            (3, 1.0),
        ])));

    (
        schedule,
        Interval::new("2025-03-05T00:00Z".parse().unwrap(), 24.hours()),
    )
}

#[cfg(test)]
mod tests {}
