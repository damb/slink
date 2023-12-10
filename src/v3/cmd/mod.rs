use std::fmt;

pub use batch::Batch;
pub use bye::Bye;
pub use data::Data;
pub use end::End;
pub use fetch::Fetch;
pub use hello::Hello;
pub use info::{Info, InfoItem};
pub use select::Select;
pub use self::time::Time;
pub use station::Station;
pub use unknown::Unknown;

use crate::Frame;

mod batch;
mod bye;
mod data;
mod end;
mod fetch;
mod hello;
mod info;
mod select;
mod station;
mod time;
mod unknown;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Bye(Bye),
    Hello(Hello),
    Info(Info),
    Batch(Batch),
    Station(Station),
    Select(Select),
    Data(Data),
    Fetch(Fetch),
    Time(Time),
    End(End),
    Unknown(Unknown),
}

impl Command {
    pub fn into_frame(&self) -> Frame {
        Frame::Line(self.to_string().as_bytes().to_vec())
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let serialized = match self {
            Self::Bye(cmd) => cmd.to_string(),
            Self::Hello(cmd) => cmd.to_string(),
            Self::Info(cmd) => cmd.to_string(),
            Self::Batch(cmd) => cmd.to_string(),
            Self::Station(cmd) => cmd.to_string(),
            Self::Select(cmd) => cmd.to_string(),
            Self::Data(cmd) => cmd.to_string(),
            Self::Fetch(cmd) => cmd.to_string(),
            Self::Time(cmd) => cmd.to_string(),
            Self::End(cmd) => cmd.to_string(),
            Self::Unknown(cmd) => cmd.to_string(),
        };
        write!(f, "{}", serialized)
    }
}

