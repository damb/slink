use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use time::OffsetDateTime;

use crate::{
    StationIdV4, StationV3, StationV4, InventoryV3, StreamFormatV4, StreamIdV4, StreamSubFormatV4,
    StreamTypeV3, StreamV3, StreamV4,
};

const SID_DELIMITER: char = '_';

/// Station identifier.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StationId {
    /// Network code
    net_code: String,
    /// Station code
    sta_code: String,
}

impl StationId {
    /// Returns the network code
    pub fn net_code(&self) -> &str {
        &self.net_code
    }

    /// Returns the station code
    pub fn sta_code(&self) -> &str {
        &self.sta_code
    }
}

impl From<StationIdV4> for StationId {
    fn from(item: StationIdV4) -> Self {
        Self {
            net_code: item.net_code().to_string(),
            sta_code: item.sta_code().to_string(),
        }
    }
}

impl fmt::Display for StationId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.net_code, SID_DELIMITER, self.sta_code)
    }
}

/// Structure representing a station in the inventory.
#[derive(Debug, Clone)]
pub struct Station {
    /// Station identifier
    id: StationId,
    /// Station description
    description: String,
    /// First packet sequence number
    start_seq: u64,
    /// Packet sequence number of the most recent packet
    end_seq: u64,

    /// Streams
    streams: Vec<Stream>,
}

impl Station {
    /// Returns the station identifier.
    pub fn id(&self) -> &StationId {
        &self.id
    }

    /// Returns the network code.
    pub fn net_code(&self) -> &str {
        &self.id.net_code
    }

    /// Returns the station code.
    pub fn sta_code(&self) -> &str {
        &self.id.sta_code
    }

    /// Returns the station description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the first sequence number.
    pub fn start_seq(&self) -> u64 {
        self.start_seq
    }

    /// Returns the sequence number of the most recent packet.
    pub fn end_seq(&self) -> u64 {
        self.end_seq
    }

    /// Returns the stream identified by the `location` and `channel` identifiers.
    pub fn get(&self, stream_id: &StreamId) -> Option<&Stream> {
        match self.streams.iter().position(|s| s.id == *stream_id) {
            Some(idx) => Some(&self.streams[idx]),
            None => None,
        }
    }
}

impl From<StationV3> for Station {
    fn from(item: StationV3) -> Self {
        Self {
            id: StationId {
                net_code: item.network,
                sta_code: item.code,
            },
            description: item.description,
            start_seq: item.begin_seq as u64,
            end_seq: item.end_seq as u64,
            streams: match item.stream {
                Some(s) => s.into_iter().map(|s| Stream::from(s)).collect(),
                None => vec![],
            },
        }
    }
}

impl From<StationV4> for Station {
    fn from(item: StationV4) -> Self {
        let streams: Vec<Stream> = if let Some(ref streams) = item.streams() {
            streams.iter().map(|s| s.clone().into()).collect()
        } else {
            vec![]
        };

        Self {
            id: item.id().clone().into(),
            description: item.description().to_string(),
            start_seq: item.start_seq(),
            end_seq: item.end_seq(),
            streams,
        }
    }
}

impl PartialEq for Station {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Station {}

impl Hash for Station {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Deref for Station {
    type Target = Vec<Stream>;

    fn deref(&self) -> &Self::Target {
        &self.streams
    }
}

/// Enumeration of format codes.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Format {
    /// miniSEED 2.x
    MiniSeed2,
    /// miniSEED 3.x with FDSN source identifier
    MiniSeed3,
}

impl From<StreamFormatV4> for Format {
    fn from(item: StreamFormatV4) -> Self {
        match item {
            StreamFormatV4::MiniSeed2 => Self::MiniSeed2,
            StreamFormatV4::MiniSeed3 => Self::MiniSeed3,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::MiniSeed2 => "2",
            Self::MiniSeed3 => "3",
        };

        write!(f, "{}", s)
    }
}

/// Enumeration of subformat codes.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SubFormat {
    /// Data/generic
    Data,
    /// Event detection
    Event,
    /// Calibration
    Calibration,
    /// Opaque
    Opaque,
    /// Timing exception
    Timing,
    /// Log
    Log,
}

impl From<StreamTypeV3> for SubFormat {
    fn from(item: StreamTypeV3) -> Self {
        match item {
            StreamTypeV3::Data => Self::Data,
            StreamTypeV3::Event => Self::Event,
            StreamTypeV3::Calibration => Self::Calibration,
            StreamTypeV3::Blockette => Self::Opaque,
            StreamTypeV3::Timing => Self::Timing,
            StreamTypeV3::Log => Self::Log,
        }
    }
}

impl From<StreamSubFormatV4> for SubFormat {
    fn from(item: StreamSubFormatV4) -> Self {
        match item {
            StreamSubFormatV4::Data => Self::Data,
            StreamSubFormatV4::Event => Self::Event,
            StreamSubFormatV4::Calibration => Self::Calibration,
            StreamSubFormatV4::Opaque => Self::Opaque,
            StreamSubFormatV4::Timing => Self::Timing,
            StreamSubFormatV4::Log => Self::Log,
        }
    }
}

impl fmt::Display for SubFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Data => "D",
            Self::Event => "E",
            Self::Calibration => "C",
            Self::Opaque => "O",
            Self::Timing => "T",
            Self::Log => "L",
        };

        write!(f, "{}", s)
    }
}

/// Stream identifier.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StreamId {
    /// Location code
    loc_code: String,
    /// Band code
    band_code: String,
    /// Source code
    source_code: String,
    /// Subsource code
    subsource_code: String,
}

impl StreamId {
    /// Returns the location code.
    pub fn loc_code(&self) -> &str {
        &self.loc_code
    }

