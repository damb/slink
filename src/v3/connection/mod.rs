use std::io;

use futures::stream::StreamExt;
use quick_xml::de;
use time::PrimitiveDateTime;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_util::codec::FramedRead;
use tracing::{debug, instrument, warn};

use crate::{
    ActualConnection, BatchCmdV3, ByeCmdV3, CommandV3, EndCmdV3, Frame, HelloCmdV3, InfoCmdItemV3,
    InfoCmdV3, InventoryV3, SeedLinkError, SeedLinkInfoPacketV3, SeedLinkResult, StreamConfig,
    TcpConnection,
};

use negotiate::Negotiator;
use seedlink::SeedLinkCodec;

mod negotiate;
mod seedlink;

#[derive(Debug)]
struct FramedTcpConnection {
    read: FramedRead<OwnedReadHalf, SeedLinkCodec>,
    write: BufWriter<OwnedWriteHalf>,

    open: bool,
}

#[derive(Debug)]
enum ActualFramedConnection {
    Tcp(FramedTcpConnection),
}

impl ActualFramedConnection {
    pub async fn flush(&mut self) -> SeedLinkResult<()> {
        match self {
            Self::Tcp(FramedTcpConnection { ref mut write, .. }) => write.flush().await?,
        }

        Ok(())
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> SeedLinkResult<()> {
        match self {
            Self::Tcp(FramedTcpConnection { ref mut write, .. }) => write.write_all(buf).await?,
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> SeedLinkResult<()> {
        match self {
            Self::Tcp(FramedTcpConnection {
                ref mut write,
                ref mut open,
                ..
            }) => {
                _ = write.shutdown().await;
                *open = false;
            }
        }

        Ok(())
    }

    pub fn is_open(&self) -> bool {
        match self {
            Self::Tcp(FramedTcpConnection { ref open, .. }) => *open,
        }
    }
}

impl ActualFramedConnection {
    /// Creates a new `ActualFramedConnection` from the actual connection `con`.
    fn new(con: ActualConnection) -> Self {
        // TODO(damb): allow to configure read buffer size
        match con {
            ActualConnection::Tcp(TcpConnection { rw, open }) => {
                let (read, write) = rw.into_split();
                Self::Tcp(FramedTcpConnection {
                    read: FramedRead::with_capacity(read, SeedLinkCodec::new(), 8 * 1024),
                    write: BufWriter::with_capacity(255, write),
                    open,
                })
            }
        }
    }
}

/// Enumeration representing the various connection states.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FramedConnectionState {
    Initialized,
    HandShaking,
    DataTransfer,
    Closed,
}

/// Stateful SeedLink framed connection structure encapsulating the actual connection.
///
/// Receives and sends frames to a remote peer.
#[derive(Debug)]
pub(crate) struct FramedConnectionV3 {
    con: ActualFramedConnection,
    state: FramedConnectionState,
    batch_cmd_mode: bool,

    expect_info_resp: bool,
}

impl FramedConnectionV3 {
    /// Creates a new `FramedConnection`, backed by the actual connection `con`.
    pub fn new(con: ActualConnection) -> Self {
        Self {
            con: ActualFramedConnection::new(con),
            state: FramedConnectionState::Initialized,
            batch_cmd_mode: false,

            expect_info_resp: false,
        }
    }

    /// Returns whether the connection is open.
    pub fn is_open(&self) -> bool {
        self.con.is_open()
    }

    /// Returns whether batch command mode is enabled.
    pub fn batch_cmd_mode(&self) -> bool {
        self.batch_cmd_mode
    }

    /// Sends the `HELLO` command and returns the corresponding response.
    #[instrument(skip(self))]
    pub async fn say_hello(&mut self) -> SeedLinkResult<(String, String)> {
        if self.state >= FramedConnectionState::HandShaking {
            return Err(SeedLinkError::ClientError(
                "invalid connection state".to_string(),
            ));
        }

        let cmd = CommandV3::Hello(HelloCmdV3);
        let frame = cmd.into_frame();

        debug!("sending command: '{}'", cmd);
        self.write_frame(&frame).await?;

        let first_response_line = self.read_line_frame().await?;
        let second_response_line = self.read_line_frame().await?;

        Ok((first_response_line, second_response_line))
    }

    /// Performs a connection shutdown.
    #[instrument(skip(self))]
    pub async fn shutdown(&mut self) -> SeedLinkResult<()> {
        self.say_bye().await?;
        self.con.shutdown().await?;
        self.state = FramedConnectionState::Closed;

        Ok(())
    }

    /// Requests the SeedLink server's information at level `item` and returns XML.
    #[instrument(skip(self))]
    pub async fn request_info(&mut self, item: InfoCmdItemV3) -> SeedLinkResult<String> {
        self.try_send_info(item).await?;
        self.expect_info_resp = true;

        let mut info_packet_buf = String::new();
        loop {
            match self.read_frame().await? {
                Frame::InfoPacket(buf) => {
                    let mut packet = SeedLinkInfoPacketV3::new(buf);
                    if packet.is_err() {
                        return Err(SeedLinkError::UnsupportedCommand(
                            "INFO level request is not supported.".to_string(),
                        ));
                    }
                    let payload = packet.payload()?;
                    // debug!("{}", payload);
                    info_packet_buf.push_str(&payload);

                    if packet.is_last() {
                        break;
                    }
                }
                _ => {
                    // ignore
                }
            };
        }

        self.expect_info_resp = false;

        Ok(info_packet_buf)
    }

    /// Configures the connection and completes the handshaking.
    #[instrument(skip(self))]
    pub async fn configure(
        &mut self,
        stream_configs: &[StreamConfig],
        data_transfer_mode: &SeedLinkDataTransferModeV3,
        batch_cmd_mode: bool,
    ) -> SeedLinkResult<()> {
        if stream_configs.len() == 0 {
            return Ok(());
        }

        if batch_cmd_mode {
            let cmd = CommandV3::Batch(BatchCmdV3);
            let frame = cmd.into_frame();

            debug!("sending command: '{}'", cmd);
            self.write_frame(&frame).await?;

            match self.read_frame().await? {
                Frame::Ok => {
                    debug!("response: batch is OK (batch command mode enabled)");
                    self.batch_cmd_mode = true;
                }
                Frame::Error => {
                    warn!("response: batch is ERROR (failed to switch to batch command mode)");
                    return Err(SeedLinkError::UnsupportedCommand(
                        "failed to switch to batch mode".to_string(),
                    ));
                }
                frame => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "response: invalid response to command ({}): {:?}",
                            cmd, frame
                        ),
                    )
                    .into());
                }
            }
        }

        self.state = FramedConnectionState::HandShaking;

        let mut accepted_sta_cnt = 0;
        for stream_config in stream_configs {
            let negotiator = Negotiator { stream_config };
            if negotiator.negotiate(self, &data_transfer_mode).await? {
                accepted_sta_cnt += 1;
            }
        }

        if accepted_sta_cnt == 0 {
            self.state = FramedConnectionState::Initialized;
            warn!("no station selected");
        } else {
            // switch to data transfer mode
            self.state = FramedConnectionState::DataTransfer;
            match &mut self.con {
                ActualFramedConnection::Tcp(FramedTcpConnection { ref mut read, .. }) => {
                    read.decoder_mut().enable_data_transfer_phase();
                }
            }

            // end handshaking in multi-station mode
            let cmd = CommandV3::End(EndCmdV3);
            let frame = cmd.into_frame();

            debug!("sending command: '{}'", cmd);
            self.write_frame(&frame).await?;
        }

        Ok(())
    }

    /// Tries to send a keep alive packet to the SeedLink server.
    pub(crate) async fn try_send_keep_alive(&mut self) -> SeedLinkResult<()> {
        let resp = match self.try_send_info(InfoCmdItemV3::Id).await {
            Ok(()) => Ok(()),
            Err(e) => match e {
                SeedLinkError::ClientError(_) => {
                    // ignore client errors
                    Ok(())
                }
                e => Err(e),
            },
        };
        self.expect_info_resp = true;
        resp
    }

    pub(crate) fn ack_keep_alive(&mut self) {
        self.expect_info_resp = false;
    }

    /// Low level function which writes a `Frame` literal to the underlying actual framed connection.
    #[instrument(skip(self))]
    pub async fn write_frame(&mut self, frame: &Frame) -> SeedLinkResult<()> {
        match frame {
            Frame::Line(buf) => {
                self.con.write_all(buf).await?;
                self.con.write_all(b"\r\n").await?;
                self.con.flush().await?;
            }
            _ => unimplemented!(),
        }

        Ok(())
    }

    /// Low level function which reads a `Frame` literal from the underlying actual framed connection.
    #[instrument(skip(self))]
    pub async fn read_frame(&mut self) -> SeedLinkResult<Frame> {
        match &mut self.con {
            ActualFramedConnection::Tcp(FramedTcpConnection { ref mut read, .. }) => {
                if let Some(frame) = read.next().await {
                    return frame;
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::BrokenPipe, "disconnected").into())
    }

    /// Reads a response line frame from the underlying actual framed connection.
    async fn read_line_frame(&mut self) -> SeedLinkResult<String> {
        match self.read_frame().await? {
            Frame::Line(buf) => String::from_utf8(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()).into()),
            frame => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("response: invalid response: {:?}", frame),
            )
            .into()),
        }
    }

    /// Sends the `BYE` command to the SeedLink server.
    #[instrument(skip(self))]
    async fn say_bye(&mut self) -> SeedLinkResult<()> {
        let cmd = CommandV3::Bye(ByeCmdV3);
        let frame = cmd.into_frame();

        debug!("sending command: '{}'", cmd);
        self.write_frame(&frame).await
    }

    #[instrument(skip(self))]
    async fn try_send_info(&mut self, item: InfoCmdItemV3) -> SeedLinkResult<()> {
        if self.expect_info_resp {
            return Err(SeedLinkError::ClientError(
                "multiple concurrent info requests are not allowed".to_string(),
            ));
        }

        let cmd = CommandV3::Info(InfoCmdV3::new(item));
        let frame = cmd.into_frame();

        debug!("sending command: '{}'", cmd);
        self.write_frame(&frame).await
    }
}

