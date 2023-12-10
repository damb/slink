use std::fmt;

/// Command to enable *batch mode*.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Batch;

impl Batch {
    pub const NAME: &'static str = "batch";
}

impl fmt::Display for Batch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Batch::NAME)
    }
}