    /// Returns the band code.
    pub fn band_code(&self) -> &str {
        &self.band_code
    }

    /// Returns the source code.
    pub fn source_code(&self) -> &str {
        &self.source_code
    }

    /// Returns the subsource code.
    pub fn subsource_code(&self) -> &str {
        &self.subsource_code
    }
}

impl From<StreamIdV4> for StreamId {
    fn from(item: StreamIdV4) -> Self {
        Self {
            loc_code: item.loc_code().to_string(),
            band_code: item.band_code().to_string(),
            source_code: item.source_code().to_string(),
            subsource_code: item.subsource_code().to_string(),
        }
    }
}

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}{}",
            self.loc_code,
            SID_DELIMITER,
            self.band_code,
            SID_DELIMITER,
            self.source_code,
            SID_DELIMITER,
            self.subsource_code
        )
    }
}

/// Structure representing a stream in the inventory.
#[derive(Debug, Clone)]
pub struct Stream {
    id: StreamId,
    /// Format.
    format: Format,
    /// Subformat.
    subformat: SubFormat,

    /// Time of the first buffered packet.
    start_time: OffsetDateTime,
    /// Time of the last buffered packet.
    end_time: OffsetDateTime,
}

impl Stream {
    /// Returns the stream identifier.
    pub fn id(&self) -> &StreamId {
        &self.id
    }

    /// Returns the location code.
    pub fn loc_code(&self) -> &str {
        &self.id.loc_code
    }

    /// Returns the band code.
    pub fn band_code(&self) -> &str {
        &self.id.band_code
    }

    /// Returns the source code.
    pub fn source_code(&self) -> &str {
        &self.id.source_code
    }

    /// Returns the subsource code.
    pub fn subsource_code(&self) -> &str {
        &self.id.subsource_code
    }

    /// Returns the format.
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// Returns the subformat.
    pub fn subformat(&self) -> &SubFormat {
        &self.subformat
    }

    /// Returns the time of the first buffered packet.
    pub fn start_time(&self) -> &OffsetDateTime {
        &self.start_time
    }

    /// Returns the time of the most recent buffered packet.
    pub fn end_time(&self) -> &OffsetDateTime {
        &self.end_time
    }
}

impl From<StreamV3> for Stream {
    fn from(item: StreamV3) -> Self {
        let mut it = item.channel.chars();
        let band_code = it.next().unwrap().to_string();
        let source_code = it.next().unwrap().to_string();
        let subsource_code = it.next().unwrap().to_string();

        Self {
            id: StreamId {
                loc_code: item.location,
                band_code,
                source_code,
                subsource_code,
            },
            format: Format::MiniSeed2,
            subformat: item.stream_type.into(),
            start_time: item.begin_time,
            end_time: item.end_time,
        }
    }
}
impl From<StreamV4> for Stream {
    fn from(item: StreamV4) -> Self {
        Self {
            id: item.id().clone().into(),
            format: (*item.format()).into(),
            subformat: (*item.subformat()).into(),
            start_time: (*item.start_time()).into(),
            end_time: (*item.end_time()).into(),
        }
    }
}

impl PartialEq for Stream {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Stream {}

impl Hash for Stream {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Struct representing the SeedLink server's stream information available.
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    stations: Vec<Station>,
    stations_idx: HashMap<StationId, usize>,
}

impl Inventory {
    /// Returns the number of stations in the inventory.
    pub fn len(&self) -> usize {
        self.stations.len()
    }

    /// Returns a reference to the station in the inventory.
    pub fn get(&self, station_id: &StationId) -> Option<&Station> {
        match self.stations_idx.get(&station_id) {
            Some(idx) => Some(&self.stations[*idx]),
            None => None,
        }
    }

    /// Adds a new station to the inventory.
    pub fn insert(&mut self, station: Station) -> Option<Station> {
        if let Some(idx) = self.stations_idx.get(&station.id) {
            let sta = self.stations[*idx].clone();
            self.stations[*idx] = station;
            Some(sta)
        } else {
            let station_id = station.id.clone();
            self.stations.push(station);
            self.stations_idx
                .insert(station_id, self.stations.len() - 1);
            None
        }
    }
}

impl Deref for Inventory {
    type Target = Vec<Station>;

    fn deref(&self) -> &Self::Target {
        &self.stations
    }
}

impl From<&Vec<StationV3>> for Inventory {
    fn from(item: &Vec<StationV3>) -> Self {
        let stas: Vec<Station> = item.iter().map(|s| s.clone().into()).collect();
        let idx: HashMap<StationId, usize> = stas
            .iter()
            .enumerate()
            .map(|(idx, s)| (s.id.clone(), idx))
            .collect();
        Self {
            stations: stas,
            stations_idx: idx,
        }
    }
}

impl From<&Vec<StationV4>> for Inventory {
    fn from(item: &Vec<StationV4>) -> Self {
        let stas: Vec<Station> = item.iter().map(|s| s.clone().into()).collect();
        let idx: HashMap<StationId, usize> = stas
            .iter()
            .enumerate()
            .map(|(idx, s)| (s.id.clone(), idx))
            .collect();
        Self {
            stations: stas,
            stations_idx: idx,
        }
    }
}

impl From<InventoryV3> for Inventory {
    fn from(item: InventoryV3) -> Self {
        let stas: Vec<Station> = item.station.into_iter().map(|s| s.clone().into()).collect();
        let idx: HashMap<StationId, usize> = stas
            .iter()
            .enumerate()
            .map(|(idx, s)| (s.id.clone(), idx))
            .collect();
        Self {
            stations: stas,
            stations_idx: idx,
        }
    }
}


