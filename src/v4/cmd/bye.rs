use std::fmt;

/// Command to tell the server to close the connection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Bye;

impl Bye {
    pub const NAME: &'static str = "bye";
}

impl fmt::Display for Bye {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Bye::NAME)
    }
}
