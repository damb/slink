use std::collections::HashMap;

use serde::Serialize;

use crate::ProtocolErrorV4;
use crate::StationV4;

// TODO(damb): implement `Deserialize` for client deserialization

/// SeedLink v4 `INFO` response information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Info {
    Id(IdInfo),
    Formats(FormatsInfo),
    Capabilities(CapabilitiesInfo),
    Stations(StationsInfo),
    Streams(StreamsInfo),
    Connections(ConnectionsInfo),
    Error(ErrorInfo),
}

/// Dictionary of filters supported by the server
type Filters = HashMap<String, String>;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Format {
    /// MIME type of format
    pub mimetype: String,
    // Descriptions of subformats
    pub subformat: HashMap<String, String>,
}

/// Dictionary of formats supported by the server
type Formats = HashMap<String, Format>;

/// SeedLink `v4` `INFO ID` response information.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct IdInfo {
    /// Software ID as in HELLO response
    pub software: String,
    /// Station or data center description as in HELLO response
    pub organization: String,
}

/// SeedLink `v4` `INFO STATIONS` response information.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct StationsInfo {
    #[serde(flatten)]
    pub id: IdInfo,

    /// Dictionary of filters supported by the server
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub filter: Filters,
    /// Dictionary of formats supported by the server
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub format: Formats,

    pub station: Vec<StationV4>,
}

/// SeedLink `v4` `INFO STREAMS` response information.
pub type StreamsInfo = StationsInfo;

/// SeedLink `v4` `INFO FORMATS` response information.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FormatsInfo {
    #[serde(flatten)]
    pub id: IdInfo,

    /// Dictionary of filters supported by the server
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub filter: Filters,
    /// Dictionary of formats supported by the server
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub format: Formats,
}

/// SeedLink `v4` `INFO CAPABILITIES` response information.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CapabilitiesInfo {
    #[serde(flatten)]
    pub id: IdInfo,
    // TODO(damb): not specified, yet. See:
    // https://seedlink.readthedocs.io/en/draft/protocol.html#appendix-b-json-schema
}

/// SeedLink `v4` `INFO CONNECTIONS` response information.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ConnectionsInfo {
    #[serde(flatten)]
    pub id: IdInfo,
    // TODO(damb):
}

/// SeedLink `v4` `INFO` error response information.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ErrorInfo {
    #[serde(flatten)]
    pub id: IdInfo,

    pub error: ProtocolErrorV4,
}
