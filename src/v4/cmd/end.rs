use std::fmt;

/// Action command that ends handshaking and switches to data transfer phase in real-time mode.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct End;

impl End {
    pub const NAME: &'static str = "end";
}

impl fmt::Display for End {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", End::NAME)
    }
}
