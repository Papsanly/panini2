#![cfg(test)]
use crate::{
    allocators::TaskAllocatorWithPlans, heuristics, interval::Interval, scheduler::Scheduler,
    tasks::Task,
};
use jiff::ToSpan;
use std::collections::BTreeMap;

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
        plans: BTreeMap::from([
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
        .add_heuristic(heuristics::priority)
        .add_heuristic(heuristics::deadline)
}

#[test]
fn test_scheduler() {
    let mut scheduler = get_test_scheduler();

    while let Some((task_idx, task_interval)) = scheduler.next() {
        scheduler.schedule_task(task_idx, task_interval);
    }

    let mut all_intervals = Vec::new();
    for (task_idx, intervals) in scheduler.iter().enumerate() {
        for interval in intervals {
            all_intervals.push((task_idx, interval.clone()));
        }
    }
    all_intervals.sort_by_key(|(_, interval)| interval.start);

    assert_eq!(
        all_intervals,
        vec![
            (
                2,
                Interval::from_span("2025-03-05T09:00Z".parse().unwrap(), 2.hour())
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
