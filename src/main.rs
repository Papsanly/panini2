use derive_more::{Deref, DerefMut};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::{self, Display, Formatter},
    iter,
    time::{Duration, Instant},
};

trait Normalize {
    fn normalize(&self) -> Self;
}

impl Normalize for Vec<f32> {
    // Normalizes the vector so that its elements sum to 1. If the sum is 0, it returns the
    // original vector.
    fn normalize(&self) -> Self {
        let total = self.iter().sum::<f32>();
        if total < f32::EPSILON {
            return self.clone();
        }
        self.iter().map(|x| *x / total).collect()
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
struct Interval {
    start: Instant,
    end: Instant,
}

impl Interval {
    fn intercepts(&self, other: Interval) -> bool {
        self.start < other.end && other.start < self.end
    }

    fn len(&self) -> Duration {
        self.end - self.start
    }
}

#[derive(Hash, Debug, Eq, PartialEq)]
struct Task {
    description: String,
    deadline: Instant,
    intensity: u32,
    granularity: Duration,
}

#[derive(Deref, DerefMut)]
struct TaskChain(Vec<Task>);

#[derive(Deref, DerefMut)]
struct Schedule<'a> {
    #[deref]
    #[deref_mut]
    inner: HashMap<&'a Task, Vec<Interval>>,
    start: Instant,
}

impl<'a> Schedule<'a> {
    fn get_total_task_duration(&self, task: &'a Task) -> Duration {
        self.get(task)
            .map(|s| s.iter().map(|interval| interval.len()).sum::<Duration>())
            .unwrap_or_default()
    }

    fn get_target_tasks_distribution(&self, tasks: &[&'a Task]) -> Vec<f32> {
        tasks
            .iter()
            .map(|task| task.intensity as f32)
            .collect::<Vec<_>>()
            .normalize()
    }

    fn get_tasks_distribution(&self, tasks: &[&'a Task]) -> Vec<f32> {
        tasks
            .iter()
            .map(|task| self.get_total_task_duration(task).as_secs_f32())
            .collect::<Vec<_>>()
            .normalize()
    }

    fn schedule_tasks(&mut self, interval: Interval, task_distribution: Vec<(f32, &'a Task)>) {
        let mut task_start = interval.start;
        let total_duration = interval.len();
        for (task_time, task) in task_distribution {
            let task_schedule = self.entry(task).or_default();
            if task_time < f32::EPSILON {
                continue;
            }
            let task_duration = Duration::from_secs_f32(total_duration.as_secs_f32() * task_time);
            task_schedule.push(Interval {
                start: task_start,
                end: task_start + task_duration,
            });
            task_start += task_duration
        }
    }
}

impl Display for Schedule<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for (task, intervals) in self.iter() {
            writeln!(f, "{}:", task.description)?;
            for interval in intervals {
                writeln!(
                    f,
                    "    {:?} - {:?}",
                    (interval.start - self.start).as_secs(),
                    (interval.end - self.start).as_secs()
                )?;
            }
        }
        Ok(())
    }
}

struct ScheduleAlgorithm {
    start: Instant,
    task_chains: Vec<TaskChain>,
    idle_intervals: Vec<Interval>,
    max_critical_interval: Duration,
}

impl ScheduleAlgorithm {
    // Generates a vector of intervals derived from critical points such as task deadlines,
    // idle interval edges, and the scheduling algorithm's start time. It also divides excessively
    // large intervals into smaller sub-intervals. Long intervals could lead to overly coarse
    // scheduling decisions, with long intervals of doing only one task.
    fn get_critical_intervals(&self) -> Vec<Interval> {
        let mut critical_points = BTreeSet::new();

        for chain in &self.task_chains {
            for task in chain.iter() {
                critical_points.insert(task.deadline);
            }
        }

        for interval in &self.idle_intervals {
            critical_points.insert(interval.start);
            critical_points.insert(interval.end);
        }

        critical_points.insert(self.start);

        let mut curr = self.start;
        for point in critical_points.clone().into_iter().skip(1) {
            let interval = point - curr;
            let max_critical_interval_ratio =
                interval.as_secs() / self.max_critical_interval.as_secs();
            if max_critical_interval_ratio >= 1 {
                for i in 1..=max_critical_interval_ratio {
                    critical_points.insert(curr + (i as u32) * self.max_critical_interval);
                }
            }
            curr = point;
        }

        critical_points
            .iter()
            .zip(critical_points.iter().skip(1))
            .map(|(&start, &end)| Interval { start, end })
            .collect()
    }

