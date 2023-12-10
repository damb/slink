use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, Stream, StreamExt, TryStream};
use time::PrimitiveDateTime;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time as tokio_time;
use tokio_stream::wrappers::IntervalStream;
use tracing::{debug, info, instrument, warn};

use crate::{
    util, Frame, Inventory, SeedLinkConnectionV3, SeedLinkDataTransferModeV3,
    SeedLinkError, SeedLinkGenericDataPacketV3, SeedLinkInfoPacketV3, SeedLinkPacket,
    SeedLinkPacketV3, SeedLinkResult, StateDB, StreamConfig, AVAILABLE_CLIENT_PROTO_VERSIONS,
    DEFAULT_PORT,
};

#[derive(Debug)]
pub(crate) struct TcpConnection {
    pub rw: TcpStream,
    pub open: bool,
}

/// Enumerations of actual raw connections.
#[derive(Debug)]
pub(crate) enum ActualConnection {
    Tcp(TcpConnection),
}

impl ActualConnection {
    pub async fn new(addr: &ConnectionAddr, timeout: Option<Duration>) -> SeedLinkResult<Self> {
        Ok(match *addr {
            ConnectionAddr::Tcp(ref host, ref port) => {
                let addr = (host.as_str(), *port);
                if let Some(timeout) = timeout {
                    let socket = tokio_time::timeout(timeout, TcpStream::connect(addr))
                        .await
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "connection timeout")
                        })??;

                    Self::Tcp(TcpConnection {
                        rw: socket,
                        open: true,
                    })
                } else {
                    let socket = TcpStream::connect(addr).await?;
                    Self::Tcp(TcpConnection {
                        rw: socket,
                        open: true,
                    })
                }
            }
        })
    }
}

#[derive(Debug)]
pub(crate) enum ActualSeedLinkConnection {
    V3(SeedLinkConnectionV3),
    // V4(),
}

/// Enumeration of possible data transfer modes.
#[derive(Debug)]
pub enum DataTransferMode {
    /// Real-time mode.
    RealTime,
    /// The connection will be closed once all buffered data was transferred.
    DialUp,
}

#[derive(Debug, Clone, Default)]
struct StreamConfigs(pub HashMap<String, StreamConfig>);

impl StreamConfigs {
    pub fn add_stream(
        &mut self,
        net: &str,
        sta: &str,
        select_arg: &Option<String>,
        seq_num: &Option<String>,
        time: &Option<PrimitiveDateTime>,
    ) -> SeedLinkResult<()> {
        let mut key = net.to_string();
        key.push_str(sta);

        if let Some(stream_config) = self.0.get_mut(&key) {
            if let Some(select_arg) = select_arg {
                stream_config.add_select_arg(select_arg);
            }
        } else {
            self.0.insert(
                key,
                StreamConfig::new(net, sta, select_arg.clone(), seq_num.clone(), time.clone()),
            );
        }

        Ok(())
    }

    pub fn seq_num(&self, net: &str, sta: &str) -> Option<&str> {
        let key = format!("{}{}", net, sta);

        if let Some(stream_config) = self.0.get(&key) {
            if let Some(seq_num) = &stream_config.seq_num {
                return Some(seq_num);
            }
        }

        None
    }
}

// TODO(damb):
// - Provide additional member functions
//
/// Represents a stateful SeedLink connection.
#[derive(Debug)]
pub struct Connection {
    /// The actual underlying SeedLink connection handle.
    con: ActualSeedLinkConnection,

    stream_configs: StreamConfigs,
}

impl Connection {
    pub(crate) fn new(con: ActualSeedLinkConnection) -> Self {
        Self {
            con,
            stream_configs: StreamConfigs::default(),
        }
    }

    /// Returns the SeedLink protocol version used.
    pub fn protocol_version(&self) -> u8 {
        match self.con {
            ActualSeedLinkConnection::V3(_) => 3,
        }
    }

