use crate::SeedLinkPacketV3;

/// Enumeration of SeedLink packets
#[derive(Debug)]
pub enum SeedLinkPacket {
    V3(SeedLinkPacketV3),
    //V4()
}

impl SeedLinkPacket {
    /// Returns whether the packet is a SeedLink info packet.
    pub fn is_info(&self) -> bool {
        match self {
            Self::V3(packet) => packet.is_info(),
        }
    }

    /// Returns whether the packet is a SeedLink data packet.
    pub fn is_data(&self) -> bool {
        match self {
            Self::V3(packet) => packet.is_data(),
        }
    }
}

