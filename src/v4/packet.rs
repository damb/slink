use std::convert;
use std::fmt;
use std::io;
use std::str::{self, FromStr};

use mseed::{MSControlFlags, MSRecord};

use crate::{SeedLinkError, SeedLinkResult};

/// SeedLink `v4` packet data formats.
///
/// Including both the data format code and the subformat code.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DataFormat {
    MiniSeed2xDataGeneric,
    MiniSeed2xEventDetection,
    MiniSeed2xCalibration,
    MiniSeed2xTimingException,
    MiniSeed2xLog,
    MiniSeed2xOpaque,
    MiniSeed3xDataGeneric,
    JsonSeedLinkInfo,
    JsonSeedLinkError,
    Xml,
}

impl DataFormat {
    /// Returns the ASCII character representation.
    pub const fn code(&self) -> &'static str {
        match *self {
            Self::MiniSeed2xDataGeneric => "2D",
            Self::MiniSeed2xEventDetection => "2E",
            Self::MiniSeed2xCalibration => "2C",
            Self::MiniSeed2xTimingException => "2T",
            Self::MiniSeed2xLog => "2L",
            Self::MiniSeed2xOpaque => "2O",
            Self::MiniSeed3xDataGeneric => "3D",
            Self::JsonSeedLinkInfo => "JI",
            Self::JsonSeedLinkError => "JE",
            Self::Xml => "X ",
        }
    }

    /// Returns the data format code ASCII character representation.
    pub fn format_code(&self) -> char {
        self.code().chars().next().unwrap()
    }

    /// Returns the subformat code ASCII character representation.
    pub fn subformat_code(&self) -> char {
        self.code().chars().rev().next().unwrap()
    }

    /// Returns the encoded ASCII character representation.
    pub fn code_to_u8(&self) -> [u8; 2] {
        let mut chars = self.code().chars();
        [chars.next().unwrap() as _, chars.next().unwrap() as _]
    }
}

impl str::FromStr for DataFormat {
    type Err = SeedLinkError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "2D" => Self::MiniSeed2xDataGeneric,
            "2E" => Self::MiniSeed2xEventDetection,
            "2C" => Self::MiniSeed2xCalibration,
            "2T" => Self::MiniSeed2xTimingException,
            "2L" => Self::MiniSeed2xLog,
            "2O" => Self::MiniSeed2xOpaque,
            "3D" => Self::MiniSeed3xDataGeneric,
            "JI" => Self::JsonSeedLinkInfo,
            "JE" => Self::JsonSeedLinkError,
            "X " => Self::Xml,
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid data format code: {}", other),
                )
                .into());
            }
        })
    }
}

impl convert::TryFrom<[u8; 2]> for DataFormat {
    type Error = SeedLinkError;

    fn try_from(value: [u8; 2]) -> Result<Self, Self::Error> {
        let mut s = char::from(value[0]).to_string();
        s.push(char::from(value[1]));
        Self::from_str(s.as_str())
    }
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.code(), f)
    }
}

/// SeedLink `v4` packet.
#[derive(Debug, Clone)]
pub struct SeedLinkPacket {
    packet: Vec<u8>,

    format: DataFormat,
    len_payload: u32,
    seq_num: u64,
    len_sta_id: u8,
    sta_id: Option<String>,
}

impl SeedLinkPacket {
    /// Creates a new SeedLink packet.
    pub fn parse(buf: &[u8]) -> SeedLinkResult<Self> {
        // XXX(damb): packet headers are big endian encoded where required
        let signature = buf[..2].to_vec();
        let signature = String::from_utf8(signature).map_err(|e| {
            SeedLinkError::from(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
        })?;
        if signature != "SE" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid packet signature: {}", signature),
            )
            .into());
        }
        let format: [u8; 2] = buf[2..4].try_into().unwrap();
        let format = DataFormat::try_from(format)?;
        let len_payload = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        if len_payload == 0 {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "missing packet payload").into(),
            );
        }
        let seq_num = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let len_sta_id = buf[16];
        let sta_id = if len_sta_id == 0 {
            None
        } else {
            let sta_id: Vec<u8> = buf[17..17 + len_sta_id as usize].to_vec();
            Some(String::from_utf8(sta_id).map_err(|e| {
                SeedLinkError::from(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
            })?)
        };

        Ok(Self {
            packet: buf.to_vec(),
            format,
            len_payload,
            seq_num,
            len_sta_id,
            sta_id,
        })
    }

    /// Returns the packet data format.
    pub fn format(&self) -> &DataFormat {
        &self.format
    }

    /// Returns the packet data format code.
    pub fn format_code(&self) -> char {
        self.format().format_code()
    }

    /// Returns the packet subformat code.
    pub fn subformat_code(&self) -> char {
        self.format().subformat_code()
    }

    /// Returns the packet payload length in bytes.
    pub fn len_payload(&self) -> u32 {
        self.len_payload
    }

    /// Returns the packet sequence number.
    pub fn sequence_number(&self) -> u64 {
        self.seq_num
    }

    /// Returns the packet station identifier length in bytes.
    pub fn len_sta_id(&self) -> u8 {
        self.len_sta_id
    }

    /// Returns the raw packet station identifier.
    pub fn sta_id_raw(&self) -> &[u8] {
        &self.packet[17..17 + self.len_sta_id() as usize]
    }

    /// Returns the packet station identifier.
    pub fn sta_id(&self) -> &Option<String> {
        &self.sta_id
    }

    /// Returns the raw packet bytes.
    pub fn raw(&self) -> &[u8] {
        &self.packet
    }

    /// Returns the raw packet payload.
    pub fn payload_raw(&self) -> &[u8] {
        &self.packet[17 + self.len_sta_id() as usize..]
    }

    /// Returns the packet payload decoded as miniSEED record.
    pub fn payload_to_ms_record(&self) -> SeedLinkResult<MSRecord> {
        Ok(
            MSRecord::parse(self.payload_raw(), MSControlFlags::empty()).map_err(|e| {
                SeedLinkError::from(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to decode miniSEED record: {}", e),
                ))
            })?,
        )
    }

    /// Returns the packet payload decoded as [`String`].
    pub fn payload_to_string(&self) -> SeedLinkResult<String> {
        Ok(String::from_utf8_lossy(self.payload_raw()).to_string())
    }
}

