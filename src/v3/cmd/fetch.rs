use std::fmt;

// TODO(damb): use `time::OffsetDataTime`
use time::PrimitiveDateTime;

use super::super::util;

/// Action command to enable *dial-up* mode for a given station.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Fetch {
    seq_num: Option<i32>,
    begin: Option<PrimitiveDateTime>,
}

impl Fetch {
    pub const NAME: &'static str = "fetch";

    pub fn new(seq_num: Option<i32>, begin: Option<PrimitiveDateTime>) -> Self {
        Self { seq_num, begin }
    }
}

impl fmt::Display for Fetch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seq_num_time_str = String::new();
        if let Some(seq_num) = &self.seq_num {
            seq_num_time_str = format!(" {:x}", seq_num);
            if let Some(begin) = &self.begin {
                seq_num_time_str.push_str(&format!(" {}", util::time_as_seedlink_str(begin)));
            }
        }

        write!(f, "{}{}", Fetch::NAME, seq_num_time_str)
    }
}
