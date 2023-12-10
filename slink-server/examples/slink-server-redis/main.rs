use tokio::sync::mpsc::Sender;
use tracing::info;
use tracing_subscriber;

use slink::{ProtocolErrorV4, SeedLinkPacketV4, Station};
use slink_server::{DataTransferMode, SeedLinkServer, Select};

use slink::DEFAULT_PORT;

// TODO(damb): client specific data required for streaming
#[derive(Clone, Debug, Default)]
struct Client;

#[derive(Debug, Default)]
struct SeedLinkServerBackend;

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
        stream_pattern: &Option<String>,
        format_subformat_pattern: &Option<String>,
    ) -> Result<&Vec<Station>, ProtocolErrorV4> {
        todo!()
    }

    /// Returns the inventory including stream related data.
    async fn inventory_streams(
        &self,
        station_pattern: &str,
        stream_pattern: &Option<String>,
        format_subformat_pattern: &Option<String>,
    ) -> Result<&Vec<Station>, ProtocolErrorV4> {
        todo!()
    }

    async fn packets(
        &mut self,
        selects: Vec<Select>,
        mode: DataTransferMode,
        tx: Sender<Result<SeedLinkPacketV4, ProtocolErrorV4>>,
    ) -> Result<(), ProtocolErrorV4> {
        todo!()
    }

    // async fn shutdown(&self) -> SeedLinkResult<()> {
    //     Ok(())
    // }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let server = SeedLinkServerBackend::default();

    let (server_handle, join_handle) = slink_server::spawn_main_loop(server);

    tokio::spawn(async move {
        let bind = ([0, 0, 0, 0], DEFAULT_PORT).into();
        slink_server::start_accept(bind, server_handle).await;
    });

    info!("Starting on port {}", DEFAULT_PORT);

    join_handle.await.unwrap();
}
