pub use auth::Auth as AuthV4;
pub use cmd::{
    Auth as AuthCmdV4, AuthMethod as AuthCmdMethodV4, Bye as ByeCmdV4, Command as CommandV4,
    Data as DataCmdV4, End as EndCmdV4, EndFetch as EndFetchCmdV4, Hello as HelloCmdV4,
    Info as InfoCmdV4, InfoItem as InfoCmdItemV4, Select as SelectCmdV4,
    SelectPattern as SelectCmdPatternV4, SequenceNumber as SequenceNumberV4,
    SlProto as SlProtoCmdV4, Station as StationCmdV4, Unknown as UnknownCmdV4,
    UserAgent as UserAgentCmdV4, UserAgentInfo as UserAgentCmdInfoV4,
};
pub use error::{Error as ProtocolErrorV4, ErrorCode as ErrorCodeV4};
pub use info::{
    CapabilitiesInfo as CapabilitiesInfoV4, ConnectionsInfo as ConnectionsInfoV4,
    ErrorInfo as ErrorInfoV4, FormatsInfo as FormatsInfoV4, IdInfo as IdInfoV4, Info as InfoV4,
    StationsInfo as StationsInfoV4, StreamsInfo as StreamsInfoV4,
};
pub use inventory::{
    Station as StationV4, StationId as StationIdV4, Stream as StreamV4,
    StreamFormat as StreamFormatV4, StreamId as StreamIdV4, StreamOrigin as StreamOriginV4,
    StreamSubFormat as StreamSubFormatV4,
};
pub use packet::{
    pack_info_err as pack_info_err_v4, pack_info_ok as pack_info_ok_v4,
    pack_ms_record as pack_ms_record_v4, pack_packet as pack_packet_v4,
    pack_packet_with_seq_num as pack_packet_with_seq_num_v4, DataFormat as DataFormatV4,
    SeedLinkPacket as SeedLinkPacketV4,
};
pub use util::{
    to_first_hello_resp_line as to_first_hello_resp_line_v4, to_id_info as to_id_info_v4,
};

mod auth;
mod cmd;
mod error;
mod info;
mod inventory;
mod packet;
mod util;

/// SeedLink `v4` frame enumeration.
#[derive(Debug, Clone)]
pub enum FrameV4 {
    Lines(Vec<String>),
    Packet(SeedLinkPacketV4),
    Error(ProtocolErrorV4),
    End,
    Ok,
}
