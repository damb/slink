use std::fmt;
use std::str;

use time::{format_description::well_known::Iso8601, OffsetDateTime};

use crate::ProtocolErrorV4;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SequenceNumber {
    All,
    Next,
    Number(u64),
}

impl fmt::Display for SequenceNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Next => write!(f, "next"),
            Self::Number(seq_num) => write!(f, "{}", seq_num),
        }
    }
}

impl str::FromStr for SequenceNumber {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<SequenceNumber, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "all" => SequenceNumber::All,
            "next" => SequenceNumber::Next,
            other => SequenceNumber::Number(
                u64::from_str_radix(other, 10)
                    .map_err(|_| ProtocolErrorV4::incorrect_arguments())?,
            ),
        })
    }
}

/// Action command to enable *real-time* mode for a given station.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Data {
    pub seq_num: Option<SequenceNumber>,
    pub start_time: Option<OffsetDateTime>,
    pub end_time: Option<OffsetDateTime>,
}

impl Data {
    pub const NAME: &'static str = "data";

    /// Creates a new data command.
    pub fn new(
        seq_num: Option<SequenceNumber>,
        begin: Option<OffsetDateTime>,
        end: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            seq_num,
            start_time: begin,
            end_time: end,
        }
    }

    pub fn all(start_time: Option<OffsetDateTime>, end_time: Option<OffsetDateTime>) -> Self {
        Self {
            seq_num: Some(SequenceNumber::All),
            start_time,
            end_time,
        }
    }

    pub fn next(start_time: Option<OffsetDateTime>, end_time: Option<OffsetDateTime>) -> Self {
        Self {
            seq_num: Some(SequenceNumber::Next),
            start_time,
            end_time,
        }
    }
}

impl str::FromStr for Data {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<Data, Self::Err> {
        let split: Vec<&str> = s.split(' ').collect();
        if split.len() > 3 {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        if split.is_empty() {
            return Ok(Data::default());
        }

        let seq_num = split[0].parse::<SequenceNumber>()?;

        if split.len() == 1 {
            return Ok(Data {
                seq_num: Some(seq_num),
                start_time: None,
                end_time: None,
            });
        }

        let start_time = OffsetDateTime::parse(split[1], &Iso8601::DEFAULT)
            .map_err(|_| ProtocolErrorV4::incorrect_arguments())?;
        if !start_time.offset().is_utc() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }
        if split.len() == 2 {
            return Ok(Data {
                seq_num: Some(seq_num),
                start_time: Some(start_time),
                end_time: None,
            });
        }

        let end_time = OffsetDateTime::parse(split[2], &Iso8601::DEFAULT)
            .map_err(|_| ProtocolErrorV4::incorrect_arguments())?;
        if !end_time.offset().is_utc() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        Ok(Data {
            seq_num: Some(seq_num),
            start_time: Some(start_time),
            end_time: Some(end_time),
        })
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut seq_num_time_str = String::new();
        if let Some(ref seq_num) = self.seq_num {
            seq_num_time_str = format!(" {}", seq_num);
            if let Some(ref begin) = self.start_time {
                seq_num_time_str
                    .push_str(format!(" {}", begin.format(&Iso8601::DEFAULT).unwrap()).as_str());
                if let Some(ref end) = self.end_time {
                    seq_num_time_str
                        .push_str(format!(" {}", end.format(&Iso8601::DEFAULT).unwrap()).as_str());
                }
            }
        }

        write!(f, "{}{}", Data::NAME, seq_num_time_str)
    }
}
