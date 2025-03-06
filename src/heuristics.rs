use crate::{schedule::TaskIdx, Schedule};
use jiff::Timestamp;

pub type Heuristic = fn(&Schedule, Timestamp, TaskIdx) -> f32;

pub fn dependency(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &schedule.get_task(task_idx);
    let condition = task
        .dependencies
        .iter()
        .any(|dependency| current_time < schedule.get_task(*dependency).deadline);
    if condition {
        1.0
    } else {
        0.0
    }
}

pub fn priority(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    todo!()
}

pub fn deadline(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    todo!()
}

pub fn volume(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    todo!()
}