    /// Returns whether the connection is open.
    pub fn is_open(&self) -> bool {
        match &self.con {
            ActualSeedLinkConnection::V3(con) => con.is_open(),
        }
    }

    /// Configures the connection with the provided stream specific data.
    pub fn add_stream(
        &mut self,
        net: &str,
        sta: &str,
        select_arg: &Option<String>,
        seq_num: &Option<String>,
        time: &Option<PrimitiveDateTime>,
    ) -> SeedLinkResult<()> {
        self.stream_configs
            .add_stream(net, sta, select_arg, seq_num, time)
    }

    /// Recovers the `StateDB` and updates the streams previously added by `Connection::add_stream`.
    pub async fn recover_state(
        &mut self,
        db: &mut StateDB,
        add_select_args: bool,
    ) -> SeedLinkResult<()> {
        let protocol_version = self.protocol_version();

        for (sid, seq_num) in db.state().await? {
            if let Some(stream_config) = self
                .stream_configs
                .0
                .get_mut(&format!("{}{}", sid.nslc.net, sid.nslc.sta))
            {
                if add_select_args {
                    if protocol_version == 3 {
                        stream_config.add_select_arg(&util::get_select_arg_v3(&sid));
                    }
                }

                let seq_num = format!("{:x}", seq_num);
                if let Some(prev_seq_num) = &stream_config.seq_num {
                    if &seq_num < prev_seq_num {
                        continue;
                    }
                }
                stream_config.seq_num.replace(seq_num);
            }
        }

        Ok(())
    }

