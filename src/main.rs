mod allocators;
mod group_by;
mod heuristics;
mod interval;
mod schedule;
mod tasks;

use crate::{
    allocators::{Plans, TaskAllocatorWithPlans},
    interval::Interval,
    schedule::Schedule,
    tasks::{Task, Tasks},
};
use jiff::{RoundMode, ToSpan, Unit, Zoned, ZonedRound};
use std::{collections::HashMap, error::Error, fs};

const TASKS_FILE: &str = "data/tasks.yaml";
const PLANS_FILE: &str = "data/plans.yaml";
const SCHEDULE_FILE: &str = "data/schedule.yaml";

fn run() -> Result<(), Box<dyn Error>> {
    let tasks: Tasks = fs::read_to_string(TASKS_FILE)?.parse()?;
    let plans: Plans = fs::read_to_string(PLANS_FILE)?.parse()?;

    let allocator = TaskAllocatorWithPlans {
        plans: plans.into(),
        granularity: 1.hour(),
    };

    let start = Zoned::now()
        .round(ZonedRound::new().smallest(Unit::Day).mode(RoundMode::Trunc))?
        .timestamp();
    let interval = Interval::new(start, start + 1.month());

    let mut schedule = Schedule::new(allocator, tasks.into(), interval)
        .add_heuristic(heuristics::dependency)
        .add_heuristic(heuristics::volume)
        .add_heuristic(heuristics::deadline)
        .add_heuristic(heuristics::priority)
        .add_heuristic(heuristics::locality);

    loop {
        let next_item = schedule.next();
        match next_item {
            Some((task_idx, task_interval)) => schedule.schedule_task(task_idx, task_interval),
            None => break,
        }
    }

    fs::write(SCHEDULE_FILE, schedule.to_string())?;
    Ok(())
}

fn main() {
    run().expect("Failed to run the scheduler");
}

pub fn get_test_schedule() -> Schedule {
    let tasks = vec![
        Task {
            description: "Task 0".to_string(),
            deadline: "2025-03-05T12:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 2.0,
            dependencies: vec![4],
        },
        Task {
            description: "Task 1".to_string(),
            deadline: "2025-03-05T17:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 1.0,
            dependencies: vec![0],
        },
        Task {
            description: "Task 2".to_string(),
            deadline: "2025-03-05T13:00Z".parse().unwrap(),
            priority: 2.0,
            volume: 3.0,
            dependencies: vec![],
        },
        Task {
            description: "Task 3".to_string(),
            deadline: "2025-03-05T18:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 3.0,
            dependencies: vec![2],
        },
        Task {
            description: "Empty task".to_string(),
            deadline: "2025-03-05T19:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 0.0,
            dependencies: vec![],
        },
        Task {
            description: "Zero priority task".to_string(),
            deadline: "2025-03-05T19:00Z".parse().unwrap(),
            priority: 0.0,
            volume: 0.5,
            dependencies: vec![],
        },
    ];

    let allocator = TaskAllocatorWithPlans {
        plans: HashMap::from([
            (
                Interval::from_span("2025-03-05T00:00Z".parse().unwrap(), 9.hours()),
                "".into(),
            ),
            (
                Interval::from_span("2025-03-05T13:00Z".parse().unwrap(), 2.hours()),
                "".into(),
            ),
            (
                Interval::from_span("2025-03-05T22:00Z".parse().unwrap(), 2.hours()),
                "".into(),
            ),
        ]),
        granularity: 1.hour(),
    };

    let interval = Interval::from_span("2025-03-05T00:00Z".parse().unwrap(), 24.hours());

    Schedule::new(allocator, tasks, interval)
        .add_heuristic(heuristics::dependency)
        .add_heuristic(heuristics::volume)
        .add_heuristic(heuristics::deadline)
        .add_heuristic(heuristics::priority)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_iter() {
        let mut schedule = get_test_schedule();
        let mut scheduled_tasks = Vec::new();
        loop {
            let next_item = schedule.next();
            match next_item {
                Some((task_idx, task_interval)) => {
                    scheduled_tasks.push((task_idx, task_interval.clone()));
                    schedule.schedule_task(task_idx, task_interval)
                }
                None => break,
            }
        }

        assert_eq!(
            scheduled_tasks,
            vec![
                (
                    2,
                    Interval::from_span("2025-03-05T09:00Z".parse().unwrap(), 1.hour())
                ),
                (
                    2,
                    Interval::from_span("2025-03-05T10:00Z".parse().unwrap(), 1.hour())
                ),
                (
                    0,
                    Interval::from_span("2025-03-05T11:00Z".parse().unwrap(), 1.hour())
                ),
                (
                    2,
                    Interval::from_span("2025-03-05T12:00Z".parse().unwrap(), 1.hour())
                ),
                (
                    3,
                    Interval::from_span("2025-03-05T15:00Z".parse().unwrap(), 1.hour())
                ),
                (
                    1,
                    Interval::from_span("2025-03-05T16:00Z".parse().unwrap(), 1.hour())
                ),
                (
                    3,
                    Interval::from_span("2025-03-05T17:00Z".parse().unwrap(), 1.hour())
                ),
            ]
        );
    }
}
