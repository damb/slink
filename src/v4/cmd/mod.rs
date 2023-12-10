use std::fmt;
use std::str::{self, FromStr};

pub use auth::{Auth, AuthMethod};
pub use bye::Bye;
pub use data::{Data, SequenceNumber};
pub use end::End;
pub use endfetch::EndFetch;
pub use hello::Hello;
pub use info::{Info, InfoItem};
pub use select::{Select, SelectPattern};
pub use slproto::SlProto;
pub use station::Station;
pub use unknown::Unknown;
pub use useragent::{UserAgent, UserAgentInfo};

use crate::ProtocolErrorV4;

mod auth;
mod bye;
mod data;
mod end;
mod endfetch;
mod hello;
mod info;
mod select;
mod slproto;
mod station;
mod unknown;
mod useragent;

/// Enumeration of SeedLink `v4` commands.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    Auth(Auth),
    Bye(Bye),
    Data(Data),
    End(End),
    EndFetch(EndFetch),
    Hello(Hello),
    Info(Info),
    Select(Select),
    SlProto(SlProto),
    Station(Station),
    Unknown(Unknown),
    UserAgent(UserAgent),
}

impl Command {
    /// Parses the command from a buffer.
    pub fn parse(buf: &[u8]) -> Result<Self, ProtocolErrorV4> {
        let s =
            String::from_utf8(buf.to_vec()).map_err(|_| ProtocolErrorV4::unsupported_command())?;

        Self::from_str(s.as_str())
    }
}

impl str::FromStr for Command {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<Command, Self::Err> {
        assert!(!s.is_empty());
        let split: Vec<&str> = s.splitn(2, [' ', '\t']).collect();

        let cmd_id = split[0].to_lowercase();

        let cmd = match cmd_id.as_str() {
            Auth::NAME => {
                check_cmd_length(&split, 2)?;
                Self::Auth(Auth::from_str(split[1])?)
            }
            Bye::NAME => {
                check_cmd_length(&split, 1)?;
                Self::Bye(Bye)
            }
            Data::NAME => {
                if split.len() == 2 {
                    Self::Data(Data::default())
                } else {
                    Self::Data(Data::from_str(split[1])?)
                }
            }
            End::NAME => {
                check_cmd_length(&split, 1)?;
                Self::End(End)
            }
            EndFetch::NAME => {
                check_cmd_length(&split, 1)?;
                Self::EndFetch(EndFetch)
            }
            Hello::NAME => {
                check_cmd_length(&split, 1)?;
                Self::Hello(Hello)
            }
            Info::NAME => {
                let res = check_cmd_length(&split, 2);
                if let Err(mut err) = res {
                    err.info = true;
                    return Err(err);
                }

                let res = Info::from_str(split[1]);
                if let Err(mut err) = res {
                    err.info = true;
                    return Err(err);
                }

                Self::Info(res.unwrap())
            }
            Select::NAME => {
                check_cmd_length(&split, 2)?;
                Self::Select(Select::from_str(split[1])?)
            }
            SlProto::NAME => {
                check_cmd_length(&split, 2)?;
                Self::SlProto(SlProto::from_str(split[1])?)
            }
            Station::NAME => {
                check_cmd_length(&split, 2)?;
                Self::Station(Station::from_str(split[1])?)
            }
            UserAgent::NAME => {
                check_cmd_length(&split, 2)?;
                Self::UserAgent(UserAgent::from_str(split[1])?)
            }
            other => Self::Unknown(Unknown::new(other)),
        };

        Ok(cmd)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match *self {
            Self::Auth(ref cmd) => cmd.to_string(),
            Self::Bye(ref cmd) => cmd.to_string(),
            Self::Data(ref cmd) => cmd.to_string(),
            Self::End(ref cmd) => cmd.to_string(),
            Self::EndFetch(ref cmd) => cmd.to_string(),
            Self::Hello(ref cmd) => cmd.to_string(),
            Self::Info(ref cmd) => cmd.to_string(),
            Self::Select(ref cmd) => cmd.to_string(),
            Self::SlProto(ref cmd) => cmd.to_string(),
            Self::Station(ref cmd) => cmd.to_string(),
            Self::Unknown(ref cmd) => cmd.to_string(),
            Self::UserAgent(ref cmd) => cmd.to_string(),
        };

        write!(f, "{}", s)
    }
}

fn check_cmd_length(cmd: &[&str], expected_length: usize) -> Result<(), ProtocolErrorV4> {
    if cmd.len() != expected_length {
        Err(ProtocolErrorV4::incorrect_arguments())
    } else {
        Ok(())
    }
}
