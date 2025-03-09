use derive_more::Into;
use jiff::Timestamp;
use std::{error::Error, str::FromStr};

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

impl FromStr for Tasks {
    type Err = Box<dyn Error>;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}
