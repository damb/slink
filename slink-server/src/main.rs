use std::collections::HashMap;

use tracing::info;
use tracing_subscriber;

use slink::Station;
use slink_server::{ClientId, SeedLinkServer};

use slink::DEFAULT_PORT;

// TODO(damb): client specific data required for streaming
#[derive(Clone, Debug, Default)]
struct Client;

#[derive(Debug, Default)]
struct SeedLinkServerBackend {
    clients: HashMap<ClientId, Client>,
}

#[slink_server::async_trait]
impl SeedLinkServer for SeedLinkServerBackend {
    fn implementation(&self) -> &str {
        "NeedLink"
    }

    fn implementation_version(&self) -> &str {
        "0.1"
    }

    fn data_center_description(&self) -> &str {
        "FOO DC"
    }

    async fn inventory_stations(
        &self,
        station_pattern: &str,
        stream_pattern: Option<String>,
        format_subformat_pattern: Option<String>,
    ) -> &Vec<Station> {
        todo!()
    }

    async fn inventory_streams(
        &self,
        station_pattern: &str,
        stream_pattern: Option<String>,
        format_subformat_pattern: Option<String>,
    ) -> &Vec<Station> {
        todo!()
    }

    // async fn shutdown(&self) -> SeedLinkResult<()> {
    //     Ok(())
    // }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut server = SeedLinkServerBackend::default();

    let (server_handle, join_handle) = slink_server::spawn_main_loop(server);

    tokio::spawn(async move {
        let bind = ([0, 0, 0, 0], DEFAULT_PORT).into();
        slink_server::start_accept(bind, server_handle).await;
    });

    info!("Starting on port {}", DEFAULT_PORT);

    join_handle.await.unwrap();
}