/// Enumeration of the possible SeedLink v3 data transfer modes.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum SeedLinkDataTransferModeV3 {
    /// Real-time mode.
    RealTime,
    /// The connection will be closed once all buffered data was transferred.
    DialUp,
    /// Request data in *time window* mode. I.e. data will be requested until the given *end time*.
    TimeWindow(PrimitiveDateTime),
}

// TODO(damb):
// - is it required to maintain both SeedLinkConnectionV3 and FramedConnection. Why not to merge
// the structs?
//
/// Represents an established connection to a SeedLink server.
///
/// Implements SeedLink protocol version <=3.1. Note that at the time being only *multi-station*
/// mode is implemented. Note also that pipelining is supported only if the remote peer implements
/// the batch command mode.
#[derive(Debug)]
pub(crate) struct SeedLinkConnectionV3 {
    con: FramedConnectionV3,
}

impl SeedLinkConnectionV3 {
    pub(crate) fn new(con: ActualConnection) -> Self {
        let con = FramedConnectionV3::new(con);
        Self { con }
    }

    /// Returns a reference to the underlying framed connection.
    pub fn get_framed_connection(&self) -> &FramedConnectionV3 {
        &self.con
    }

    /// Returns a mutable reference to the underlying framed connection.
    pub fn get_framed_connection_mut(&mut self) -> &mut FramedConnectionV3 {
        &mut self.con
    }

