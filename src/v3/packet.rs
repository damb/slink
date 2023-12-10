use std::io;
use std::str;

use mseed::{MSControlFlags, MSRecord};

use crate::SeedLinkResult;

/// SeedLink packet header size.
pub const HEADER_SIZE: usize = 8;
/// SeedLink packet record size.
pub const RECORD_SIZE: usize = 512;
/// SeedLink packet signature.
pub const SIGNATURE: &[u8; 2] = b"SL";
/// SeedLink info packet signature.
pub const INFO_SIGNATURE: &[u8; 6] = b"SLINFO";
/// SeedLink error packet signature.
pub const ERROR_SIGNATURE: &[u8; 5] = b"ERROR";
/// SeedLink end packet signature.
pub const END_SIGNATURE: &[u8; 3] = b"END";
/// SeedLink ok packet signature
pub const OK_SIGNATURE: &[u8; 2] = b"OK";
/// SeedLink info packet flag indicating that the info packet is the last packet for a
/// given request.
pub const INFO_TERMINATION_FLAG: &[u8; 1] = b"*";

#[derive(Debug)]
struct SeedLinkPacketBase {
    packet: Vec<u8>,
}

impl SeedLinkPacketBase {
    fn new(buf: Vec<u8>) -> Self {
        if buf.len() != HEADER_SIZE + RECORD_SIZE {}
        Self { packet: buf }
    }

    pub fn raw(&self) -> &[u8] {
        &self.packet
    }

    pub fn header(&self) -> &[u8] {
        &self.packet[..HEADER_SIZE]
    }

    pub fn raw_ms_record(&self) -> &[u8] {
        &self.packet[HEADER_SIZE..]
    }

    pub fn ms_record(&self, flags: MSControlFlags) -> SeedLinkResult<MSRecord> {
        MSRecord::parse(self.raw_ms_record(), flags).map_err(Into::into)
    }
}

/// A structure implementing a SeedLink info packet.
#[derive(Debug)]
pub struct SeedLinkInfoPacketV3 {
    base: SeedLinkPacketBase,
}

impl SeedLinkInfoPacketV3 {
    pub fn new(buf: Vec<u8>) -> Self {
        Self {
            base: SeedLinkPacketBase::new(buf),
        }
    }

    /// Returns the raw packet bytes.
    pub fn raw(&self) -> &[u8] {
        self.base.raw()
    }

    /// Returns whether the packet meets an error condition.
    pub fn is_err(&self) -> bool {
        match self.base.ms_record(MSControlFlags::empty()) {
            Ok(msr) => match msr.channel() {
                Ok(cha) => cha == "ERR",
                Err(_) => true,
            },
            Err(_) => true,
        }
    }

    /// Returns `true` if the packet is marked as the last packet for a request, else `false`.
    pub fn is_last(&self) -> bool {
        self.base.header()[HEADER_SIZE - 1] != INFO_TERMINATION_FLAG[0]
    }

    /// Returns the raw packet payload.
    pub fn raw_payload(&self) -> &[u8] {
        self.base.raw_ms_record()
    }

    /// Returns the decoded packet payload.
    pub fn payload(&self) -> SeedLinkResult<String> {
        let msr = self.base.ms_record(MSControlFlags::MSF_UNPACKDATA)?;

        if let Some(data_samples) = &msr.data_samples() {
            String::from_utf8(data_samples.to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()).into())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "missing payload").into())
        }
    }
}

/// A SeedLink packet shipping a generic data record.
#[derive(Debug)]
pub struct SeedLinkGenericDataPacketV3 {
    base: SeedLinkPacketBase,
}

impl SeedLinkGenericDataPacketV3 {
    pub fn new(buf: Vec<u8>) -> Self {
        Self {
            base: SeedLinkPacketBase::new(buf),
        }
    }

    /// Returns the raw packet bytes.
    pub fn raw(&self) -> &[u8] {
        self.base.raw()
    }

    /// Returns the raw packet payload.
    pub fn raw_payload(&self) -> &[u8] {
        self.base.raw_ms_record()
    }

    /// Returns the decoded packet payload.
    pub fn payload(&self, flags: MSControlFlags) -> SeedLinkResult<MSRecord> {
        self.base.ms_record(flags)
    }

    /// Returns the packet sequence number
    pub fn sequence_number_str(&self) -> SeedLinkResult<&str> {
        str::from_utf8(&self.base.header()[SIGNATURE.len()..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()).into())
    }

    /// Returns the decoded packet sequence number
    pub fn sequence_number(&self) -> SeedLinkResult<i32> {
        i32::from_str_radix(self.sequence_number_str()?, 16)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()).into())
    }
}

/// Enumeration of v3 SeedLink packets
#[derive(Debug)]
pub enum SeedLinkPacketV3 {
    Info(SeedLinkInfoPacketV3),
    GenericData(SeedLinkGenericDataPacketV3),
}

impl SeedLinkPacketV3 {
    pub fn is_info(&self) -> bool {
        match self {
            Self::Info(_) => true,
            Self::GenericData(_) => false,
        }
    }

    pub fn is_data(&self) -> bool {
        match self {
            Self::Info(_) => false,
            Self::GenericData(_) => true,
        }
    }
}

