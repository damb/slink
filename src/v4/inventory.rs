use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use time::OffsetDateTime;

const SID_DELIMITER: char = '_';

/// SeedLink v4 station identifier.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StationId {
    /// Network code
    net_code: String,
    /// Station code
    sta_code: String,
}

impl StationId {
    /// Returns the network code.
    pub fn net_code(&self) -> &str {
        &self.net_code
    }

    /// Returns the station code.
    pub fn sta_code(&self) -> &str {
        &self.sta_code
    }
}

impl<'de> Deserialize<'de> for StationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let s: &str = Deserialize::deserialize(deserializer)?;

        let split: Vec<&str> = s.split(SID_DELIMITER).collect();
        if split.len() != 2 {
            return Err(D::Error::custom("invalid station identifier"));
        }

        // http://docs.fdsn.org/projects/source-identifiers/en/v1.0/definition.html
        let net_code = split[0].to_string();
        if net_code.len() < 1 || net_code.len() > 8 {
            return Err(D::Error::custom(
                "invalid network code identifier (invalid length)",
            ));
        }

        let sta_code = split[1].to_string();
        if sta_code.len() < 1 || sta_code.len() > 8 {
            return Err(D::Error::custom(
                "invalid station code identifier (invalid length)",
            ));
        }

        Ok(Self { net_code, sta_code })
    }
}

impl Serialize for StationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for StationId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.net_code, SID_DELIMITER, self.sta_code)
    }
}

/// Structure representing a SeedLink v4 station in the inventory.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Station {
    /// Station identifier
    id: StationId,
    /// Description
    description: String,
    /// First packet sequence number available.
    start_seq: u64,
    /// Next sequence number available (i.e. last sequence number available + 1).
    end_seq: u64,
    /// How many seconds to wait for gaps to fill: -1 = undefined
    #[serde(skip_serializing_if = "Option::is_none")]
    backfill: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<Vec<Stream>>,
}

impl Station {
    /// Returns the station identifier.
    pub fn id(&self) -> &StationId {
        &self.id
    }

    /// Returns the station description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the first packet sequence number available.
    pub fn start_seq(&self) -> u64 {
        self.start_seq
    }

    /// Returnst the next sequence number available (i.e. last sequence number available + 1).
    pub fn end_seq(&self) -> u64 {
        self.end_seq
    }

    /// Returns how many seconds to wait for gaps to fill: -1 = undefined
    pub fn backfill(&self) -> &Option<i32> {
        &self.backfill
    }

    pub fn streams(&self) -> &Option<Vec<Stream>> {
        &self.stream
    }
}

/// SeedLink v4 stream identifier.
#[derive(Debug, Clone, Eq, PartialEq)]
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

impl<'de> Deserialize<'de> for StreamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let s: &str = Deserialize::deserialize(deserializer)?;

        let split: Vec<&str> = s.split(SID_DELIMITER).collect();
        if split.len() != 4 {
            return Err(D::Error::custom("invalid stream identifier"));
        }

        // http://docs.fdsn.org/projects/source-identifiers/en/v1.0/definition.html
        let loc_code = split[0].to_string();
        if loc_code.len() > 8 || loc_code == "--" {
            return Err(D::Error::custom("invalid location code identifier"));
        }

        // Band code may be empty for non-time series data.
        let band_code = match split[1].len() {
            0 => String::new(),
            1 => split[1].to_string(),
            _ => return Err(D::Error::custom("invalid band code identifier")),
        };

        // Source code must not be empty.
        if split[2].len() != 1 {
            return Err(D::Error::custom("invalid source code identifier"));
        }

        // Subsource code may be empty.
        let subsource_code = match split[3].len() {
            0 => String::new(),
            1 => split[3].to_string(),
            _ => return Err(D::Error::custom("invalid subsource code identifier")),
        };

        Ok(Self {
            loc_code: split[0].to_string(),
            band_code,
            source_code: split[2].to_string(),
            subsource_code,
        })
    }
}

impl Serialize for StreamId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
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

/// Enumeration
#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum StreamOrigin {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "converted")]
    Converted,
}

impl fmt::Display for StreamOrigin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Native => "native",
            Self::Converted => "converted",
        };
        write!(f, "{}", s)
    }
}

