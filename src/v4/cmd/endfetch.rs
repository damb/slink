use std::fmt;

/// Action command that ends handshaking and switches to data transfer phase in dial-up mode.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct EndFetch;

impl EndFetch {
    pub const NAME: &'static str = "endfetch";
}

impl fmt::Display for EndFetch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", EndFetch::NAME)
    }
}
