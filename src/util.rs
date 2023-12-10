use std::fmt;
use std::io;
use std::str::FromStr;

use crate::{SeedLinkError, SeedLinkResult};

pub struct ParsedHelloResponse {
    pub protocol_versions: Vec<String>,
    pub station_or_datacenter_desc: String,
}

pub fn parse_hello_response(
    first_resp_line: &str,
    second_resp_line: String,
) -> SeedLinkResult<ParsedHelloResponse> {
    let split: Vec<&str> = first_resp_line.splitn(2, " v").collect();
    if split.len() != 2 || split[1].len() < 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "failed to parse SeedLink protocol version",
        )
        .into());
    }

    if let Err(_) = split[1][..3].parse::<f32>() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "failed to parse SeedLink protocol version",
        )
        .into());
    }

    let highest_supported_protocol_version = split[1][..3].to_string();

    // TODO(damb): prepare for SeedLink v4.0 and parse additionally supported protocol versions

    let seedlink_id = split[0].to_lowercase();
    if seedlink_id != "seedlink" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid SeedLink server identifier",
        )
        .into());
    }

    Ok(ParsedHelloResponse {
        protocol_versions: vec![highest_supported_protocol_version],
        station_or_datacenter_desc: second_resp_line,
    })
}

/// Utility structure for network, station, location, and channel code identifiers.
#[derive(Debug, Clone)]
pub struct NSLC {
    pub net: String,
    pub sta: String,
    pub loc: String,
    pub cha: String,
}

impl NSLC {
    pub const SEP: char = '_';

    /// Parses the individual `NSLC` components from `nslc`.
    fn parse(nslc: &str) -> SeedLinkResult<Self> {
        let split: Vec<&str> = nslc.splitn(4, Self::SEP).collect();

        if split.len() != 4 {
            return Err(SeedLinkError::InvalidStreamId(
                "invalid fdsn source identifier".into(),
            ));
        }

        Ok(Self {
            net: split[0].to_string(),
            sta: split[1].to_string(),
            loc: split[2].to_string(),
            cha: split[3].to_string(),
        })
    }
}

impl fmt::Display for NSLC {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}{}",
            self.net,
            Self::SEP,
            self.sta,
            Self::SEP,
            self.loc,
            Self::SEP,
            self.cha
        )
    }
}

/// Represents a FDSN source identifier.
#[derive(Debug, Clone)]
pub struct FDSNSourceId {
    pub ns: String,
    pub nslc: NSLC,
}

impl FDSNSourceId {
    pub const NS_SEP: char = ':';

    /// Parses a `FDSNSourceId` from `sid`.
    fn parse(sid: &str) -> SeedLinkResult<Self> {
        let split: Vec<&str> = sid.split(Self::NS_SEP).collect();
        if split.len() != 2 {
            return Err(SeedLinkError::InvalidStreamId(
                "missing namespace identifier".into(),
            ));
        }

        Ok(Self {
            ns: split[0].to_string(),
            nslc: NSLC::parse(split[1])?,
        })
    }
}

impl fmt::Display for FDSNSourceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.ns, Self::NS_SEP, self.nslc,)
    }
}

impl FromStr for FDSNSourceId {
    type Err = SeedLinkError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Returns the select argument as used in SeedLink v3.
pub fn get_select_arg_v3(sid: &FDSNSourceId) -> String {
    let split: Vec<&str> = sid.nslc.cha.split(NSLC::SEP).collect();
    format!("{}{}{}{}", sid.nslc.loc, split[0], split[1], split[2])
}

/// Returns the select argument as used in SeedLink v4.
pub fn get_select_arg_v4(sid: &FDSNSourceId) -> String {
    format!("{}{}{}", sid.nslc.loc, NSLC::SEP, sid.nslc.cha)
}