    /// Returns whether the connection is open.
    pub fn is_open(&self) -> bool {
        self.con.is_open()
    }

    /// Sends the `HELLO` command to the SeedLink server and returns the raw response.
    #[instrument(skip(self))]
    pub async fn say_hello_raw(&mut self) -> SeedLinkResult<(String, String)> {
        self.con.say_hello().await
    }

    /// Performs a connection shutdown.
    #[instrument(skip(self))]
    pub async fn shutdown(&mut self) -> SeedLinkResult<()> {
        self.con.shutdown().await
    }

    /// Requests the raw id information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_id_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::Id).await
    }

    /// Requests the raw station information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_station_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::Stations).await
    }

    /// Requests the raw stream information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_stream_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::Streams).await
    }

    /// Requests the raw connection information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_connection_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::Connections).await
    }

    /// Requests the raw gap information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_gap_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::Gaps).await
    }

    /// Requests the raw capability information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_capability_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::Capabilities).await
    }

    /// Requests the raw information XML from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_all_info_raw(&mut self) -> SeedLinkResult<String> {
        self.con.request_info(InfoCmdItemV3::All).await
    }

    /// Requests station information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_station_info(&mut self) -> SeedLinkResult<InventoryV3> {
        let resp_xml = self.request_station_info_raw().await?;

        let ret: InventoryV3 = de::from_str::<InventoryV3>(&resp_xml)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid response to INFO command: {}", e.to_string()),
                )
            })?
            .into();

        Ok(ret)
    }

    /// Requests stream information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_stream_info(&mut self) -> SeedLinkResult<InventoryV3> {
        let resp_xml = self.request_stream_info_raw().await?;

        let ret: InventoryV3 = de::from_str::<InventoryV3>(&resp_xml)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid response to INFO command: {}", e.to_string()),
                )
            })?
            .into();

        Ok(ret)
    }

    /// Configures the connection and completes handshaking.
    #[instrument(skip(self))]
    pub async fn configure(
        &mut self,
        stream_configs: &[StreamConfig],
        data_transfer_mode: &SeedLinkDataTransferModeV3,
        batch_cmd_mode: bool,
    ) -> SeedLinkResult<()> {
        self.con
            .configure(stream_configs, data_transfer_mode, batch_cmd_mode)
            .await
    }
}