    /// Directly configures the connection from a `StateDB` and completes handshaking.
    #[instrument(skip(self))]
    pub async fn configure_from_state_db(
        &mut self,
        db: &mut StateDB,
        data_transfer_mode: DataTransferMode,
        pipelining: bool,
    ) -> SeedLinkResult<()> {
        let protocol_version = self.protocol_version();

        let mut stream_configs = StreamConfigs::default();
        for (sid, seq_num) in db.state().await? {
            let seq_num = {
                let seq_num = format!("{:x}", seq_num);
                if let Some(prev_seq_num) = stream_configs.seq_num(&sid.nslc.net, &sid.nslc.sta) {
                    if seq_num.as_str() < prev_seq_num {
                        None
                    } else {
                        Some(seq_num)
                    }
                } else {
                    Some(seq_num)
                }
            };

            let select_arg = {
                if protocol_version == 3 {
                    Some(util::get_select_arg_v3(&sid))
                } else {
                    None
                }
            };

            stream_configs.add_stream(
                &sid.nslc.net,
                &sid.nslc.sta,
                &select_arg,
                &seq_num,
                &None,
            )?;
        }

        let stream_configs: Vec<StreamConfig> = self.stream_configs.0.values().cloned().collect();

        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => {
                let v3_data_transfer_mode = match data_transfer_mode {
                    DataTransferMode::RealTime => SeedLinkDataTransferModeV3::RealTime,
                    DataTransferMode::DialUp => SeedLinkDataTransferModeV3::DialUp,
                };

                con.configure(&stream_configs, &v3_data_transfer_mode, pipelining)
                    .await
            }
        }
    }

    /// Configures the connection and completes handshaking.
    #[instrument(skip(self))]
    pub async fn configure(
        &mut self,
        data_transfer_mode: DataTransferMode,
        end_time: Option<PrimitiveDateTime>,
        pipelining: bool,
    ) -> SeedLinkResult<()> {
        let stream_configs: Vec<StreamConfig> = self.stream_configs.0.values().cloned().collect();

        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => {
                let v3_data_transfer_mode;
                if let Some(end_time) = end_time {
                    v3_data_transfer_mode = SeedLinkDataTransferModeV3::TimeWindow(end_time);
                } else {
                    v3_data_transfer_mode = match data_transfer_mode {
                        DataTransferMode::RealTime => SeedLinkDataTransferModeV3::RealTime,
                        DataTransferMode::DialUp => SeedLinkDataTransferModeV3::DialUp,
                    };
                }
                con.configure(&stream_configs, &v3_data_transfer_mode, pipelining)
                    .await
            }
        }
    }

    /// Greets the SeedLink server and returns the raw response.
    #[instrument(skip(self))]
    pub async fn greet_raw(&mut self) -> SeedLinkResult<Vec<String>> {
        let rv: Vec<String>;

        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => {
                let (first_resp_line, second_resp_line) = con.say_hello_raw().await?;
                rv = vec![first_resp_line, second_resp_line];
            }
        }

        Ok(rv)
    }

    /// Requests raw id information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_id_info_raw(&mut self) -> SeedLinkResult<String> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => con.request_id_info_raw().await,
        }
    }

    /// Requests raw station information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_station_info_raw(&mut self) -> SeedLinkResult<String> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => con.request_station_info_raw().await,
        }
    }

    /// Requests raw stream information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_stream_info_raw(&mut self) -> SeedLinkResult<String> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => con.request_stream_info_raw().await,
        }
    }

    /// Requests raw connection information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_connection_info_raw(&mut self) -> SeedLinkResult<String> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => con.request_connection_info_raw().await,
        }
    }

    /// Requests stream information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_station_info(&mut self) -> SeedLinkResult<Inventory> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => {
                con.request_station_info().await.map(|inv_v3| inv_v3.into())
            }
        }
    }

    /// Requests stream information from the SeedLink server.
    #[instrument(skip(self))]
    pub async fn request_stream_info(&mut self) -> SeedLinkResult<Inventory> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => {
                con.request_stream_info().await.map(|inv_v3| inv_v3.into())
            }
        }
    }

    // TODO(damb): provide an example (i.e. code snippet)
    /// Returns a stream producing SeedLink version dependent packets asynchronously.
    ///
    /// If `keep_alive_interval` is not `None` the stream sents keepalive packets to the remote
    /// peer SeedLink server backed by the specified `Duration`. Panics if the `Duration` is zero.
    ///
    /// Note that keepalive packets are returned, too.
    /// ```
    pub fn packets(
        self,
        keep_alive_interval: Option<Duration>,
    ) -> impl TryStream<Item = SeedLinkResult<SeedLinkPacket>> {
        let keep_alive_stream: Arc<Mutex<Pin<Box<dyn Stream<Item = tokio_time::Instant>>>>>;
        if let Some(duration) = keep_alive_interval {
            assert!(
                !duration.is_zero(),
                "keep_alive_interval must be greater than zero"
            );
            let interval = tokio_time::interval(duration);
            keep_alive_stream = Arc::new(Mutex::new(Box::pin(IntervalStream::new(interval))));
        } else {
            keep_alive_stream = Arc::new(Mutex::new(Box::pin(stream::pending::<
                tokio_time::Instant,
            >())));
        }

        let inner_con = match self.con {
            ActualSeedLinkConnection::V3(con) => con,
        };
        let inner_con = Arc::new(Mutex::new(inner_con));

        stream::try_unfold((), move |_| {
            let cloned_inner_con = inner_con.clone();
            let cloned_keep_alive = keep_alive_stream.clone();
            async move {
                loop {
                    let mut inner_con = cloned_inner_con.lock().await;
                    let mut keep_alive = cloned_keep_alive.lock().await;
                    tokio::select! {
                        frame = inner_con.get_framed_connection_mut().read_frame() => match frame? {
                            Frame::GenericDataPacket(buf) => {
                                return Ok(Some((SeedLinkPacket::V3(SeedLinkPacketV3::GenericData(SeedLinkGenericDataPacketV3::new(buf))), ())));
                            }
                            Frame::InfoPacket(buf) => {
                                inner_con.get_framed_connection_mut().ack_keep_alive();
                                return Ok(Some((SeedLinkPacket::V3(SeedLinkPacketV3::Info(SeedLinkInfoPacketV3::new(buf))), ())));
                            }
                            Frame::End => {
                                inner_con.shutdown().await?;
                                return Ok(None)
                            },
                            frame => {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("unexpected frame received: {:?}", frame),
                                )
                                .into());
                            }
                        },
                        _  = keep_alive.next() => {
                            inner_con.get_framed_connection_mut().try_send_keep_alive().await?;
                        },
                    }
                }
            }
        })
    }

    pub async fn shutdown(&mut self) -> SeedLinkResult<()> {
        match &mut self.con {
            ActualSeedLinkConnection::V3(con) => con.shutdown().await,
        }
    }
}