    // The function finds, for each task chain, the task whose computed interval from the schedule
    // start or previous task deadline to its deadline overlaps the given interval.
    fn get_intercepting_tasks(&self, interval: Interval) -> Vec<&Task> {
        let mut res = Vec::new();
        for chain in &self.task_chains {
            let task_deadlines = chain.iter().map(|task| task.deadline);

            let mut task_intervals = iter::once(self.start)
                .chain(task_deadlines.clone())
                .zip(task_deadlines)
                .map(|(start, end)| Interval { start, end });

            if let Some(idx) = task_intervals.position(|i| i.intercepts(interval)) {
                res.push(&chain[idx])
            }
        }
        res
    }

    // The function creates a new distribution the tasks in a way that when added to current task
    // distribution minimizes the distance between the new task distribution and the target task
    // distribution. The distance is calculated as the sum of the absolute differences between
    // corresponding elements of the two distributions. Basically it is minkowski distance with p=1.
    // However, if this distance on some tasks is negative, it is set to 0, since we cannot
    // un-schedule tasks in the past. If the difference is negligible, it returns the target task
    // distribution.
    fn distribute_tasks(
        &self,
        current_task_distribution: Vec<f32>,
        target_task_distribution: Vec<f32>,
    ) -> Vec<f32> {
        let res = target_task_distribution
            .iter()
            .zip(current_task_distribution)
            .map(|(pd, d)| (pd - d).max(0.0));

        if res.clone().sum::<f32>() < f32::EPSILON {
            return target_task_distribution;
        }

        res.collect::<Vec<_>>().normalize()
    }

    fn is_intercepting_idle_interval(&self, interval: Interval) -> bool {
        self.idle_intervals.iter().any(|i| i.intercepts(interval))
    }

