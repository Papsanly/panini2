use crate::{schedule::TaskIdx, Schedule};
use jiff::Timestamp;

pub type Heuristic = fn(&Schedule, Timestamp, TaskIdx) -> f32;

// if the task is not dependent on any other task or other tasks are past the deadline,
// it will be 1.0, 0.0 otherwise
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

// proportional to priority of the task. e.g. priority 2.0 means that task heuristic score will be multiplied by 2.0
pub fn priority(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    todo!()
}

// inversely proportional to the amount of hours I can work on the task until the deadline
pub fn deadline(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    todo!()
}

// proportional to volume units which are hours of work needed to finish the task
pub fn volume(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    todo!()
}
