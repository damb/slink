use std::fmt;

/// Action command to end handshaking in multi-station mode.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct End;

impl End {
    pub const NAME: &'static str = "end";
}

impl fmt::Display for End {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", End::NAME)
    }
}
