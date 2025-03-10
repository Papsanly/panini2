use derive_more::Into;
use jiff::{civil::Date, tz::TimeZone, Timestamp};
use std::error::Error;

impl TryFrom<String> for Task {
    type Error = Box<dyn Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts: Vec<_> = value.split('/').map(|p| p.trim()).collect();

        let priority = if parts.len() == 5 {
            let res = parts.pop().unwrap();
            if !res.chars().all(|c| c == '!') {
                return Err(format!("Invalid priority: {}", res).into());
            }
            res.len() as f32
        } else {
            1.0
        };

        let [description, deadline, volume, progress]: [&str; 4] =
            parts.try_into().map_err(|e: Vec<_>| {
                format!("Expected at least 4 elements, got {}: {:?}", e.len(), e)
            })?;

        let deadline = deadline
            .parse::<Date>()?
            .to_zoned(TimeZone::system())?
            .timestamp();
        let volume = volume[..volume.len() - 1].parse::<u32>()? as f32;
        let progress = progress[..progress.len() - 1].parse::<u32>()? as f32;

        Ok(Task {
            description: description.to_string(),
            deadline,
            priority,
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

impl TryFrom<Vec<Vec<String>>> for Tasks {
    type Error = Box<dyn Error>;

    fn try_from(value: Vec<Vec<String>>) -> Result<Self, Self::Error> {
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
