use std::io;

pub use crate::client::Client;
pub use crate::connection::{
    parse_slink_url, Connection, ConnectionInfo, DataTransferMode, IntoConnectionInfo,
    SeedLinkConnectionInfo,
};
pub use crate::frame::Frame;
pub use crate::inventory::{Format, Inventory, Station, StationId, Stream, StreamId, SubFormat};
pub use crate::packet::SeedLinkPacket;
pub use crate::state::StateDB;
pub use crate::util::{FDSNSourceId, NSLC};
pub use crate::v3::{
    BatchCmdV3, ByeCmdV3, CommandV3, DataCmdV3, EndCmdV3, FetchCmdV3, HelloCmdV3, InfoCmdItemV3,
    InfoCmdV3, InventoryV3, ProtocolErrorV3, SeedLinkGenericDataPacketV3, SeedLinkInfoPacketV3,
    SeedLinkPacketV3, SelectCmdV3, StationCmdV3, StationV3, StreamTypeV3, StreamV3, TimeCmdV3,
    UnknownCmdV3, SEEDLINK_PACKET_HEADER_SIZE_V3, SEEDLINK_PACKET_RECORD_SIZE_V3,
    SEEDLINK_PACKET_SIZE_V3,
};
pub use crate::v4::{
    pack_info_err_v4, pack_info_ok_v4, pack_ms_record_v4, pack_packet_v4,
    pack_packet_with_seq_num_v4, to_first_hello_resp_line_v4, to_id_info_v4, AuthCmdMethodV4,
    AuthCmdV4, AuthV4, ByeCmdV4, CapabilitiesInfoV4, CommandV4, ConnectionsInfoV4, DataCmdV4,
    DataFormatV4, EndCmdV4, EndFetchCmdV4, ErrorCodeV4, ErrorInfoV4, FormatsInfoV4, FrameV4,
    HelloCmdV4, IdInfoV4, InfoCmdItemV4, InfoCmdV4, InfoV4, ProtocolErrorV4, SeedLinkPacketV4,
    SelectCmdPatternV4, SelectCmdV4, SequenceNumberV4, SlProtoCmdV4, StationCmdV4, StationIdV4,
    StationV4, StationsInfoV4, StreamFormatV4, StreamIdV4, StreamOriginV4, StreamSubFormatV4,
    StreamV4, StreamsInfoV4, UnknownCmdV4, UserAgentCmdInfoV4, UserAgentCmdV4,
};

use crate::connection::{connect, ActualConnection, TcpConnection};
use crate::stream_config::StreamConfig;
use crate::v3::{SeedLinkConnectionV3, SeedLinkDataTransferModeV3};

mod client;
mod connection;
mod frame;
mod inventory;
mod packet;
mod state;
mod stream_config;
mod util;
mod v3;
mod v4;

/// Default port that a SeedLink server listens on.
pub const DEFAULT_PORT: u16 = 18000;

/// Available client protocol versions (sorted, non-decreasing) implemented by the library.
pub const AVAILABLE_CLIENT_PROTO_VERSIONS: [u8; 1] = [3];

/// Generic library error type.
#[derive(thiserror::Error, Debug)]
pub enum SeedLinkError {
    #[error("{0}")]
    UnsupportedCommand(String),
    #[error("{0}")]
    UnexpectedCommand(String),
    #[error("{0}")]
    UnauthorizedCommand(String),
    #[error("{0}")]
    InvalidProtocolVersion(String),
    #[error("{0}")]
    InvalidCommandArgument(String),
    #[error("{0}")]
    ClientError(String),
    #[error("{0}")]
    InvalidClientConfig(String),
    #[error("{0}")]
    StateDBError(String),
    #[error("{0}")]
    InvalidStreamId(String),
    #[error(transparent)]
    MSError(#[from] mseed::MSError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// A specialized library [`Result`] type.
///
/// [`Result`]: enum@std::result::Result
pub type SeedLinkResult<T> = std::result::Result<T, SeedLinkError>;
