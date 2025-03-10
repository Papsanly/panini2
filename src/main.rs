mod allocators;
mod group_by;
mod heuristics;
mod interval;
mod schedule;
mod tasks;
mod tests;

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
