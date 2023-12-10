use slink::ProtocolErrorV4;
use slink::{CommandV4, SequenceNumberV4};

use crate::select::Select;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum State {
    Station,
    Select,
    Finished,
    Error,
}

impl State {
    fn next(self, cmd: &CommandV4) -> Self {
        match (self, cmd) {
            (Self::Station, CommandV4::Select(_)) => Self::Select,
            (Self::Station, CommandV4::Data(_)) => Self::Finished,
            (Self::Select, CommandV4::Select(_)) => Self::Select,
            (Self::Select, CommandV4::Data(_)) => Self::Finished,
            (_, _) => Self::Error,
        }
    }
}

/// Structure keeping track of the stream selection for a station.
#[derive(Debug, Clone)]
pub struct StationNegotiator {
    pub select: Select,

    state: State,
}

impl StationNegotiator {
    /// Creates a new negotiator.
    pub fn new(select: Select) -> Self {
        Self {
            select,
            state: State::Station,
        }
    }

    /// Transitions the negotiator by feeding the next command.
    pub fn next(&mut self, cmd: &CommandV4) -> Result<(), ProtocolErrorV4> {
        self.state = self.state.next(cmd);
        if self.state == State::Error {
            return Err(ProtocolErrorV4::unexpected_command());
        }

        match cmd {
            CommandV4::Select(cmd) => {
                for select_pattern in cmd.iter() {
                    self.select.apply(
                        select_pattern.exclude,
                        &select_pattern.stream_pattern,
                        &select_pattern.format_subformat_pattern,
                        &select_pattern.filter,
                    );
                }
            }
            CommandV4::Data(cmd) => {
                if let Some(ref seq_num) = cmd.seq_num {
                    self.select.set_seq_num(seq_num);
                } else {
                    // If the sequence number is omitted than start the transfer from the next
                    // available packet.
                    self.select.set_seq_num(&SequenceNumberV4::Next);
                }

                if let Some(ref start_time) = cmd.start_time {
                    self.select.set_time(start_time, &cmd.end_time)
                }
            }
            _ => {}
        };

        Ok(())
    }
}
