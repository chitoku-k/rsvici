use async_stream::try_stream;
use futures_util::Stream;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::{self, Sender, UnboundedSender},
    task,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use self::{
    listener::{Listener, Registration},
    packet::Packet,
    packet_type::PacketType,
};
use crate::error::{self, Error, ErrorCode};

pub mod tcp;

#[cfg(unix)]
pub mod unix;

mod listener;
mod packet;
mod packet_type;
mod session;

type Handler = Sender<error::Result<Packet>>;
type CommandSender = Sender<(Packet, Handler)>;
type EventSender = Sender<(Packet, String, Registration, Handler)>;
type ErrorHandlerSender = UnboundedSender<UnboundedSender<Error>>;

#[derive(Deserialize)]
struct Response {
    success: Option<bool>,
    errmsg: Option<String>,
}

/// A structure to interact with the IKE daemon using the VICI protocol.
pub struct Client {
    commands: CommandSender,
    events: EventSender,
    error_handler: ErrorHandlerSender,
    listener: task::JoinHandle<()>,
}

impl Client {
    /// Creates an rsvici client from a stream.
    ///
    /// Typically it is more convenient to use either of the following methods instead:
    ///
    /// - [`rsvici::tcp::connect`]
    #[cfg_attr(unix, doc = "- [`rsvici::unix::connect`]")]
    ///
    /// [`rsvici::tcp::connect`]: tcp::connect
    #[cfg_attr(unix, doc = "[`rsvici::unix::connect`]: unix::connect")]
    pub fn new<S>(session: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (commands_tx, commands_rx) = mpsc::channel(8);
        let (events_tx, events_rx) = mpsc::channel(8);
        let (error_handler_tx, error_handler_rx) = mpsc::unbounded_channel();

        Self {
            commands: commands_tx,
            events: events_tx,
            error_handler: error_handler_tx,
            listener: Listener::new(session).start(commands_rx, events_rx, error_handler_rx),
        }
    }

    /// Makes a request call and receives a response.
    ///
    /// Since the IKE daemon does not support sequence numbers that associate a request and response, do not make more than one request call at a time.
    ///
    /// For the list of available commands, see [Client-initiated commands][].
    ///
    /// # Example
    #[cfg_attr(unix, doc = "```no_run")]
    #[cfg_attr(not(unix), doc = "```ignore")]
    /// use std::error::Error;
    ///
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct Version {
    ///     daemon: String,
    ///     version: String,
    ///     sysname: String,
    ///     release: String,
    ///     machine: String,
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut client = rsvici::unix::connect("/run/charon.vici").await?;
    ///
    ///     let version: Version = client.request("version", ()).await?;
    ///     println!("Version: {:#?}", version);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [Client-initiated commands]: https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#client-initiated-commands
    pub async fn request<T, U>(&mut self, cmd: &str, message: T) -> error::Result<U>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let (tx, mut rx) = mpsc::channel(1);

        let req = Packet::from(PacketType::CmdRequest(cmd.to_string()), message)?;
        self.commands.send((req, tx)).await.map_err(|_| Error::data(ErrorCode::ListenerClosed))?;

        match rx.recv().await {
            Some(Ok(packet)) => packet.message().map_err(Into::into),
            Some(Err(e)) => Err(e),
            None => Err(Error::data(ErrorCode::ListenerClosed)),
        }
    }

