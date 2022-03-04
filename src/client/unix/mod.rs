//! Connection over a Unix Domain Stream Socket.

use std::{io, path::Path};

use tokio::net::UnixStream;

use crate::client::Client;

/// Connects to the IKE daemon via a Unix socket named by `path`. See [`Client`][] for its usage.
pub async fn connect<P>(path: P) -> io::Result<Client>
where
    P: AsRef<Path>,
{
    let session = UnixStream::connect(&path).await?;
    Ok(Client::new(session))
}
