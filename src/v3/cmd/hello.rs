use std::fmt;

/// Command to start handshaking.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Hello;

impl Hello {
    pub const NAME: &'static str = "hello";
}

impl fmt::Display for Hello {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Hello::NAME)
    }
}
