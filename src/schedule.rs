use crate::{allocators::TaskAllocator, heuristics::Heuristic, interval::Interval};
use derive_more::{Deref, DerefMut};
use jiff::{
    civil::{Date, Weekday},
    tz::TimeZone,
    RoundMode, Timestamp, ToSpan, Unit, ZonedRound,
};
use std::{
    cmp::Ordering,
    collections::HashMap,
    error::Error,
    fmt,
    fmt::{Display, Formatter},
    hash::Hash,
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

impl FromStr for Schedule {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lines = s.split('\n');
        let mut tasks = Vec::new();

        let mut idx = 0;
        let mut has_dependency = false;
        for line in lines {
            if line.trim().is_empty() {
                has_dependency = false;
                continue;
            }

            if line.trim() == "|" {
                has_dependency = true;
                continue;
            }

            let parts: Vec<&str> = line.split('/').map(|p| p.trim()).collect();
            let [description, deadline, volume, completion]: [&str; 4] =
                parts.as_slice().try_into()?;

            let mut task = Task {
                description: description.to_string(),
                deadline: Date::strptime("%F", deadline)?
                    .to_zoned(TimeZone::system())?
                    .timestamp(),
                priority: 1.0,
                volume: volume[..volume.len() - 1].parse::<u32>()? as f32
                    * (1.0 - completion[..completion.len() - 1].parse::<u32>()? as f32 / 100.0),
                dependencies: Vec::new(),
            };

            if has_dependency {
                task.dependencies = vec![idx];
            }

            tasks.push(task);

            idx = tasks.len() - 1;
        }

        // todo: generate allocator config from .alloc file
        let interval = Interval::new(
            Date::strptime("%F", "2025-03-07")?
                .to_zoned(TimeZone::system())?
                .timestamp(),
            1.month(),
        );

        let mut idle_intervals = Vec::new();

        for day in 0..interval
            .span
            .total((Unit::Day, &interval.timestamp.to_zoned(TimeZone::system())))
            .unwrap() as i32
        {
            let zoned = &interval.timestamp.to_zoned(TimeZone::system()) + day.days();
            if zoned.weekday() == Weekday::Sunday {
                idle_intervals.push(Interval::new(zoned.timestamp(), 1.day()));
            } else {
                idle_intervals.push(Interval::new(zoned.timestamp(), 12.hours()));
                idle_intervals.push(Interval::new(zoned.timestamp() + 17.hour(), 2.hours()));
                idle_intervals.push(Interval::new(zoned.timestamp() + 22.hour(), 2.hours()));
            }
        }

        let allocator = TaskAllocator {
            granularity: 1.hour(),
            idle_intervals,
        };

        Ok(Self::new(allocator, tasks, interval))
    }
}

trait GroupBy<K, V> {
    fn group_by(self, key_fn: impl Fn(&V) -> K) -> HashMap<K, Vec<V>>;
}

impl<K: Hash + Eq, V: Clone, I: Iterator<Item = V>> GroupBy<K, V> for I {
    fn group_by(self, key_fn: impl Fn(&V) -> K) -> HashMap<K, Vec<V>> {
        let mut res: HashMap<K, Vec<V>> = HashMap::new();
        for item in self {
            let key = key_fn(&item);
            res.entry(key).or_default().push(item);
        }

        res
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

        let mut all_intervals_grouped = all_intervals
            .into_iter()
            .group_by(|(_, interval)| {
                interval
                    .timestamp
                    .to_zoned(TimeZone::system())
                    .round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))
                    .unwrap()
            })
            .into_iter()
            .collect::<Vec<_>>();

        all_intervals_grouped.sort_by_key(|(day, _)| day.clone());

        for (day, mut task_intervals) in all_intervals_grouped {
            writeln!(f, "{}:", day.strftime("%F"))?;
            task_intervals.sort_by_key(|(_, interval)| interval.timestamp);
            for (task_idx, interval) in task_intervals {
                let task = &self.tasks[*task_idx];
                writeln!(
                    f,
                    "    {}: {} - {}",
                    task.description,
                    interval
                        .timestamp
                        .to_zoned(TimeZone::system())
                        .strftime("%R"),
                    interval.end().to_zoned(TimeZone::system()).strftime("%R"),
                )?;
            }
        }

        Ok(())
    }
}
