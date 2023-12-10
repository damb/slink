use std::fmt;

// TODO(damb): use OffsetDataTime
use time::PrimitiveDateTime;

use super::super::util;

/// Action command to enable *real-time* mode for a given station.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Data {
    seq_num: Option<i32>,
    begin: Option<PrimitiveDateTime>,
}

impl Data {
    pub const NAME: &'static str = "data";

    pub fn new(seq_num: Option<i32>, begin: Option<PrimitiveDateTime>) -> Self {
        Self { seq_num, begin }
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seq_num_time_str = String::new();
        if let Some(seq_num) = &self.seq_num {
            seq_num_time_str = format!(" {:x}", seq_num);
            if let Some(begin) = &self.begin {
                seq_num_time_str.push_str(&format!(" {}", util::time_as_seedlink_str(begin)));
            }
        }

        write!(f, "{}{}", Data::NAME, seq_num_time_str)
    }
}

