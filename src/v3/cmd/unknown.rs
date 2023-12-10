use std::fmt;

/// Represents an *unknown* command. This is not a real SeedLink `v3` command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unknown {
    command_name: String,
}

impl Unknown {
    /// Creates a new `Unknown` command.
    pub(crate) fn new(key: impl ToString) -> Unknown {
        Unknown {
            command_name: key.to_string(),
        }
    }
}

impl fmt::Display for Unknown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.command_name)
    }
}
