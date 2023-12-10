use std::fmt;

/// Represents an *unknown* command. This is not a real SeedLink `v4` command.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Unknown {
    pub command_name: String,
}

impl Unknown {
    /// Create a new `Unknown` command.
    pub(crate) fn new(key: impl ToString) -> Self {
        Self {
            command_name: key.to_string(),
        }
    }
}

impl fmt::Display for Unknown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.command_name)
    }
}

