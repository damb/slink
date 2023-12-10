use std::cmp;
use std::convert::From;
use std::io::{self, Write};

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use tracing::trace;

use slink::{CommandV4, ProtocolErrorV4};

use crate::client::FromServer;
use crate::{ClientId, DEFAULT_PROTO_VERSION};

/// Maximum length of the command line is 255 characters, including the `<CR><LF>` terminator.
const MAX_COMMAND_LINE_LENGTH: usize = 255;

/// Enumeration of errors that can occur when parsing SeedLink commands.
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("maximum command line length exceeded")]
    CommandLineTooLong,
    #[error(transparent)]
    ProtocolError(#[from] ProtocolErrorV4),
    #[error(transparent)]
    IoError(#[from] io::Error),
}

/// SeedLink protocol version structure.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

impl From<(u8, u8)> for ProtocolVersion {
    fn from(value: (u8, u8)) -> Self {
        Self {
            major: value.0,
            minor: value.1,
        }
    }
}

/// A simple [`Decoder`] implementation that both splits up data into lines and parses SeedLink
/// commands.
///
/// Note that SeedLink commands consist of an ASCII string followed by zero or more arguments
/// separated by spaces and terminated with carriage return (`\r`, `<CR>`, ASCII code 13) followed
/// by linefeed (`\n`, <LF>`, ASCII code 10).
/// The codec also accepts a single `<CR>` or `<LF>` as a command terminator. Empty command lines
/// are ignored.
///
/// `SeedLinkCodec::decode` will return a `ParseError` when a line exceeds the length limit (i.e.
/// 255 characters). Subsequent calls will discard up to 255 bytes from that line until a line
/// ending character is reached, returning `None` until the line over the limit has been fully
/// discarded. After that point, calls to `decode` will function as normal.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SeedLinkCodec {
    client_id: ClientId,

    next_index: usize,
    is_discarding: bool,

    protocol_version: ProtocolVersion,
    protocol_version_locked: bool,
}

impl SeedLinkCodec {
    /// Creates a new codec instance.
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            next_index: 0,
            is_discarding: false,
            protocol_version: DEFAULT_PROTO_VERSION.into(),
            protocol_version_locked: false,
        }
    }

    /// Returns the configured SeedLink protocol version.
    pub fn protocol_version(&self) -> &ProtocolVersion {
        &self.protocol_version
    }

    /// Sets the SeedLink protocol version used by the codec.
    ///
    /// Returns an error if setting the protocol version is not allowed.
    pub fn try_set_protocol_version(
        &mut self,
        protocol_version: ProtocolVersion,
    ) -> Result<(), ProtocolErrorV4> {
        if self.is_locked_protocol_version() {
            let mut err = ProtocolErrorV4::unexpected_command();
            err.message = Some(
                format!(
                    "{}: failed to switch protocol version",
                    err.code.description()
                )
                .into(),
            );

            return Err(err);
        }

        self.protocol_version = protocol_version;

        Ok(())
    }

    /// Locks protocol version configuration.
    pub fn lock_protocol_version(&mut self) {
        self.protocol_version_locked = true;
    }

    /// Returns whether the protocol version is locked.
    pub fn is_locked_protocol_version(&self) -> bool {
        self.protocol_version_locked
    }
}

impl Decoder for SeedLinkCodec {
    type Item = CommandV4;
    type Error = ParseError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<CommandV4>, ParseError> {
        // XXX(damb): slightly modified version of
        // https://docs.rs/tokio-util/latest/src/tokio_util/codec/lines_codec.rs.html#112-166
        // Reimplementing the decoder is required due to accepting a single `\r` as a line ending
        // is required.
        loop {
            // Determine how far into the buffer we'll search for a newline. If
            // there's no max_length set, we'll read to the end of the buffer.
            let read_to = cmp::min(MAX_COMMAND_LINE_LENGTH, buf.len());

            let newline_offset = buf[self.next_index..read_to]
                .iter()
                .position(|b| *b == b'\n' || *b == b'\r');

            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    // If we found a newline, discard up to that offset and
                    // then stop discarding. On the next iteration, we'll try
                    // to read a line normally.
                    buf.advance(offset + self.next_index + 1);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    // Otherwise, we didn't find a newline, so we'll discard
                    // everything we read. On the next iteration, we'll continue
                    // discarding up to max_len bytes unless we find a newline.
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Ok(None);
                    }
                }
                (false, Some(offset)) => {
                    // Found a line!
                    let mut newline_index = offset + self.next_index;
                    // Handle <CR><LF> i.e. "\r\n"
                    if b'\r' == buf[newline_index]
                        && newline_index + 1 < buf.len()
                        && b'\n' == buf[newline_index + 1]
                    {
                        newline_index += 1;
                    }

                    self.next_index = 0;
                    let line = buf.split_to(newline_index + 1);
                    let line = &line[..line.len() - 1];
                    let line = without_carriage_return(line);
                    if line.is_empty() {
                        // Ignore empty command lines
                        return Ok(None);
                    }

                    trace!("{:?}: <- {:?}", self.client_id, line);
                    let cmd = match self.protocol_version.major {
                        4 => CommandV4::parse(&line)?,
                        0_u8..=3_u8 | 5_u8..=u8::MAX => todo!(),
                    };

                    return Ok(Some(cmd));
                }
                (false, None) if buf.len() > MAX_COMMAND_LINE_LENGTH => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(ParseError::CommandLineTooLong);
                }
                (false, None) => {
                    // We didn't find a line or reach the length limit, so the next
                    // call will resume searching at the current offset.
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }
}

fn without_carriage_return(s: &[u8]) -> &[u8] {
    if let Some(&b'\r') = s.last() {
        &s[..s.len() - 1]
    } else {
        s
    }
}

impl Encoder<FromServer> for SeedLinkCodec {
    type Error = io::Error;

    fn encode(&mut self, item: FromServer, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match self.protocol_version.major {
            4 => match item {
                _ => todo!()
            },
            _ => todo!(),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::BufMut;

    use slink::{CommandV4, HelloCmdV4};

    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn decode_hello() {
        let mut codec = SeedLinkCodec::new(ClientId(42));
        let mut buffer = BytesMut::from("HELLO\r\n");
        let cmd = codec.decode(&mut buffer).unwrap();
        assert_eq!(cmd, Some(CommandV4::Hello(HelloCmdV4)));
    }
}
