use std::fmt;
use std::ops::Deref;
use std::str;

use crate::ProtocolErrorV4;

/// Pattern configuration for `SELECT` command.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SelectPattern {
    pub exclude: bool,

    pub stream_pattern: String,
    pub format_subformat_pattern: Option<String>,
    pub filter: Option<String>,
}

impl str::FromStr for SelectPattern {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<SelectPattern, Self::Err> {
        let split: Vec<&str> = s.split(':').collect();
        if split.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        let filter = if split.len() == 2 {
            if split[1].is_empty() {
                return Err(ProtocolErrorV4::incorrect_arguments());
            }
            Some(split[1].to_string())
        } else {
            None
        };
        let split: Vec<&str> = split[0].split('.').collect();
        if split.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }
        let format_subformat_pattern = if split.len() == 2 {
            if split[1].is_empty() {
                return Err(ProtocolErrorV4::incorrect_arguments());
            }
            Some(split[1].to_string())
        } else {
            None
        };

        let exclude = if split[0].chars().next().unwrap() == '!' {
            true
        } else {
            false
        };

        // XXX: the `:filter` suffix MUST NOT be used together with the `!` prefix.
        if exclude && filter.is_some() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        let stream_pattern = if exclude {
            if split[0].len() < 2 {
                return Err(ProtocolErrorV4::incorrect_arguments());
            }
            split[0][1..].to_string()
        } else {
            split[0].to_string()
        };

        Ok(Self {
            exclude,
            stream_pattern,
            format_subformat_pattern,
            filter,
        })
    }
}

impl fmt::Display for SelectPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = if self.exclude {
            "!".to_string()
        } else {
            String::new()
        };
        s.push_str(&self.stream_pattern);
        if let Some(ref format_subformat_pattern) = self.format_subformat_pattern {
            s.push_str(".");
            s.push_str(format_subformat_pattern);
        }
        if let Some(ref filter) = self.filter {
            s.push_str(":");
            s.push_str(filter);
        }

        write!(f, "{}", s)
    }
}

/// Command to select streams for a given station.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Select {
    pub patterns: Vec<SelectPattern>,
}

impl Select {
    pub const NAME: &'static str = "select";
}

impl str::FromStr for Select {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<Select, Self::Err> {
        let split: Vec<&str> = s.split(' ').collect();
        if split.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        let patterns: Result<Vec<_>, _> = split
            .into_iter()
            .map(|s| SelectPattern::from_str(s))
            .collect();

        Ok(Self {
            patterns: patterns?,
        })
    }
}

impl fmt::Display for Select {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let patterns: Vec<String> = self.patterns.iter().map(|s| s.to_string()).collect();
        write!(f, "{} {}", Self::NAME, patterns.join(" "))
    }
}

impl Deref for Select {
    type Target = Vec<SelectPattern>;

    fn deref(&self) -> &Self::Target {
        &self.patterns
    }
}
