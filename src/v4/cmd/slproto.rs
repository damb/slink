use std::fmt;
use std::str;

use crate::ProtocolErrorV4;

/// Command to request the protocol version.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SlProto {
    pub major: u8,
    pub minor: u8,
}

impl SlProto {
    pub const NAME: &'static str = "slproto";

    /// Returns the version string.
    pub fn version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

impl str::FromStr for SlProto {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<SlProto, Self::Err> {
        let split = s.split_once(".");
        if let Some((major, minor)) = s.split_once(".") {
            let major: u8 = major
                .parse()
                .map_err(|_| ProtocolErrorV4::incorrect_arguments())?;
            let minor: u8 = minor
                .parse()
                .map_err(|_| ProtocolErrorV4::incorrect_arguments())?;

            return Ok(Self { major, minor });
        }

        Err(ProtocolErrorV4::incorrect_arguments())
    }
}

impl fmt::Display for SlProto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", SlProto::NAME)
    }
}
