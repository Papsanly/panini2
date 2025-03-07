use crate::{interval::Interval, schedule::TaskIdx, Schedule};
use jiff::{tz::TimeZone, Timestamp, Unit};

pub type Heuristic = fn(&Schedule, Timestamp, TaskIdx) -> f32;

// if the task is not dependent on any other task or other tasks are past the deadline,
// it will be 1.0, 0.0 otherwise
pub fn dependency(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &schedule.tasks[task_idx];
    let condition = task.dependencies.iter().all(|&dependency_idx| {
        let dependency = &schedule.tasks[dependency_idx];
        dependency.deadline <= current_time
            || dependency.volume - schedule.get_total_task_hours(dependency_idx) <= f32::EPSILON
    });
    if condition {
        1.0
    } else {
        0.0
    }
}

// proportional to priority of the task. e.g. priority 2.0 means that task heuristic score will be multiplied by 2.0
pub fn priority(schedule: &Schedule, _current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &schedule.tasks[task_idx];
    task.priority
}

// inversely proportional to the amount of hours I can work on the task until the deadline
pub fn deadline(schedule: &Schedule, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &schedule.tasks[task_idx];
    let total = task.deadline - current_time;
    let total_hours = total
        .total((Unit::Hour, &current_time.to_zoned(TimeZone::system())))
        .unwrap() as f32;

    if total_hours <= 0.0 {
        return 0.0;
    }

    let idle_hours = schedule.get_idle_hours(Interval::new(current_time, total));

    let working_hours = total_hours - idle_hours;
    if working_hours <= 0.0 {
        return 0.0;
    }

    1.0 / working_hours
}

// proportional to volume units which are hours of work needed to finish the task
pub fn volume(schedule: &Schedule, _current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &schedule.tasks[task_idx];
    task.volume - schedule.get_total_task_hours(task_idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{get_test_schedule, interval::Interval};
    use jiff::ToSpan;

    #[test]
    fn test_dependency_heuristic() {
        let mut schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.timestamp;

        let score = dependency(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 1;
        let score = dependency(&schedule, current_time, task_idx);
        assert_eq!(score, 0.0);

        let current_time = schedule.interval.timestamp + 12.hours();
        let score = dependency(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 2;
        let score = dependency(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 0;
        let current_time = schedule.interval.timestamp + 9.hours();
        schedule
            .entry(task_idx)
            .or_default()
            .push(Interval::new(current_time, 2.hours()));
        let task_idx = 1;
        let current_time = schedule.interval.timestamp + 11.hours();
        let score = dependency(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_priority_heuristic() {
        let schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.timestamp;

        let score = priority(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 2;
        let score = priority(&schedule, current_time, task_idx);
        assert_eq!(score, 2.0);
    }

    #[test]
    fn test_deadline_heuristic() {
        let schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.timestamp;

        let score = deadline(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0 / 3.0);

        let task_idx = 3;
        let score = deadline(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0 / 7.0);

        let task_idx = 2;
        let current_time = schedule.tasks[task_idx].deadline + 1.hour();
        let score = deadline(&schedule, current_time, task_idx);
        assert_eq!(score, 0.0);

        let current_time = schedule.tasks[task_idx].deadline;
        let score = deadline(&schedule, current_time, task_idx);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_volume_heuristic() {
        let mut schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.timestamp;

        let score = volume(&schedule, current_time, task_idx);
        assert_eq!(score, 2.0);

        schedule
            .entry(task_idx)
            .or_default()
            .push(Interval::new(current_time, 1.hour().minutes(30)));

        let score = volume(&schedule, current_time, task_idx);
        assert_eq!(score, 0.5);
    }
}
