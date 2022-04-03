//! Connection over TCP.

use std::io;

use tokio::net::{TcpStream, ToSocketAddrs};

use crate::client::Client;

/// Connects to the IKE daemon via a TCP connection. See [`Client`][] for its usage.
pub async fn connect<A>(addr: A) -> io::Result<Client>
where
    A: ToSocketAddrs,
{
    let session = TcpStream::connect(&addr).await?;
    Ok(Client::new(session))
}