    // The function runs the scheduling algorithm, iterating over the critical intervals and
    // distributing tasks based on the current and target task distributions.
    fn run(&self) -> Schedule {
        let mut schedule = Schedule {
            inner: HashMap::with_capacity(self.task_chains.iter().map(|c| c.len()).sum()),
            start: self.start,
        };

        for interval in self.get_critical_intervals() {
            let intercepting_tasks = self.get_intercepting_tasks(interval);
            if self.is_intercepting_idle_interval(interval) {
                continue;
            }
            let current_dist = schedule.get_tasks_distribution(&intercepting_tasks);
            let target_dist = schedule.get_target_tasks_distribution(&intercepting_tasks);
            let new_task_dist = self.distribute_tasks(current_dist, target_dist);
            schedule.schedule_tasks(
                interval,
                new_task_dist.into_iter().zip(intercepting_tasks).collect(),
            );
        }

        schedule
    }
}

fn get_test_algorithm(now: Instant) -> ScheduleAlgorithm {
    let task1_chain = TaskChain(vec![
        Task {
            description: "Task 1".to_string(),
            deadline: now + Duration::new(3600, 0),
            intensity: 5,
            granularity: Duration::new(3600, 0),
        },
        Task {
            description: "Task 2".to_string(),
            deadline: now + Duration::new(7200, 0),
            intensity: 3,
            granularity: Duration::new(3600, 0),
        },
    ]);

    let task2_chain = TaskChain(vec![
        Task {
            description: "Task 3".to_string(),
            deadline: now + Duration::new(5400, 0),
            intensity: 4,
            granularity: Duration::new(3600, 0),
        },
        Task {
            description: "Task 4".to_string(),
            deadline: now + Duration::new(10800, 0),
            intensity: 2,
            granularity: Duration::new(3600, 0),
        },
    ]);

    let idle_intervals = vec![
        Interval {
            start: now + Duration::new(1800, 0),
            end: now + Duration::new(3600, 0),
        },
        Interval {
            start: now + Duration::new(7200, 0),
            end: now + Duration::new(10800, 0),
        },
    ];

    ScheduleAlgorithm {
        start: now,
        task_chains: vec![task1_chain, task2_chain],
        idle_intervals,
        max_critical_interval: Duration::new(1000, 0),
    }
}

fn main() {
    let now = Instant::now();
    let algorithm = get_test_algorithm(now);
    let schedule = algorithm.run();
    println!("{schedule}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_interval_generation() {
        let now = Instant::now();
        let algorithm = get_test_algorithm(now);
        let critical_intervals = algorithm.get_critical_intervals();

        assert_eq!(
            critical_intervals,
            vec![
                Interval {
                    start: now,
                    end: now + Duration::new(1000, 0)
                },
                Interval {
                    start: now + Duration::new(1000, 0),
                    end: now + Duration::new(1800, 0)
                },
                Interval {
                    start: now + Duration::new(1800, 0),
                    end: now + Duration::new(2800, 0)
                },
                Interval {
                    start: now + Duration::new(2800, 0),
                    end: now + Duration::new(3600, 0)
                },
                Interval {
                    start: now + Duration::new(3600, 0),
                    end: now + Duration::new(4600, 0)
                },
                Interval {
                    start: now + Duration::new(4600, 0),
                    end: now + Duration::new(5400, 0)
                },
                Interval {
                    start: now + Duration::new(5400, 0),
                    end: now + Duration::new(6400, 0)
                },
                Interval {
                    start: now + Duration::new(6400, 0),
                    end: now + Duration::new(7200, 0)
                },
                Interval {
                    start: now + Duration::new(7200, 0),
                    end: now + Duration::new(8200, 0)
                },
                Interval {
                    start: now + Duration::new(8200, 0),
                    end: now + Duration::new(9200, 0)
                },
                Interval {
                    start: now + Duration::new(9200, 0),
                    end: now + Duration::new(10200, 0)
                },
                Interval {
                    start: now + Duration::new(10200, 0),
                    end: now + Duration::new(10800, 0)
                },
            ]
        );
    }

    #[test]
    fn test_intercepting_tasks() {
        let now = Instant::now();
        let algorithm = get_test_algorithm(now);
        let critical_interval = algorithm.get_critical_intervals()[5];

        let intercepting_tasks = algorithm.get_intercepting_tasks(critical_interval);
        assert_eq!(
            intercepting_tasks,
            vec![&algorithm.task_chains[0][1], &algorithm.task_chains[1][0],]
        );
    }

    #[test]
    fn test_task_distribution() {
        let now = Instant::now();
        let algorithm = get_test_algorithm(now);

        let new_distribution = algorithm.distribute_tasks(
            vec![0.1, 0.3, 0.1].normalize(),
            vec![5.0, 3.0, 4.0].normalize(),
        );

        for (new_distribution, true_new_distribution) in new_distribution
            .iter()
            .zip([0.6190476, 0.0, 0.38095242].iter())
        {
            assert!((true_new_distribution - new_distribution).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_task_distribution_diff_is_zero() {
        let now = Instant::now();
        let algorithm = get_test_algorithm(now);

        let new_distribution = algorithm.distribute_tasks(
            vec![0.1, 0.3, 0.1].normalize(),
            vec![1.0, 3.0, 1.0].normalize(),
        );

        for (new_distribution, true_new_distribution) in
            new_distribution.iter().zip([0.2, 0.6, 0.2].iter())
        {
            assert!((true_new_distribution - new_distribution).abs() < f32::EPSILON);
        }
    }
}
