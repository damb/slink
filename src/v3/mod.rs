pub use cmd::{
    Batch as BatchCmdV3, Bye as ByeCmdV3, Command as CommandV3, Data as DataCmdV3, End as EndCmdV3,
    Fetch as FetchCmdV3, Hello as HelloCmdV3, Info as InfoCmdV3, InfoItem as InfoCmdItemV3,
    Select as SelectCmdV3, Station as StationCmdV3, Time as TimeCmdV3, Unknown as UnknownCmdV3,
};
pub use error::Error as ProtocolErrorV3;
pub use inventory::{
    Inventory as InventoryV3, Station as StationV3, Stream as StreamV3, StreamType as StreamTypeV3,
};
pub use packet::{
    SeedLinkGenericDataPacketV3, SeedLinkInfoPacketV3, SeedLinkPacketV3,
    HEADER_SIZE as SEEDLINK_PACKET_HEADER_SIZE_V3, RECORD_SIZE as SEEDLINK_PACKET_RECORD_SIZE_V3,
};

pub(crate) use connection::{
    SeedLinkConnectionV3, SeedLinkDataTransferModeV3, 
};

mod cmd;
mod connection;
mod error;
mod inventory;
mod packet;
mod util;

/// SeedLink v3 packet size
pub const SEEDLINK_PACKET_SIZE_V3: usize =
    SEEDLINK_PACKET_HEADER_SIZE_V3 + SEEDLINK_PACKET_RECORD_SIZE_V3;