/// This function takes a SeedLink URL string and parses it into a URL
/// as used by rust-url. This is necessary as the default parser does
/// not understand how SeedLink URLs function.
pub fn parse_slink_url(input: &str) -> Option<url::Url> {
    match url::Url::parse(input) {
        Ok(result) => match result.scheme() {
            "slink" | "slinkv3" => Some(result),
            _ => None,
        },
        Err(_) => None,
    }
}

/// Defines the connection address.
#[derive(Clone, Debug)]
pub enum ConnectionAddr {
    /// Format for this is `(host, port)`.
    Tcp(String, u16),
    ///// Format for this is `(host, port)`.
    //TcpTls {
    //    /// Hostname
    //    host: String,
    //    /// Port
    //    port: u16,
    //    /// Disable hostname verification when connecting.
    //    ///
    //    /// # Warning
    //    ///
    //    /// You should think very carefully before you use this method. If hostname
    //    /// verification is not used, any valid certificate for any site will be
    //    /// trusted for use from any other. This introduces a significant
    //    /// vulnerability to man-in-the-middle attacks.
    //    insecure: bool,
    //},
    ///// Format for this is the path to the unix socket.
    //Unix(PathBuf),
}

impl fmt::Display for ConnectionAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Cluster::get_connection_info depends on the return value from this function
        match *self {
            ConnectionAddr::Tcp(ref host, port) => write!(f, "{host}:{port}"),
            // ConnectionAddr::TcpTls { ref host, port, .. } => write!(f, "{host}:{port}"),
            // ConnectionAddr::Unix(ref path) => write!(f, "{}", path.display()),
        }
    }
}

/// Holds the connection information that SeedLink should use for connecting.
#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    /// A connection address for where to connect to.
    pub addr: ConnectionAddr,

    /// SeedLink specific connection information.
    pub slink: SeedLinkConnectionInfo,
}

/// SeedLink specific/connection independent information used to establish a connection to redis.
#[derive(Clone, Debug, Default)]
pub struct SeedLinkConnectionInfo {
    /// The SeedLink protocol to be used.
    pub protocol_version: Option<u8>,
    /// Optionally a username that should be used for connection.
    pub username: Option<String>,
    /// Optionally a password that should be used for connection.
    pub password: Option<String>,
}

impl FromStr for ConnectionInfo {
    type Err = SeedLinkError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        s.into_connection_info()
    }
}

/// Converts an object into a connection info struct. This allows the
/// constructor of the client to accept connection information in a
/// range of different formats.
pub trait IntoConnectionInfo {
    /// Converts the object into a connection info object.
    fn into_connection_info(self) -> SeedLinkResult<ConnectionInfo>;
}

impl IntoConnectionInfo for ConnectionInfo {
    fn into_connection_info(self) -> SeedLinkResult<ConnectionInfo> {
        Ok(self)
    }
}

impl<'a> IntoConnectionInfo for &'a str {
    fn into_connection_info(self) -> SeedLinkResult<ConnectionInfo> {
        match parse_slink_url(self) {
            Some(u) => u.into_connection_info(),
            None => Err(SeedLinkError::InvalidClientConfig(
                "SeedLink URL did not parse".to_string(),
            )),
        }
    }
}

impl<T> IntoConnectionInfo for (T, u16)
where
    T: Into<String>,
{
    fn into_connection_info(self) -> SeedLinkResult<ConnectionInfo> {
        Ok(ConnectionInfo {
            addr: ConnectionAddr::Tcp(self.0.into(), self.1),
            slink: SeedLinkConnectionInfo::default(),
        })
    }
}

