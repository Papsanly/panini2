mod allocators;
mod chrono;
mod group_by;
mod heuristics;
mod interval;
mod scheduler;
mod tasks;
mod tests;

use crate::scheduler::{Schedule, Scheduler, SchedulerConfig};
use std::{error::Error, fs};

const CONFIG_FILE: &str = "data/config.yaml";
const SCHEDULE_FILE: &str = "data/schedule.yaml";

fn run() -> Result<(), Box<dyn Error>> {
    let config = serde_yaml::from_str::<SchedulerConfig>(&fs::read_to_string(CONFIG_FILE)?)?;

    let mut scheduler = Scheduler::try_from(config)?
        .add_heuristic(heuristics::dependency)
        .add_heuristic(heuristics::volume)
        .add_heuristic(heuristics::deadline)
        .add_heuristic(heuristics::priority)
        .add_heuristic(heuristics::locality);

    scheduler.schedule();
    let schedule = Schedule::from(scheduler);

    fs::write(SCHEDULE_FILE, serde_yaml::to_string(&schedule)?)?;

    Ok(())
}

fn main() {
    run().expect("Failed to run the scheduler");
}
