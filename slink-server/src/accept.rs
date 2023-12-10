use std::io;
use std::net::SocketAddr;

use crate::client::{self, ClientInfo};
use crate::server::{ServerHandle, ToServer};

use tokio::net::TcpListener;

/// Starts accepting client connections.
pub async fn start_accept(bind: SocketAddr, mut server_handle: ServerHandle) {
    if let Some(err) = accept_loop(bind, server_handle.clone()).await.err() {
        server_handle.send(ToServer::FatalError(err)).await;
    }
}

async fn accept_loop(bind: SocketAddr, server_handle: ServerHandle) -> Result<(), io::Error> {
    let listen = TcpListener::bind(bind).await?;

    loop {
        let (tcp, ip) = listen.accept().await?;

        let id = server_handle.next_id();

        let data = ClientInfo {
            ip,
            id,
            tcp,
            handle: server_handle.clone(),
        };

        client::spawn_client(data);
    }
}