impl IntoConnectionInfo for String {
    fn into_connection_info(self) -> SeedLinkResult<ConnectionInfo> {
        match parse_slink_url(&self) {
            Some(u) => u.into_connection_info(),
            None => Err(SeedLinkError::InvalidClientConfig(
                "SeedLink URL did not parse".to_string(),
            )),
        }
    }
}

fn url_to_tcp_connection_info(url: url::Url) -> SeedLinkResult<ConnectionInfo> {
    let host = match url.host() {
        Some(host) => {
            // Here we manually match host's enum arms and call their to_string().
            // Because url.host().to_string() will add `[` and `]` for ipv6:
            // https://docs.rs/url/latest/src/url/host.rs.html#170
            // And these brackets will break host.parse::<Ipv6Addr>() when
            // `client.open()` - `ActualConnection::new()` - `addr.to_socket_addrs()`:
            // https://doc.rust-lang.org/src/std/net/addr.rs.html#963
            // https://doc.rust-lang.org/src/std/net/parser.rs.html#158
            // IpAddr string with brackets can ONLY parse to SocketAddrV6:
            // https://doc.rust-lang.org/src/std/net/parser.rs.html#255
            // But if we call Ipv6Addr.to_string directly, it follows rfc5952 without brackets:
            // https://doc.rust-lang.org/src/std/net/ip.rs.html#1755
            match host {
                url::Host::Domain(path) => path.to_string(),
                url::Host::Ipv4(v4) => v4.to_string(),
                url::Host::Ipv6(v6) => v6.to_string(),
            }
        }
        None => {
            return Err(SeedLinkError::InvalidClientConfig(
                "Missing hostname".to_string(),
            ));
        }
    };

    let port = url.port().unwrap_or(DEFAULT_PORT);

    let addr = ConnectionAddr::Tcp(host, port);

    Ok(ConnectionInfo {
        addr,
        slink: SeedLinkConnectionInfo {
            protocol_version: if url.scheme() == "slinkv3" {
                Some(3)
            } else {
                None
            },
            username: if url.username().is_empty() {
                None
            } else {
                match percent_encoding::percent_decode(url.username().as_bytes()).decode_utf8() {
                    Ok(decoded) => Some(decoded.into_owned()),
                    Err(_) => {
                        return Err(SeedLinkError::InvalidClientConfig(
                            "Username is not a valid UTF-8 string".to_string(),
                        ));
                    }
                }
            },
            password: match url.password() {
                Some(pw) => match percent_encoding::percent_decode(pw.as_bytes()).decode_utf8() {
                    Ok(decoded) => Some(decoded.into_owned()),
                    Err(_) => {
                        return Err(SeedLinkError::InvalidClientConfig(
                            "Password is not a valid UTF-8 string".to_string(),
                        ));
                    }
                },
                None => None,
            },
        },
    })
}

impl IntoConnectionInfo for url::Url {
    fn into_connection_info(self) -> SeedLinkResult<ConnectionInfo> {
        match self.scheme() {
            "slink" | "slinkv3" => url_to_tcp_connection_info(self),
            _ => Err(SeedLinkError::InvalidClientConfig(
                "URL provided is not a SeedLink URL".to_string(),
            )),
        }
    }
}

pub async fn connect(
    connection_info: &ConnectionInfo,
    timeout: Option<Duration>,
) -> SeedLinkResult<Connection> {
    let con = ActualConnection::new(&connection_info.addr, timeout).await?;
    setup_connection(con, &connection_info.slink).await
}

