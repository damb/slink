use std::ops::Deref;
use std::{convert, fmt, str};

use crate::ProtocolErrorV4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAgentInfo {
    pub program_or_library: String,
    pub version: String,
}

impl UserAgentInfo {
    pub fn new(program_or_library: String, version: String) -> Self {
        Self {
            program_or_library,
            version,
        }
    }
}

impl str::FromStr for UserAgentInfo {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<UserAgentInfo, Self::Err> {
        let split: Vec<&str> = s.split('/').collect();
        if split.len() != 2 {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        Ok(Self {
            program_or_library: split[0].into(),
            version: split[1].into(),
        })
    }
}

impl fmt::Display for UserAgentInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.program_or_library, self.version)
    }
}

impl convert::From<(String, String)> for UserAgentInfo {
    fn from(value: (String, String)) -> Self {
        Self {
            program_or_library: value.0,
            version: value.1,
        }
    }
}

/// Passes user agent data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserAgent {
    pub info: Vec<UserAgentInfo>,
}

impl UserAgent {
    pub const NAME: &'static str = "useragent";

    pub fn new(info: Vec<UserAgentInfo>) -> Self {
        Self { info }
    }
}

impl str::FromStr for UserAgent {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<UserAgent, Self::Err> {
        let split: Vec<&str> = s.split(' ').collect();
        if split.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        let info: Result<Vec<_>, _> = split
            .into_iter()
            .map(|s| UserAgentInfo::from_str(s))
            .collect();

        Ok(Self { info: info? })
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let info: Vec<String> = self.info.iter().map(|s| s.to_string()).collect();
        write!(f, "{} {}", Self::NAME, info.join(" "))
    }
}

impl Deref for UserAgent {
    type Target = Vec<UserAgentInfo>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}