    /// Makes a streamed request call and iterates through its responses.
    ///
    /// Since the IKE daemon does not support sequence numbers that associate a request and response, do not make more than one request call at a time.
    ///
    /// For the list of available commands, see [Client-initiated commands][] and [Server-issued events][].
    ///
    /// You may also want to have a look at the documentation for [async-stream][] and [futures-util][].
    ///
    /// # Example
    #[cfg_attr(unix, doc = "```no_run")]
    #[cfg_attr(not(unix), doc = "```ignore")]
    /// use std::{
    ///     collections::HashMap,
    ///     error::Error,
    /// };
    ///
    /// use futures_util::{
    ///     stream::{StreamExt, TryStreamExt},
    ///     pin_mut,
    /// };
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct Conn {
    ///     local_addrs: Vec<String>,
    ///     remote_addrs: Vec<String>,
    ///     version: String,
    ///     reauth_time: u64,
    ///     rekey_time: u64,
    /// }
    ///
    /// type Conns = HashMap<String, Conn>;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut client = rsvici::unix::connect("/run/charon.vici").await?;
    ///
    ///     let conns = client.stream_request::<(), Conns>("list-conns", "list-conn", ());
    ///     pin_mut!(conns);
    ///
    ///     while let Some(conn) = conns.try_next().await? {
    ///         println!("Conn: {:#?}", conn);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [async-stream]:              https://docs.rs/async-stream
    /// [futures-util]:              https://docs.rs/futures-util
    /// [Client-initiated commands]: https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#client-initiated-commands
    /// [Server-issued events]:      https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#server-issued-events
    pub fn stream_request<T, U>(&mut self, cmd: &str, event: &str, message: T) -> impl Stream<Item = error::Result<U>>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let events = self.events.clone();
        let commands = self.commands.clone();

        let cmd = cmd.to_string();
        let event = event.to_string();

        try_stream! {
            let (tx, mut rx) = mpsc::channel(1);
            let cmd_response: Response;

            let req = Packet::from(PacketType::EventRegister(event.clone()), ())?;
            events
                .send((req, event.clone(), Registration::Register, tx.clone()))
                .await
                .map_err(|_| Error::data(ErrorCode::ListenerClosed))?;

            match rx.recv().await {
                Some(Ok(_)) => {},
                Some(Err(e)) => Err(e)?,
                None => Err(Error::data(ErrorCode::ListenerClosed))?,
            }

            let req = Packet::from(PacketType::CmdRequest(cmd), message)?;
            commands.send((req, tx)).await.map_err(|_| Error::data(ErrorCode::ListenerClosed))?;

            loop {
                match rx.recv().await {
                    Some(Ok(packet)) => match packet.packet_type() {
                        PacketType::CmdResponse => {
                            match packet.message() {
                                Ok(resp) => {
                                    cmd_response = resp;
                                    break;
                                },
                                Err(e) => {
                                    Err(Error::from(e))?;
                                },
                            }
                        },
                        PacketType::Event(_) => {
                            match packet.message() {
                                Ok(item) => {
                                    yield item;
                                },
                                Err(e) => {
                                    Err(Error::from(e))?;
                                },
                            }
                        },
                        packet_type => {
                            Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())))?;
                        },
                    },
                    Some(Err(e)) => {
                        Err(e)?;
                    },
                    None => {
                        Err(Error::data(ErrorCode::ListenerClosed))?;
                    },
                }
            }

            let (unregister_tx, mut unregister_rx) = mpsc::channel(1);

            let req = Packet::from(PacketType::EventUnregister(event.clone()), ())?;
            events
                .send((req, event, Registration::Unregister, unregister_tx))
                .await
                .map_err(|_| Error::data(ErrorCode::ListenerClosed))?;

            match unregister_rx.recv().await {
                Some(Ok(_)) => {},
                Some(Err(e)) => Err(e)?,
                None => Err(Error::data(ErrorCode::ListenerClosed))?,
            }

            match cmd_response.success {
                Some(true) => {},
                Some(false) => Err(Error::data(ErrorCode::CommandFailed(cmd_response.errmsg)))?,
                None => {},
            }
        }
    }

    /// Subscribes to an event and iterates through its messages. It is safe to subscribe to events while making other requests at a time. The rsvici will
    /// automatically unsubscribe from the event when the returned stream is dropped.
    ///
    /// For the list of available events, see [Server-issued events][].
    ///
    /// You may also want to have a look at the documentation for [async-stream][] and [futures-util][].
    ///
    /// # Example
    #[cfg_attr(unix, doc = "```no_run")]
    #[cfg_attr(not(unix), doc = "```ignore")]
    /// use std::error::Error;
    ///
    /// use futures_util::{
    ///     stream::{StreamExt, TryStreamExt},
    ///     pin_mut,
    /// };
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct Log {
    ///     group: String,
    ///     level: u32,
    ///     thread: u32,
    ///     msg: String,
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut client = rsvici::unix::connect("/run/charon.vici").await?;
    ///
    ///     let logs = client.subscribe::<Log>("log");
    ///     tokio::spawn(async move {
    ///         pin_mut!(logs);
    ///
    ///         while let Some(log) = logs.try_next().await? {
    ///             println!("Log: {:#?}", log);
    ///         }
    ///
    ///         Ok::<(), rsvici::Error>(())
    ///     });
    ///
    ///     // Do other stuff with `client` here while streaming logs from the daemon...
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [async-stream]:         https://docs.rs/async-stream
    /// [futures-util]:         https://docs.rs/futures-util
    /// [Server-issued events]: https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#server-issued-events
    pub fn subscribe<U>(&mut self, event: &str) -> impl Stream<Item = error::Result<U>>
    where
        U: DeserializeOwned,
    {
        let events = self.events.clone();
        let event = event.to_string();

        try_stream! {
            let (tx, mut rx) = mpsc::channel(1);

            let req = Packet::from(PacketType::EventRegister(event.clone()), ())?;
            events
                .send((req, event.clone(), Registration::Register, tx.clone()))
                .await
                .map_err(|_| Error::data(ErrorCode::ListenerClosed))?;

            match rx.recv().await {
                Some(Ok(_)) => {},
                Some(Err(e)) => Err(e)?,
                None => Err(Error::data(ErrorCode::ListenerClosed))?,
            }

            tokio::spawn(async move {
                tx.closed().await;

                let (unregister_tx, mut unregister_rx) = mpsc::channel(1);

                let req = Packet::from(PacketType::EventUnregister(event.clone()), ())?;
                events
                    .send((req, event, Registration::Unregister, unregister_tx))
                    .await
                    .map_err(|_| Error::data(ErrorCode::ListenerClosed))?;

                match unregister_rx.recv().await {
                    Some(Ok(_)) => Ok(()),
                    Some(Err(e)) => Err(e),
                    None => Err(Error::data(ErrorCode::ListenerClosed)),
                }
            });

            loop {
                match rx.recv().await {
                    Some(Ok(packet)) => match (packet.packet_type(), packet.message()) {
                        (PacketType::Event(_), Ok(item)) => {
                            yield item;
                        },
                        (PacketType::Event(_), Err(e)) => {
                            Err(Error::from(e))?;
                        },
                        (packet_type, _) => {
                            Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())))?;
                        },
                    },
                    Some(Err(e)) => {
                        Err(e)?;
                    },
                    None => {
                        Err(Error::data(ErrorCode::ListenerClosed))?;
                    },
                }
            }
        }
    }

    /// Listens for background errors, such as unexpected messages or unhandled packets, and iterates them.
    ///
    /// There can be only one error handler. If you call this method a second (or more) time, the registered listener will be dropped.
    /// Dropping the `Stream` will simply make the listener discard the background errors.
    ///
    /// # Example
    #[cfg_attr(unix, doc = "```no_run")]
    #[cfg_attr(not(unix), doc = "```ignore")]
    /// use std::error::Error;
    ///
    /// use futures_util::{
    ///     stream::{StreamExt, TryStreamExt},
    ///     pin_mut,
    /// };
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct Log {
    ///     group: String,
    ///     level: u32,
    ///     thread: u32,
    ///     msg: String,
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut client = rsvici::unix::connect("/run/charon.vici").await?;
    ///
    ///     let mut errors = client.listen_for_errors();
    ///     tokio::spawn(async move {
    ///         while let Some(e) = errors.next().await {
    ///             println!("Error: {}", e);
    ///         }
    ///     });
    ///
    ///     // Do other stuff with `client` here while logging background errors...
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn listen_for_errors(&mut self) -> impl Stream<Item = Error> {
        let (tx, rx) = mpsc::unbounded_channel();
        let _ = self.error_handler.send(tx);

        UnboundedReceiverStream::new(rx)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.listener.abort();
    }
}