async fn make_preflight_request(
    con: &mut ActualConnection,
) -> SeedLinkResult<util::ParsedHelloResponse> {
    let mut buf = Vec::new();

    debug!("[preflight request] sending command: 'hello'");
    match con {
        ActualConnection::Tcp(TcpConnection { ref mut rw, .. }) => {
            rw.write_all(b"hello\r\n").await?;
            rw.flush().await?;

            // read 'HELLO' respose (two lines)
            read_line(rw, &mut buf).await?;
            read_line(rw, &mut buf).await?;
        }
    };

    let buf = String::from_utf8(buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let mut lines = buf.lines();
    let first_resp_line: &str;
    if let Some(line) = lines.next() {
        first_resp_line = line;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid server response to 'HELLO': missing response line",
        )
        .into());
    }

    let second_resp_line: String;
    if let Some(line) = lines.next() {
        second_resp_line = line.to_string();
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid server response to 'HELLO': missing response line",
        )
        .into());
    }

    if lines.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid server response to 'HELLO': too many response lines",
        )
        .into());
    }

    let rv = util::parse_hello_response(first_resp_line, second_resp_line)?;

    info!("[preflight request] connected to: {}", first_resp_line);
    debug!(
        "[preflight request] seedlink protocol version(s): {:?}",
        rv.protocol_versions
    );
    if !rv.station_or_datacenter_desc.is_empty() {
        debug!(
            "[preflight request] station/datacenter description: {}",
            rv.station_or_datacenter_desc
        );
    } else {
        warn!("[preflight request] missing station or datacenter description");
    }

    Ok(rv)
}

async fn read_line<R: AsyncRead + Unpin>(read: &mut R, buf: &mut Vec<u8>) -> SeedLinkResult<()> {
    loop {
        let byte = read.read_u8().await?;
        buf.push(byte);
        if byte == 10 {
            break;
        }
    }

    Ok(())
}

async fn setup_connection(
    mut con: ActualConnection,
    slink_connection_info: &SeedLinkConnectionInfo,
) -> SeedLinkResult<Connection> {
    let hello_resp = make_preflight_request(&mut con).await?;

    let mut major_proto_versions = HashSet::new();
    for proto_version_str in &hello_resp.protocol_versions {
        if let Some(major_proto_version) = proto_version_str.splitn(2, '.').next() {
            let parsed_major_proto_version = major_proto_version.parse::<u8>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "failed to parse seedlink protocol version: {}",
                        major_proto_version
                    ),
                )
            })?;

            major_proto_versions.insert(parsed_major_proto_version);
        }
    }

    let mut selected_proto_version: Option<u8> = None;
    if let Some(proto_version) = slink_connection_info.protocol_version {
        if major_proto_versions.get(&proto_version).is_none() {
            return Err(SeedLinkError::ClientError("incompatible seedlink protocol versions: protocol version not implemented by remote peer".to_string()));
        }

        selected_proto_version = Some(proto_version);
    }

    // try most recent protocol version implemented by both the library and the remote peer
    for avail_proto_version in AVAILABLE_CLIENT_PROTO_VERSIONS.into_iter().rev() {
        if major_proto_versions.get(&avail_proto_version).is_none() {
            continue;
        }
        selected_proto_version = Some(avail_proto_version);
    }

    let con = match selected_proto_version {
        Some(v) => {
            debug!("using seedlink protocol version: v{}", v);
            if v == 3 {
                ActualSeedLinkConnection::V3(SeedLinkConnectionV3::new(con))
            } else {
                return Err(SeedLinkError::ClientError(
                    "incompatible seedlink protocol versions".to_string(),
                ));
            }
        }
        None => {
            return Err(SeedLinkError::ClientError(
                "incompatible seedlink protocol versions".to_string(),
            ));
        }
    };

    let rv = Connection::new(con);

    // TODO(damb):
    // - perform authentication

    // if connection_info.password.is_some() {
    //     connect_auth(&mut rv, connection_info)?;
    // }

    // if connection_info.db != 0 {
    //     match cmd("SELECT")
    //         .arg(connection_info.db)
    //         .query::<Value>(&mut rv)
    //     {
    //         Ok(Value::Okay) => {}
    //         _ => fail!((
    //             ErrorKind::ResponseError,
    //             "Redis server refused to switch database"
    //         )),
    //     }
    // }

    Ok(rv)
}
