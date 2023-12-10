/// A frame in the SeedLink protocol.
#[derive(Clone, Debug)]
pub enum Frame {
    Line(Vec<u8>),
    InfoPacket(Vec<u8>),
    GenericDataPacket(Vec<u8>),
    Error,
    End,
    Ok,
}

