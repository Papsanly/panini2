#![cfg(test)]
use crate::{
    allocators::TaskAllocatorWithPlans, heuristics, interval::Interval, scheduler::Scheduler,
    tasks::Task,
};
use jiff::ToSpan;
use std::collections::HashMap;

pub fn get_test_scheduler() -> Scheduler {
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

    Scheduler::new(allocator, tasks, interval)
        .add_heuristic(heuristics::dependency)
        .add_heuristic(heuristics::volume)
        .add_heuristic(heuristics::deadline)
}

#[test]
fn test_scheduler() {
    let mut scheduler = get_test_scheduler();

    let mut scheduled_tasks = Vec::new();
    loop {
        let next_item = scheduler.next();
        match next_item {
            Some((task_idx, task_interval)) => {
                scheduled_tasks.push((task_idx, task_interval.clone()));
                scheduler.schedule_task(task_idx, task_interval)
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
