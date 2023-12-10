/// Hello response information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hello {
    pub implementation: String,
    pub implementation_version: String,

    pub data_center_description: String,
}