/// Enumeration of SeedLink v4 format codes.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum StreamFormat {
    /// miniSEED 2.x
    #[serde(rename = "2")]
    MiniSeed2,
    /// miniSEED 3.x with FDSN source identifier
    #[serde(rename = "3")]
    MiniSeed3,
}

/// Enumeration of SeedLink v4 subformat codes.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum StreamSubFormat {
    /// Data/generic
    #[serde(rename = "D")]
    Data,
    /// Event detection
    #[serde(rename = "E")]
    Event,
    /// Calibration
    #[serde(rename = "C")]
    Calibration,
    /// Opaque
    #[serde(rename = "O")]
    Opaque,
    /// Timing exception
    #[serde(rename = "T")]
    Timing,
    /// Log
    #[serde(rename = "L")]
    Log,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Stream {
    /// Stream identifier
    id: StreamId,
    /// Stream format
    format: StreamFormat,
    /// Stream subformat
    subformat: StreamSubFormat,
    /// Origin of stream
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<StreamOrigin>,
    /// Start time of the first packet buffered.
    #[serde(with = "seedlink_datetime")]
    start_time: OffsetDateTime,
    /// End time of the last packet buffered.
    #[serde(with = "seedlink_datetime")]
    end_time: OffsetDateTime,
}

impl Stream {
    /// Returns the stream identifier.
    pub fn id(&self) -> &StreamId {
        &self.id
    }

    /// Returns the stream format.
    pub fn format(&self) -> &StreamFormat {
        &self.format
    }

    /// Returns the stream subformat.
    pub fn subformat(&self) -> &StreamSubFormat {
        &self.subformat
    }

    /// Returns the stream origin.
    pub fn origin(&self) -> &Option<StreamOrigin> {
        &self.origin
    }

    /// Returns the time of the first packet buffered.
    pub fn start_time(&self) -> &OffsetDateTime {
        &self.start_time
    }

    /// Returns the end time of the last packet buffered.
    pub fn end_time(&self) -> &OffsetDateTime {
        &self.end_time
    }
}

mod seedlink_datetime {

    use serde::{self, Deserialize, Deserializer, Serializer};
    use time::format_description::FormatItem;
    use time::macros::format_description;
    use time::{OffsetDateTime, PrimitiveDateTime};

    const FORMAT: &[FormatItem<'static>] = format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second][optional [.[subsecond]]]Z"
    );

    pub fn serialize<S>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = datetime.format(FORMAT).unwrap();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let buf = Deserialize::deserialize(deserializer)?;
        Ok(PrimitiveDateTime::parse(buf, FORMAT)
            .map_err(D::Error::custom)?
            .assume_utc())
    }
}

#[cfg(test)]
mod tests {

    use super::{Station, StationId, Stream, StreamFormat, StreamId, StreamSubFormat};

    use serde_json::Value;
    use time::macros::datetime;

    use pretty_assertions::assert_eq;

    #[test]
    fn serde_empty() {
        let json = r#"[]"#;

        let inv: Vec<Station> = serde_json::from_str(json).unwrap();
        assert_eq!(inv, vec![]);
        let obj = serde_json::json!(inv);
        assert_eq!(json, obj.to_string());
    }

    #[test]
    fn serde_station_no_streams() {
        let json = r#"
            [
                {
                    "id": "AW_VNA1",
                    "description": "Station Neumayer OBS, Antarctica",
                    "start_seq": 5648896,
                    "end_seq": 5777233
                }
            ]
        "#;

        let inv: Vec<Station> = serde_json::from_str(json).unwrap();
        let sta = Station {
            id: StationId {
                net_code: "AW".to_string(),
                sta_code: "VNA1".to_string(),
            },
            description: "Station Neumayer OBS, Antarctica".to_string(),
            start_seq: 5648896,
            end_seq: 5777233,
            backfill: None,
            stream: None,
        };
        assert_eq!(inv, vec![sta]);

        let obj = serde_json::json!(inv);
        let v: Value = serde_json::from_str(json).unwrap();
        assert_eq!(v.to_string(), obj.to_string());
    }

