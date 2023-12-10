use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use futures::stream::StreamExt;
use serde::Serialize;
use socket2::{SockRef, TcpKeepalive};
use tokio::io::AsyncWriteExt;
use tokio::net::{
    tcp::{ReadHalf, WriteHalf},
    TcpStream,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::{select, try_join};
use tokio_util::codec::FramedRead;
use tracing::{error, trace};

use slink::{
    pack_info_err_v4, pack_info_ok_v4, to_first_hello_resp_line_v4, CommandV4, InfoV4,
    ProtocolErrorV4,
};

use crate::negotiate::StationNegotiator;
use crate::response::Hello;
use crate::seedlink::{ParseError, ProtocolVersion, SeedLinkCodec};
use crate::server::{ServerHandle, ToServer};
use crate::Select;
use crate::{ClientId, HIGHEST_SUPPORTED_PROTO_VERSION};

/// Messages received from the main server loop.
pub enum FromServer {
    Hello(Hello),
    Info(InfoV4),
    Ok,
    Error(String),
}

/// A handle to the client actor, used by the server.
#[derive(Debug)]
pub struct ClientHandle {
    pub id: ClientId,
    chan: Sender<FromServer>,
    kill: JoinHandle<()>,

    ip: SocketAddr,

    pub useragent_info: Vec<(String, String)>,
    authenticated: bool,

    pub selects: Vec<Select>,
    pub negotiator: Option<StationNegotiator>,
}

impl ClientHandle {
    /// Returns the socket address of the remote peer.
    pub fn addr(&self) -> &SocketAddr {
        &self.ip
    }

    /// Returns whether the client is authenticated.
    pub fn authenticated(&self) -> bool {
        self.authenticated
    }

    /// Returns whether the client is currently negotiating.
    pub fn is_negotiating(&self) -> bool {
        self.negotiator.is_some()
    }

    /// Sends a message to this client actor.
    ///
    /// Will emit an error if sending does not succeed immediately, as this means that forwarding
    /// messages to the underlying TCP connection cannot keep up.
    pub fn send(&mut self, msg: FromServer) -> Result<(), io::Error> {
        self.chan
            .try_send(msg)
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))
    }

    /// Kill the underlying actor.
    pub fn kill(self) {
        // run the destructor
        drop(self);
    }
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.kill.abort()
    }
}

/// Struct constructed by the accept loop and used as the argument to `spawn_client`.
pub struct ClientInfo {
    pub ip: SocketAddr,
    pub id: ClientId,
    pub handle: ServerHandle,
    pub tcp: TcpStream,
}

/// Struct storing the information used internally by the client actor.
struct ClientData {
    id: ClientId,
    handle: ServerHandle,
    recv: Receiver<FromServer>,
    tcp: TcpStream,
}

/// Spawns a new client actor.
pub fn spawn_client(info: ClientInfo) {
    let (send, recv) = channel(64);

    let data = ClientData {
        id: info.id,
        handle: info.handle.clone(),
        tcp: info.tcp,
        recv,
    };

    // XXX(damb): spawn client actor task
    let (my_send, my_recv) = oneshot::channel();
    let client_join_handle = tokio::spawn(start_client(my_recv, data));

    // Then we create a ClientHandle to this new task, and use the oneshot
    // channel to send it to the task.
    let client_handle = ClientHandle {
        id: info.id,
        chan: send,
        kill: client_join_handle,

        ip: info.ip,
        useragent_info: Vec::default(),
        authenticated: false,
        selects: vec![],
        negotiator: None,
    };

    // Ignore sending errors here. Should only happen if the server is shutting
    // sdown.
    let _ = my_send.send(client_handle);
}