/// Convenience function for packing a SeedLink packet.
pub fn pack_packet(packet: &SeedLinkPacket) -> SeedLinkResult<Vec<u8>> {
    Ok(packet.raw().to_vec())
}

/// Convenience function for packing a SeedLink packet.
pub fn pack_packet_with_seq_num(packet: &SeedLinkPacket, seq_num: u64) -> SeedLinkResult<Vec<u8>> {
    let seq_num_be = seq_num.to_le_bytes();

    let mut packet = packet.raw().to_vec();
    packet.splice(8..8 + seq_num_be.len(), seq_num_be);

    Ok(packet)
}

/// Packs a miniSEED record into a SeedLink `v4` packet.
pub fn pack_ms_record(rec: &MSRecord, seq_num: u64) -> SeedLinkResult<Vec<u8>> {
    let net = rec.network().map_err(|_| {
        SeedLinkError::from(io::Error::new(
            io::ErrorKind::InvalidData,
            "failed to decode network code",
        ))
    })?;
    let sta = rec.station().map_err(|_| {
        SeedLinkError::from(io::Error::new(
            io::ErrorKind::InvalidData,
            "failed to decode station code",
        ))
    })?;

    let net_sta = format!("{}_{}", net, sta);
    let len_sta_id: u8 = net_sta.len().try_into().map_err(|_| {
        SeedLinkError::from(io::Error::new(
            io::ErrorKind::InvalidData,
            "station identifier too large",
        ))
    })?;

    let mut net_sta_bytes = Vec::new();
    for ch in net_sta.chars() {
        if !ch.is_ascii() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "station identifier contains non-ASCII characters",
            )
            .into());
        }
        net_sta_bytes.push(ch as _);
    }

    let payload = rec.raw().ok_or_else(|| {
        SeedLinkError::from(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing payload",
        ))
    })?;
    let len_payload: u32 = payload.len().try_into().map_err(|_| {
        SeedLinkError::from(io::Error::new(
            io::ErrorKind::InvalidData,
            "payload too large",
        ))
    })?;

    let mut packet = Vec::with_capacity(128);
    packet.extend(b"SE");
    // TODO(damb): how to correctly determine subformat code?
    match rec.format_version() {
        2 => {
            let format = DataFormat::MiniSeed2xDataGeneric;
            packet.extend(format.code_to_u8());
        }
        3 => {
            let format = DataFormat::MiniSeed3xDataGeneric;
            packet.extend(format.code_to_u8());
        }
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown data format: {}", other),
            )
            .into());
        }
    };

    packet.extend(len_payload.to_le_bytes());
    packet.extend(seq_num.to_le_bytes());
    packet.push(len_sta_id);
    packet.append(&mut net_sta_bytes);
    packet.extend(payload);

    Ok(packet)
}


/// Packs a JSON string into a SeedLink `v4` info packet.
pub fn pack_info_ok(s: &str) -> SeedLinkResult<Vec<u8>> {
    pack_info(s, DataFormat::JsonSeedLinkInfo)
}

/// Packs a JSON string into a SeedLink `v4` info error packet.
pub fn pack_info_err(s: &str) -> SeedLinkResult<Vec<u8>> {
    pack_info(s, DataFormat::JsonSeedLinkError)
}

fn pack_info(s: &str, format: DataFormat) -> SeedLinkResult<Vec<u8>> {
    if s.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty string").into());
    }

    let mut packet = Vec::new();
    packet.extend(b"SE");

    packet.extend(format.code_to_u8());

    let payload = s.as_bytes();
    let len_payload: u32 = payload.len().try_into().map_err(|_| {
        SeedLinkError::from(io::Error::new(
            io::ErrorKind::InvalidData,
            "payload too large",
        ))
    })?;
    packet.extend(len_payload.to_le_bytes());

    let seq_num: u64 = 0;
    packet.extend(seq_num.to_le_bytes());

    // station identifier length
    packet.push(0);

    packet.extend(payload);

    Ok(packet)
}


