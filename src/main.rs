mod allocators;
mod heuristics;
mod interval;
mod schedule;

use crate::{
    allocators::TaskAllocator,
    interval::Interval,
    schedule::{scheduler_iter, Schedule, Task},
};
use jiff::ToSpan;

fn main() {
    let (mut schedule, interval) = get_test_schedule();
    let scheduled_tasks: Vec<_> = scheduler_iter(&schedule, interval).collect();
    for (task_idx, task_interval) in scheduled_tasks {
        schedule.entry(task_idx).or_default().push(task_interval);
    }
    println!("{schedule}");
}

fn get_test_schedule() -> (Schedule, Interval) {
    let tasks = vec![
        Task {
            description: "Task 0".to_string(),
            deadline: "2025-03-05T12:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 1.0,
            dependencies: vec![],
        },
        Task {
            description: "Task 1".to_string(),
            deadline: "2025-03-05T17:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 3.0,
            dependencies: vec![0],
        },
        Task {
            description: "Task 2".to_string(),
            deadline: "2025-03-05T13:00Z".parse().unwrap(),
            priority: 2.0,
            volume: 0.5,
            dependencies: vec![],
        },
        Task {
            description: "Task 3".to_string(),
            deadline: "2025-03-05T18:00Z".parse().unwrap(),
            priority: 1.0,
            volume: 1.5,
            dependencies: vec![2],
        },
    ];

    let allocator = TaskAllocator {
        idle_intervals: vec![
            Interval::new("2025-03-05T00:00Z".parse().unwrap(), 9.hours()),
            Interval::new("2025-03-05T13:00Z".parse().unwrap(), 2.hours()),
            Interval::new("2025-03-05T22:00Z".parse().unwrap(), 2.hours()),
        ],
        granularity: 1.hour(),
    };

    let schedule = Schedule::new(allocator, tasks)
        .add_heuristic(heuristics::dependency)
        .add_heuristic(heuristics::volume)
        .add_heuristic(heuristics::deadline)
        .add_heuristic(heuristics::priority);

    let interval = Interval::new("2025-03-05T00:00Z".parse().unwrap(), 24.hours());

    (schedule, interval)
}
