use std::hash::{Hash, Hasher};
use std::ops::Deref;

use time::PrimitiveDateTime;

#[derive(Debug, Clone)]
pub(crate) struct StreamConfig {
    pub network: String,
    pub station: String,
    select_args: Vec<String>,
    pub seq_num: Option<String>,
    pub time: Option<PrimitiveDateTime>,
}

impl StreamConfig {
    pub fn new(
        network: &str,
        station: &str,
        selector_arg: Option<String>,
        seq_num: Option<String>,
        time: Option<PrimitiveDateTime>,
    ) -> Self {
        let mut select_args = vec![];
        if let Some(select_arg) = selector_arg {
            select_args.push(select_arg);
        }
        Self {
            network: network.to_string(),
            station: station.to_string(),
            select_args,
            seq_num,
            time,
        }
    }

    /// Adds a `SELECT` command argument to the stream configuration.
    pub fn add_select_arg(&mut self, select_arg: &str) {
        self.select_args.push(select_arg.to_string());
    }

    /// Clears selectors for a given station.
    pub fn clear_select_args(&mut self) {
        self.select_args.clear();
    }
}

impl Deref for StreamConfig {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.select_args
    }
}

impl Hash for StreamConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.network.hash(state);
        self.station.hash(state);
    }
}

