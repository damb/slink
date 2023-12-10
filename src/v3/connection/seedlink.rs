use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use crate::{Frame, SeedLinkError};

use crate::v3::packet::{
    END_SIGNATURE, ERROR_SIGNATURE, HEADER_SIZE, INFO_SIGNATURE, OK_SIGNATURE, RECORD_SIZE,
    SIGNATURE,
};

#[derive(Debug, Clone)]
enum SessionPhase {
    HandShaking,
    DataTransfer,
}

#[derive(Debug)]
pub struct SeedLinkCodec {
    session_phase: SessionPhase,
    buf: Vec<u8>,
}

impl SeedLinkCodec {
    /// Creates a new `SeedLinkCodec` instance.
    pub fn new() -> Self {
        Self {
            session_phase: SessionPhase::HandShaking,
            buf: Vec::with_capacity(8 * 1024),
        }
    }

    /// Switches into data transfer phase.
    pub fn enable_data_transfer_phase(&mut self) {
        self.session_phase = SessionPhase::DataTransfer;
    }
    fn try_finalize_waveform_data_packet_frame(
        &mut self,
        src: &mut BytesMut,
        bytes_missing: usize,
    ) -> Option<Frame> {
        if let Some(buf) = self.try_to_finalize_frame_buffer(src, bytes_missing) {
            return Some(Frame::GenericDataPacket(buf));
        }

        None
    }

    fn try_finalize_info_packet_frame(
        &mut self,
        src: &mut BytesMut,
        bytes_missing: usize,
    ) -> Option<Frame> {
        if let Some(buf) = self.try_to_finalize_frame_buffer(src, bytes_missing) {
            return Some(Frame::InfoPacket(buf));
        }

        None
    }

    fn try_to_finalize_frame_buffer(
        &mut self,
        src: &mut BytesMut,
        bytes_missing: usize,
    ) -> Option<Vec<u8>> {
        if src.len() < bytes_missing {
            return None;
        }

        self.buf.extend_from_slice(&src[..bytes_missing]);
        src.advance(bytes_missing);

        let copied = self.buf.to_vec();
        self.buf.clear();

        Some(copied)
    }

    fn try_finalize_packet_frame(&mut self, src: &mut BytesMut) -> Option<Frame> {
        debug_assert!(self.buf.len() <= HEADER_SIZE);

        if self.buf.len() < HEADER_SIZE {
            // try to buffer remaining header bytes
            let bytes_missing = HEADER_SIZE - self.buf.len();
            if src.remaining() < bytes_missing {
                return None;
            }

            self.buf.extend_from_slice(&src[..bytes_missing]);
            src.advance(bytes_missing);
        }

        if &self.buf[..INFO_SIGNATURE.len()] == INFO_SIGNATURE {
            return self.try_finalize_info_packet_frame(src, RECORD_SIZE);
        }

        return self.try_finalize_waveform_data_packet_frame(src, RECORD_SIZE);
    }
}

impl Decoder for SeedLinkCodec {
    type Item = Frame;
    type Error = SeedLinkError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.session_phase {
            SessionPhase::HandShaking => {
                if self.buf == INFO_SIGNATURE {
                    return Ok(self.try_finalize_info_packet_frame(
                        src,
                        HEADER_SIZE + RECORD_SIZE - INFO_SIGNATURE.len(),
                    ));
                }

                loop {
                    if src.is_empty() {
                        return Ok(None);
                    }

                    let byte = src.get_u8();
                    match byte {
                        // XXX(damb): response lines are terminated with <CR><LF> (i.e. b"\r\n")
                        // <LF>
                        10 => {
                            // remove <CR> (i.e. b"\r")
                            self.buf.pop();

                            if self.buf == OK_SIGNATURE {
                                self.buf.clear();
                                return Ok(Some(Frame::Ok));
                            }

                            if self.buf == ERROR_SIGNATURE {
                                self.buf.clear();
                                return Ok(Some(Frame::Error));
                            }

                            let copied = self.buf.to_vec();
                            self.buf.clear();

                            return Ok(Some(Frame::Line(copied)));
                        }
                        _ => self.buf.push(byte),
                    }

                    if self.buf == END_SIGNATURE {
                        self.buf.clear();
                        return Ok(Some(Frame::End));
                    }

                    if self.buf == INFO_SIGNATURE {
                        return Ok(self.try_finalize_info_packet_frame(
                            src,
                            HEADER_SIZE + RECORD_SIZE - INFO_SIGNATURE.len(),
                        ));
                    }
                }
            }
            SessionPhase::DataTransfer => {
                if self.buf.len() >= SIGNATURE.len() && &self.buf[..SIGNATURE.len()] == SIGNATURE {
                    return Ok(self.try_finalize_packet_frame(src));
                }

                loop {
                    if src.is_empty() {
                        return Ok(None);
                    }

                    // TODO(damb): fix implementation -> before entering the loop try to finalize SL
                    // packets

                    self.buf.push(src.get_u8());

                    if self.buf == SIGNATURE {
                        return Ok(self.try_finalize_packet_frame(src));
                    } else if self.buf == END_SIGNATURE {
                        self.buf.clear();
                        return Ok(Some(Frame::End));
                    }
                }
            }
        }
    }
}

