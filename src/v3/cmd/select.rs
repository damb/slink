use std::fmt;

/// Command to select streams for a given station.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Select {
    pattern: Option<String>,
}

impl Select {
    pub const NAME: &'static str = "select";

    pub fn new(pattern: Option<String>) -> Self {
        Self { pattern }
    }
}

impl fmt::Display for Select {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut pattern_str = String::new();
        if let Some(pattern) = &self.pattern {
            pattern_str = format!(" {}", pattern);
        }
        write!(f, "{}{}", Select::NAME, pattern_str)
    }
}
