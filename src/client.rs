use std::time::Duration;

use crate::{connect, Connection, ConnectionInfo, IntoConnectionInfo, SeedLinkResult};

// TODO(damb):
// - allow the user to make use of certain protocol versions e.g. by means of using the URL syntax
// `slink+v3://`, `slink+v4://` (check out the syntax for tls/unix socket connections)
// - allow to switch the protocol version (if still possible)

/// The client acts as connector to the SeedLink server. By itself it does not
/// do much other than providing a convenient way to fetch a connection from
/// it.
///
/// When opening a client a URL in the following format should be used:
///
/// ```plain
/// slink://host:port/
/// ```
///
/// Example usage::
///
/// ```rust,no_run
/// let client = slink::Client::open("slink://127.0.0.1/").unwrap();
/// let con = client.get_connection().await.unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    connection_info: ConnectionInfo,
}

impl Client {
    /// Connects to a SeedLink server and returns a client.  This does not
    /// actually open a connection yet but it does perform some basic
    /// checks on the URL that might make the operation fail.
    pub fn open<T: IntoConnectionInfo>(params: T) -> SeedLinkResult<Self> {
        Ok(Self {
            connection_info: params.into_connection_info()?,
        })
    }

    /// Instructs the client to actually connect to SeedLink and returns a connection object. The
    /// connection object can be used to communicate with the server. This can fail with a variety
    /// of errors (like unreachable host) so it's important that you handle those errors.
    pub async fn get_connection(&self) -> SeedLinkResult<Connection> {
        connect(&self.connection_info, None).await
    }

    /// Instructs the client to actually connect to SeedLink with the specified timeout and returns
    /// a connection object. The connection object can be used to send commands to the server.
    /// This can fail with a variety of errors (like unreachable host) so it's important that you
    /// handle those errors.
    pub async fn get_connection_with_timeout(
        &self,
        timeout: Duration,
    ) -> SeedLinkResult<Connection> {
        connect(&self.connection_info, Some(timeout)).await
    }

    /// Returns a reference of client connection info object.
    pub fn get_connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
}