async fn start_client(my_handle: oneshot::Receiver<ClientHandle>, mut data: ClientData) {
    // Wait for `client_handle` to send us the `ClientHandle` so we can forward
    // it to the main loop. We need the oneshot channel because we cannot
    // otherwise get the `JoinHandle` returned by `tokio::spawn`. We forward it
    // from here instead of in `spawn_client` because we want the server to see
    // the NewClient message before this actor starts sending other messages.
    let client_handle = match my_handle.await {
        Ok(client_handle) => client_handle,
        Err(_) => return,
    };
    let client_id = client_handle.id.clone();
    data.handle.send(ToServer::NewClient(client_handle)).await;

    let mut server_handle = data.handle.clone();

    // We sent the client handle to the main server loop. Start talking to the tcp
    // connection.
    let res = client_loop(data).await;
    match res {
        Ok(()) => {}
        Err(err) => {
            error!("Error while shutting down client loop: {}.", err);
        }
    };

    // Inform server about the client loop termination.
    server_handle
        .send(ToServer::DisconnectClient(client_id))
        .await;
    println!("shutdown");
}

/// This method performs the actual job of running the client actor.
async fn client_loop(mut client_data: ClientData) -> Result<(), io::Error> {
    let sock_ref = SockRef::from(&client_data.tcp);

    let tcp_keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(60))
        .with_interval(Duration::from_secs(20));

    sock_ref.set_tcp_keepalive(&tcp_keepalive)?;

    let (read, write) = client_data.tcp.split();

    // direct communication between tcp_read and tcp_write
    let (send, recv) = unbounded_channel();

    let ((), ()) = try_join! {
        tcp_read(client_data.id, read, client_data.handle, send),
        tcp_write(client_data.id, write, client_data.recv, recv),
    }?;

    let _ = client_data.tcp.shutdown().await;

    Ok(())
}

#[derive(Debug)]
enum InternalMessage {
    ProtocolError(ProtocolErrorV4),
}

async fn tcp_read(
    client_id: ClientId,
    read: ReadHalf<'_>,
    mut server_handle: ServerHandle,
    to_tcp_write: UnboundedSender<InternalMessage>,
) -> Result<(), io::Error> {
    let mut framed_read = FramedRead::new(read, SeedLinkCodec::new(client_id));
    let mut next_cmd = framed_read.next().await;
    while let Some(ref res) = next_cmd {
        trace!("{:?}: <- {:?} ", client_id, res);
        match res {
            Ok(cmd_v4) => {
                // handle protocol version request
                if let CommandV4::SlProto(slproto) = cmd_v4 {
                    let res = framed_read
                        .decoder_mut()
                        .try_set_protocol_version((slproto.major, slproto.minor).into());
                    match res {
                        Ok(_) => {}
                        Err(err) => {
                            to_tcp_write
                                .send(InternalMessage::ProtocolError(err))
                                .map_err(|e| {
                                    io::Error::new(io::ErrorKind::BrokenPipe, e.to_string())
                                })?;
                        }
                    };

                    continue;
                } else {
                    match cmd_v4 {
                        CommandV4::Hello(_) => {
                            // do nothing, ignore
                        }
                        _ => {
                            framed_read.decoder_mut().lock_protocol_version();
                        }
                    }
                }

                if let CommandV4::Unknown(cmd) = cmd_v4 {
                    let mut unsupported_err = ProtocolErrorV4::unsupported_command();
                    unsupported_err.message = Some(
                        format!(
                            "{}: '{}'",
                            unsupported_err.code.description(),
                            cmd.command_name
                        )
                        .into(),
                    );

                    to_tcp_write
                        .send(InternalMessage::ProtocolError(unsupported_err))
                        .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))?;
                } else {
                    server_handle
                        .send(ToServer::Command(client_id, cmd_v4.clone()))
                        .await;
                }
            }
            Err(err) => {
                // XXX(damb): usually errors inform `FramedRead` that the stream is corrupted and
                // should be terminated. I.e. subsequent calls to `framed_read.next().await` return
                // `None` such that the loop will terminate.
                match err {
                    ParseError::IoError(_) => {
                        break;
                    }
                    ParseError::CommandLineTooLong => {
                        send_generic_error(framed_read.decoder().protocol_version(), &to_tcp_write);
                        // XXX(damb): do not recover from this error
                        break;
                    }
                    ParseError::ProtocolError(err) => {
                        if err.info {
                            // XXX(damb): `INFO` command errors require special treatment and are
                            // returned as a SeedLink error info packet.
                            server_handle
                                .send(ToServer::ErrorInfo(client_id, err.clone()))
                                .await;
                        } else {
                            to_tcp_write
                                .send(InternalMessage::ProtocolError(err.clone()))
                                .map_err(|e| {
                                    io::Error::new(io::ErrorKind::BrokenPipe, e.to_string())
                                })?;
                        }

                        // XXX(damb): resume the stream and don't disconnect the client
                        let _ = framed_read.next().await;
                    }
                };
            }
        };

        next_cmd = framed_read.next().await;
    }

    Ok(())
}