    #[test]
    fn serde_station_multi_streams() {
        let json = r#"
            [
                {
                    "id": "AW_VNA1",
                    "description": "Station Neumayer OBS, Antarctica",
                    "start_seq": 5648896,
                    "end_seq": 5777233,
                    "stream": [
                        {
                            "id": "_H_H_Z",
                            "format": "2",
                            "subformat": "D",
                            "start_time": "2012-12-29T14:18:45.89Z",
                            "end_time": "2012-12-29T14:37:57.27Z"
                        },
                        {
                            "id": "_H_H_E",
                            "format": "2",
                            "subformat": "D",
                            "start_time": "2012-12-29T14:18:45.89Z",
                            "end_time": "2012-12-29T14:37:53.22Z"
                        },
                        {
                            "id": "_H_H_N",
                            "format": "2",
                            "subformat": "D",
                            "start_time": "2012-12-29T14:18:45.89Z",
                            "end_time": "2012-12-29T14:37:58.01Z"
                        }
                    ]
                }
            ]
        "#;

        let inv: Vec<Station> = serde_json::from_str(json).unwrap();
        let sta = Station {
            id: StationId {
                net_code: "AW".to_string(),
                sta_code: "VNA1".to_string(),
            },
            description: "Station Neumayer OBS, Antarctica".to_string(),
            start_seq: 5648896,
            end_seq: 5777233,
            backfill: None,
            stream: Some(vec![
                Stream {
                    id: StreamId {
                        loc_code: String::new(),
                        band_code: "H".to_string(),
                        source_code: "H".to_string(),
                        subsource_code: "Z".to_string(),
                    },
                    format: StreamFormat::MiniSeed2,
                    subformat: StreamSubFormat::Data,
                    origin: None,
                    start_time: datetime!(2012-12-29 14:18:45.8900 UTC),
                    end_time: datetime!(2012-12-29 14:37:57.2700 UTC),
                },
                Stream {
                    id: StreamId {
                        loc_code: String::new(),
                        band_code: "H".to_string(),
                        source_code: "H".to_string(),
                        subsource_code: "E".to_string(),
                    },
                    format: StreamFormat::MiniSeed2,
                    subformat: StreamSubFormat::Data,
                    origin: None,
                    start_time: datetime!(2012-12-29 14:18:45.8900 UTC),
                    end_time: datetime!(2012-12-29 14:37:53.2200 UTC),
                },
                Stream {
                    id: StreamId {
                        loc_code: String::new(),
                        band_code: "H".to_string(),
                        source_code: "H".to_string(),
                        subsource_code: "N".to_string(),
                    },
                    format: StreamFormat::MiniSeed2,
                    subformat: StreamSubFormat::Data,
                    origin: None,
                    start_time: datetime!(2012-12-29 14:18:45.8900 UTC),
                    end_time: datetime!(2012-12-29 14:37:58.0100 UTC),
                },
            ]),
        };
        assert_eq!(inv, vec![sta]);

        let obj = serde_json::json!(inv);
        let v: Value = serde_json::from_str(json).unwrap();
        assert_eq!(v.to_string(), obj.to_string());
    }

    #[test]
    fn deserialize_station_single_stream() {
        let json = r#"
            [
                {
                    "id": "YU_TRML",
                    "description": "TRML",
                    "start_seq": 57163777,
                    "end_seq": 57165057,
                    "stream": [
                        {
                            "id": "_H_H_Z",
                            "format": "2",
                            "subformat": "D",
                            "start_time": "2012-12-29T14:18:45.89Z",
                            "end_time": "2012-12-29T14:37:57.27Z"
                        }
                    ]
                }
            ]
        "#;

        let inv: Vec<Station> = serde_json::from_str(json).unwrap();
        let sta = Station {
            id: StationId {
                net_code: "YU".to_string(),
                sta_code: "TRML".to_string(),
            },
            description: "TRML".to_string(),
            backfill: None,
            start_seq: 57163777,
            end_seq: 57165057,
            stream: Some(vec![Stream {
                id: StreamId {
                    loc_code: "".to_string(),
                    band_code: "H".to_string(),
                    source_code: "H".to_string(),
                    subsource_code: "Z".to_string(),
                },
                format: StreamFormat::MiniSeed2,
                subformat: StreamSubFormat::Data,
                origin: None,
                start_time: datetime!(2012-12-29 14:18:45.89 UTC),
                end_time: datetime!(2012-12-29 14:37:57.27 UTC),
            }]),
        };
        assert_eq!(inv, vec![sta]);

        let obj = serde_json::json!(inv);
        let v: Value = serde_json::from_str(json).unwrap();
        assert_eq!(v.to_string(), obj.to_string());
    }
}
