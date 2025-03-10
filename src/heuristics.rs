use crate::{interval::Interval, tasks::TaskIdx, Scheduler};
use jiff::{tz::TimeZone, Timestamp, Unit};

pub type Heuristic = fn(&Scheduler, Timestamp, TaskIdx) -> f32;

// if the task is not dependent on any other task or other tasks are past the deadline,
// it will be 1.0, 0.0 otherwise
pub fn dependency(scheduler: &Scheduler, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &scheduler.tasks[task_idx];
    let condition = task.dependencies.iter().all(|&dependency_idx| {
        let dependency = &scheduler.tasks[dependency_idx];
        dependency.deadline <= current_time
            || dependency.volume - scheduler.get_total_task_hours(dependency_idx) <= f32::EPSILON
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
pub fn deadline(scheduler: &Scheduler, current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &scheduler.tasks[task_idx];
    let total = task.deadline - current_time;
    let total_hours = total
        .total((Unit::Hour, &current_time.to_zoned(TimeZone::system())))
        .expect("Failed to convert total to hours") as f32;

    let planned_hours =
        scheduler.get_planned_hours(Interval::new(current_time, current_time + total));

    let working_hours = total_hours - planned_hours;
    if working_hours <= 0.0 {
        return 0.0;
    }

    1.0 / working_hours
}

// proportional to volume units which are hours of work needed to finish the task
pub fn volume(scheduler: &Scheduler, _current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let task = &scheduler.tasks[task_idx];
    task.volume - scheduler.get_total_task_hours(task_idx)
}

pub fn locality(scheduler: &Scheduler, _current_time: Timestamp, task_idx: TaskIdx) -> f32 {
    let Some(previous_task) = scheduler.get_last_task() else {
        return 1.0;
    };
    if previous_task == task_idx {
        2.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interval::Interval, tests::get_test_scheduler};
    use jiff::ToSpan;

    #[test]
    fn test_dependency_heuristic() {
        let mut scheduler = get_test_scheduler();
        let task_idx = 0;
        let current_time = scheduler.interval.start;

        let score = dependency(&scheduler, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 1;
        let score = dependency(&scheduler, current_time, task_idx);
        assert_eq!(score, 0.0);

        let current_time = scheduler.interval.start + 12.hours();
        let score = dependency(&scheduler, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 2;
        let score = dependency(&scheduler, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 0;
        let current_time = scheduler.interval.start + 9.hours();
        scheduler.schedule_task(
            task_idx,
            Interval::new(current_time, current_time + 2.hours()),
        );
        let task_idx = 1;
        let current_time = scheduler.interval.start + 11.hours();
        let score = dependency(&scheduler, current_time, task_idx);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_priority_heuristic() {
        let schedule = get_test_schedule();
        let task_idx = 0;
        let current_time = schedule.interval.start;

        let score = priority(&schedule, current_time, task_idx);
        assert_eq!(score, 1.0);

        let task_idx = 2;
        let score = priority(&schedule, current_time, task_idx);
        assert_eq!(score, 2.0);
    }

    #[test]
    fn test_deadline_heuristic() {
        let scheduler = get_test_scheduler();
        let task_idx = 0;
        let current_time = scheduler.interval.start;

        let score = deadline(&scheduler, current_time, task_idx);
        assert_eq!(score, 1.0 / 3.0);

        let task_idx = 3;
        let score = deadline(&scheduler, current_time, task_idx);
        assert_eq!(score, 1.0 / 7.0);

        let task_idx = 2;
        let current_time = scheduler.tasks[task_idx].deadline + 1.hour();
        let score = deadline(&scheduler, current_time, task_idx);
        assert_eq!(score, 0.0);

        let current_time = scheduler.tasks[task_idx].deadline;
        let score = deadline(&scheduler, current_time, task_idx);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_volume_heuristic() {
        let mut scheduler = get_test_scheduler();
        let task_idx = 0;
        let current_time = scheduler.interval.start;

        let score = volume(&scheduler, current_time, task_idx);
        assert_eq!(score, 2.0);

        scheduler.schedule_task(
            task_idx,
            Interval::new(current_time, current_time + 1.hour().minutes(30)),
        );

        let score = volume(&scheduler, current_time, task_idx);
        assert_eq!(score, 0.5);
    }
}
