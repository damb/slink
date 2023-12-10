use std::fmt;
use std::str;

use crate::ProtocolErrorV4;

/// Command to request information about the SeedLink server.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Info {
    pub item: InfoItem,

    pub station_pattern: Option<String>,
    pub stream_pattern: Option<String>,
    pub format_subformat_pattern: Option<String>,
}

impl Info {
    pub const NAME: &'static str = "info";

    pub fn new(item: InfoItem) -> Self {
        Self {
            item,
            station_pattern: None,
            stream_pattern: None,
            format_subformat_pattern: None,
        }
    }
}

impl str::FromStr for Info {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<Info, Self::Err> {
        let split: Vec<&str> = s.split(' ').collect();
        if split.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        let item = match split[0].to_lowercase().as_str() {
            "id" => InfoItem::Id,
            "formats" => InfoItem::Formats,
            "stations" => InfoItem::Stations,
            "streams" => InfoItem::Streams,
            "connections" => InfoItem::Connections,
            "capabilities" => InfoItem::Capabilities,
            _ => {
                return Err(ProtocolErrorV4::incorrect_arguments());
            }
        };

        // check whether patterns apply
        if (item == InfoItem::Id || item == InfoItem::Formats || item == InfoItem::Capabilities)
            && split.len() > 1
        {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        // TODO(damb): check whether patterns apply
        // https://seedlink.readthedocs.io/en/latest/protocol.html#commands

        if split.len() == 1 {
            return Ok(Self::new(item));
        }

        let station_pattern = split[1].to_string();
        if station_pattern.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }
        if split.len() == 2 {
            return Ok(Self {
                item,
                station_pattern: Some(station_pattern),
                stream_pattern: None,
                format_subformat_pattern: None,
            });
        }

        let split: Vec<&str> = split[2].split('.').collect();
        if split.len() == 1 {
            if split[0].is_empty() {
                return Err(ProtocolErrorV4::incorrect_arguments());
            }
            return Ok(Self {
                item,
                station_pattern: Some(station_pattern),
                stream_pattern: Some(split[0].to_string()),
                format_subformat_pattern: None,
            });
        }

        if split[1].is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        Ok(Self {
            item,
            station_pattern: Some(station_pattern),
            stream_pattern: Some(split[0].to_string()),
            format_subformat_pattern: Some(split[1].to_string()),
        })
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut pat = String::new();
        if let Some(ref station_pattern) = self.station_pattern {
            pat.push_str(" ");
            pat.push_str(station_pattern);
            if let Some(ref stream_pattern) = self.stream_pattern {
                pat.push_str(" ");
                pat.push_str(stream_pattern);
                if let Some(ref format_subformat_pattern) = self.format_subformat_pattern {
                    pat.push_str(".");
                    pat.push_str(format_subformat_pattern);
                }
            }
        }

        write!(f, "{} {}{}", Self::NAME, self.item, pat)
    }
}

/// Enumeration of `INFO` command items.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InfoItem {
    Id,
    Formats,
    Stations,
    Streams,
    Connections,
    Capabilities,
}

impl fmt::Display for InfoItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Id => "id",
            Self::Formats => "formats",
            Self::Stations => "stations",
            Self::Streams => "streams",
            Self::Connections => "connections",
            Self::Capabilities => "capabilities",
        };

        write!(f, "{}", s)
    }
}
