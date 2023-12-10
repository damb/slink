use std::fmt;
use std::str;

use crate::ProtocolErrorV4;

/// Command to request station data during handshaking.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Station {
    pub station_pattern: String,
}

impl Station {
    pub const NAME: &'static str = "station";
}

impl str::FromStr for Station {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<Station, Self::Err> {
        let t = s.trim();
        if t.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        Ok(Self {
            station_pattern: t.to_string(),
        })
    }
}

impl fmt::Display for Station {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", Self::NAME, self.station_pattern)
    }
}
