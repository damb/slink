use std::collections::HashMap;
use std::io;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{debug, error};

use slink::{CommandV4, ErrorInfoV4, InfoV4, ProtocolErrorV4};

use crate::client::{ClientHandle, FromServer};
use crate::dispatch::Dispatcher;
use crate::util::to_id_info_v4;
use crate::HIGHEST_SUPPORTED_PROTO_VERSION;
use crate::{ClientId, SeedLinkServer};

#[derive(Clone, Debug)]
pub struct ServerHandle {
    chan: Sender<ToServer>,
    next_id: Arc<AtomicUsize>,
}

impl ServerHandle {
    pub async fn send(&mut self, msg: ToServer) {
        if self.chan.send(msg).await.is_err() {
            panic!("Main loop has shut down.");
        }
    }

    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}

/// The message type used when a client actor sends messages to the main server loop.
pub enum ToServer {
    NewClient(ClientHandle),
    DisconnectClient(ClientId),
    Command(ClientId, CommandV4),
    ErrorInfo(ClientId, ProtocolErrorV4),
    FatalError(io::Error),
}

/// Spawns the main server loop.
pub fn spawn_main_loop<T>(service: T) -> (ServerHandle, JoinHandle<()>)
where
    T: SeedLinkServer,
{
    let (send, recv) = channel(64);

    let server_handle = ServerHandle {
        chan: send,
        next_id: Default::default(),
    };

    let server_join_handle = tokio::spawn(async move {
        let res = main_loop(service, recv).await;
        match res {
            Ok(()) => {}
            Err(err) => {
                // TODO(damb): handle error approriately
                error!("Failed to spawn main server loop: {}.", err);
            }
        }
    });

    (server_handle, server_join_handle)
}

/// Struct storing the information used internally by the main server loop.
#[derive(Default, Debug)]
struct ServerData<T> {
    clients: HashMap<ClientId, ClientHandle>,

    router: Dispatcher<T>,
}

impl<T: SeedLinkServer> ServerData<T> {
    /// Adds a client.
    fn add_client(&mut self, client_handle: ClientHandle) {
        let client_id = client_handle.id;
        self.clients.insert(client_id.clone(), client_handle);
    }

    /// Removes a client.
    fn remove_client(&mut self, client_id: &ClientId) -> Option<ClientHandle> {
        self.clients.remove(client_id)
    }

    fn log_remove_client(&mut self, client_id: &ClientId) {
        if let Some(client_handle) = self.remove_client(&client_id) {
            debug!(
                "{:?}: disconnected client (ip={})",
                client_handle.id,
                client_handle.addr()
            );
        }
    }
}

async fn main_loop<T>(mut service: T, mut recv: Receiver<ToServer>) -> Result<(), io::Error>
where
    T: SeedLinkServer,
{
    let mut data = ServerData {
        clients: HashMap::default(),
        router: Dispatcher::new(service),
    };

    while let Some(msg) = recv.recv().await {
        match msg {
            ToServer::NewClient(client_handle) => {
                debug!(
                    "{:?}: new client connection (ip={})",
                    client_handle.id,
                    client_handle.addr()
                );
                data.add_client(client_handle);
            }
            ToServer::Command(client_id, cmd) => {
                let mut disconnect = false;
                if let Some(client_handle) = data.clients.get_mut(&client_id) {
                    match cmd {
                        CommandV4::Bye(_) => {
                            disconnect = true;
                        }
                        CommandV4::UserAgent(inner_cmd) => {
                            client_handle.useragent_info = inner_cmd
                                .info
                                .into_iter()
                                .map(|info| (info.program_or_library, info.version))
                                .collect();

                            if let Err(_) = client_handle.send(FromServer::Ok) {
                                data.log_remove_client(&client_id);
                            }
                        }
                        _ => {
                            if let Err(_) = data.router.dispatch(&cmd, client_handle).await {
                                disconnect = true;
                            }
                        }
                    }
                }

                if disconnect {
                    data.log_remove_client(&client_id);
                }
            }
            ToServer::ErrorInfo(client_id, err) => {
                if let Some(client_handle) = data.clients.get_mut(&client_id) {
                    let error_info = ErrorInfoV4 {
                        id: to_id_info_v4(
                            data.router.server(),
                            &vec![(
                                HIGHEST_SUPPORTED_PROTO_VERSION.0,
                                HIGHEST_SUPPORTED_PROTO_VERSION.1,
                            )],
                            &None,
                        ),
                        error: err,
                    };

                    if let Err(_) = client_handle.send(FromServer::Info(InfoV4::Error(error_info)))
                    {
                        data.log_remove_client(&client_id);
                    }
                }
            }
            ToServer::DisconnectClient(client_id) => {
                data.log_remove_client(&client_id);
            }
            ToServer::FatalError(err) => return Err(err),
        }
        println!("Number of clients: {}", data.clients.len());
    }

    Ok(())
}
