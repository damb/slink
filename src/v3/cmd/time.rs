use std::fmt;

// TODO(damb): use `time::OffsetDataTime`
use time::PrimitiveDateTime;

use super::super::util;

/// Action command to request a time window for a given station.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Time {
    begin: Option<PrimitiveDateTime>,
    end: Option<PrimitiveDateTime>,
}

impl Time {
    pub const NAME: &'static str = "time";

    pub fn new(begin: Option<PrimitiveDateTime>, end: Option<PrimitiveDateTime>) -> Self {
        Self { begin, end }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut time_str = String::new();
        if let Some(begin) = &self.begin {
            time_str = format!(" {}", util::time_as_seedlink_str(begin));
            if let Some(end) = &self.end {
                time_str.push_str(&format!(" {}", util::time_as_seedlink_str(end)));
            }
        }

        write!(f, "{}{}", Time::NAME, time_str)
    }
}
