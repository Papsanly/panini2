use derive_more::Into;
use jiff::{civil::Date, tz::TimeZone, Timestamp};
use std::error::Error;

impl TryFrom<Vec<String>> for Task {
    type Error = Box<dyn Error>;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        let [description, deadline, volume, progress] = value
            .try_into()
            .map_err(|e: Vec<_>| format!("Expected 4 elements, got {}: {:?}", e.len(), e))?;

        let deadline = deadline
            .parse::<Date>()?
            .to_zoned(TimeZone::system())?
            .timestamp();
        let volume = volume[..volume.len() - 1].parse::<u32>()? as f32;
        let progress = progress[..progress.len() - 1].parse::<u32>()? as f32;

        Ok(Task {
            description,
            deadline,
            priority: 1.0,
            volume: volume * (1.0 - progress / 100.0),
            dependencies: Vec::new(),
        })
    }
}

#[derive(Debug)]
pub struct Task {
    pub description: String,
    pub deadline: Timestamp,
    pub priority: f32,
    pub volume: f32,
    pub dependencies: Vec<TaskIdx>,
}

pub type TaskIdx = usize;

#[derive(Into)]
pub struct Tasks(Vec<Task>);

impl TryFrom<Vec<Vec<Vec<String>>>> for Tasks {
    type Error = Box<dyn Error>;

    fn try_from(value: Vec<Vec<Vec<String>>>) -> Result<Self, Self::Error> {
        let mut tasks = Vec::new();

        for task_chain in value {
            let mut iter = task_chain.into_iter();
            if let Some(task) = iter.next() {
                tasks.push(task.try_into()?);
            }
            for task in iter {
                let mut task: Task = task.try_into()?;
                task.dependencies = vec![tasks.len() - 1];
                tasks.push(task);
            }
        }

        Ok(Tasks(tasks))
    }
}
