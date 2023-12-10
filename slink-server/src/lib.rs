mod accept;
mod client;
mod dispatch;
mod negotiate;
mod response;
mod seedlink;
mod select;
mod server;
mod util;

pub use accept::start_accept;
pub use server::{spawn_main_loop, ServerHandle};
pub use select::Select;

use slink::{AuthV4, Station, ProtocolErrorV4};

/// A re-export of [`async-trait`](https://docs.rs/async-trait) for convenience.
pub use async_trait::async_trait;

/// Server-side default protocol version.
pub const DEFAULT_PROTO_VERSION: (u8, u8) = (4, 0);
/// Server-side highest supported protocol version.
pub const HIGHEST_SUPPORTED_PROTO_VERSION: (u8, u8) = (4, 0);

/// Client identifier.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClientId(usize);

/// Trait implemented by SeedLink server implementations.
///
/// This interface allows servers adhering to the SeedLink protocol to be implemented in a safe way
/// without exposing the low-level implementation details.
#[async_trait]
pub trait SeedLinkServer: Send + Sync + 'static {
    /// Returns the software implementation.
    fn implementation(&self) -> &str;

    /// Returns the software implementation version.
    fn implementation_version(&self) -> &str;

    /// Returns the data center description.
    fn data_center_description(&self) -> &str;

    /// Authenticates a client.
    ///
    /// TODO(damb): support multiple protocol versions
    async fn authenticate(&self, auth: &AuthV4) -> Result<(), ProtocolErrorV4> {
        Err(ProtocolErrorV4::unsupported_command())
    }

    /// Returns the inventory without stream related data.
    async fn inventory_stations(
        &self,
        station_pattern: &str,
        stream_pattern: Option<String>,
        format_subformat_pattern: Option<String>,
    ) -> Result<&Vec<Station>, ProtocolErrorV4>;

    /// Returns the inventory including stream related data.
    async fn inventory_streams(
        &self,
        station_pattern: &str,
        stream_pattern: Option<String>,
        format_subformat_pattern: Option<String>,
    ) -> Result<&Vec<Station>, ProtocolErrorV4>;

    // async fn initialize(&self) -> SeedLinkResult<()>;

    // async fn shutdown(&self) -> SeedLinkResult<()>;
}

#[cfg(test)]
mod tests {
    // TODO
}