// TODO(damb): implement encoder which allows versionized response encoding
async fn tcp_write(
    client_id: ClientId,
    mut write: WriteHalf<'_>,
    mut recv: Receiver<FromServer>,
    mut from_tcp_read: UnboundedReceiver<InternalMessage>,
) -> Result<(), io::Error> {
    loop {
        select! {
            msg = recv.recv() => match msg {
                Some(FromServer::Hello(msg)) => {
                    trace!("{:?}: -> {:?}", client_id, msg);
            let msg = format!("{first_resp_line}\r\n{dc_desc}\r\n", first_resp_line = to_first_hello_resp_line_v4(&msg.implementation, &msg.implementation_version, &vec![(HIGHEST_SUPPORTED_PROTO_VERSION.0, HIGHEST_SUPPORTED_PROTO_VERSION.1)], &None), dc_desc = msg.data_center_description);

                    write.write_all(msg.as_bytes()).await?;
                },
                Some(FromServer::Info(info_v4)) => {
                    trace!("{:?}: -> {:?}", client_id, info_v4);
                    let serialized = match info_v4 {
                        InfoV4::Id(ref id_info) => to_json(id_info)?,
                        InfoV4::Formats(ref formats_info) => to_json(formats_info)?,
                        InfoV4::Capabilities(ref capabilities_info) => to_json(capabilities_info)?,
                        InfoV4::Stations(ref stations_info) => to_json(stations_info)?,
                        InfoV4::Streams(ref streams_info) => to_json(streams_info)?,
                        InfoV4::Connections(ref connections_info) => to_json(connections_info)?,
                        InfoV4::Error(ref error_info) => to_json(error_info)?,
                    };

                    let packet = match info_v4 {
                        InfoV4::Error(_) =>
                        pack_info_err_v4(&serialized).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?,
                        _ =>
                        pack_info_ok_v4(&serialized).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?,
                    };

                    write.write_all(&packet).await?;
                },
                Some(FromServer::Ok) => {
                    trace!("{:?}: -> OK", client_id);
                    write.write_all("OK\r\n".as_bytes()).await?

                }
                Some(FromServer::Error(msg)) => {
                    trace!("{:?}: -> {:?}", client_id, msg);
                    write.write_all(msg.as_bytes()).await?;
                    write.write_all(&[b'\r', b'\n']).await?
                }
                None => {
                    break;
                },
            },
            msg = from_tcp_read.recv() => match msg {
                Some(InternalMessage::ProtocolError(err)) => {
                    trace!("{:?}: -> {:?}", client_id, err);
                    write.write_all(err.to_string().as_bytes()).await?;
                    write.write_all(&[b'\r', b'\n']).await?
                },
                None => {
                    break;
                }
            }
        };
    }

    Ok(())
}

fn send_generic_error(
    protocol_version: &ProtocolVersion,
    to_tcp_write: &UnboundedSender<InternalMessage>,
) {
    let msg = match protocol_version.major {
        4 => InternalMessage::ProtocolError(ProtocolErrorV4::generic()),
        _ => {
            todo!();
        }
    };

    to_tcp_write.send(msg).unwrap();
}

fn to_json(obj: &impl Serialize) -> Result<String, io::Error> {
    serde_json::to_string(obj)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}
