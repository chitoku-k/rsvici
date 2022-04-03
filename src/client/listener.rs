use std::{
    collections::{HashMap, VecDeque},
    io,
};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
    sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::error::{self, Error, ErrorCode};

use super::{packet::Packet, packet_type::PacketType, Handler};

type CommandReceiver = Receiver<(Packet, Handler)>;
type EventReceiver = Receiver<(Packet, String, Registration, Handler)>;
type ErrorHandlerReceiver = UnboundedReceiver<UnboundedSender<Error>>;

pub(crate) enum Registration {
    Register,
    Unregister,
}

pub(crate) struct Listener<S> {
    session: S,
    command_queue: VecDeque<Handler>,
    event_queue: VecDeque<(String, Registration, Handler)>,
    event_subscriptions: HashMap<String, Handler>,
    error_handler: Option<UnboundedSender<Error>>,
}

impl<S> Listener<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub fn new(session: S) -> Self {
        Self {
            session,
            command_queue: VecDeque::new(),
            event_queue: VecDeque::new(),
            event_subscriptions: HashMap::new(),
            error_handler: None,
        }
    }

    pub fn start(mut self, mut commands: CommandReceiver, mut events: EventReceiver, mut error_handler: ErrorHandlerReceiver) -> task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let result = select! {
                    Some((packet, handler)) = commands.recv() => {
                        self.on_command_request(packet, handler).await
                    },
                    Some((packet, event, registration, handler)) = events.recv() => {
                        self.on_event_request(packet, event, registration, handler).await
                    },
                    handler = error_handler.recv() => {
                        self.error_handler = handler;
                        continue;
                    },
                    res = Packet::receive(&mut self.session) => {
                        self.on_response(res).await
                    },
                };

                if let (Some(error_handler), Err(e)) = (&self.error_handler, result) {
                    if error_handler.send(e).is_err() {
                        self.error_handler = None;
                    }
                }
            }
        })
    }

    async fn on_command_request(&mut self, packet: Packet, handler: Handler) -> error::Result<()> {
        match packet.send(&mut self.session).await {
            Ok(()) => self.command_queue.push_back(handler),
            Err(e) => handler
                .send(Err(e.into()))
                .await
                .map_err(|_| Error::data(ErrorCode::HandlerClosedWhileCommandRequest))?,
        }

        Ok(())
    }

    async fn on_event_request(&mut self, packet: Packet, event: String, registration: Registration, handler: Handler) -> error::Result<()> {
        match packet.send(&mut self.session).await {
            Ok(()) => {},
            Err(e) => {
                handler
                    .send(Err(e.into()))
                    .await
                    .map_err(|_| Error::data(ErrorCode::HandlerClosedWhileEventRequest(event)))?;

                return Ok(());
            },
        }

        self.event_queue.push_back((event, registration, handler));
        Ok(())
    }

    async fn on_response(&mut self, res: io::Result<Packet>) -> error::Result<()> {
        let packet = res?;
        match packet.packet_type() {
            packet_type @ PacketType::CmdResponse => match self.command_queue.pop_front() {
                Some(handler) => {
                    handler
                        .send(Ok(packet))
                        .await
                        .map_err(|e| Error::data(ErrorCode::HandlerClosedWhileStreaming(e.0.unwrap().packet_type().to_string())))?;
                },
                None => {
                    return Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())));
                },
            },
            packet_type @ PacketType::CmdUnknown => match self.command_queue.pop_front() {
                Some(handler) => {
                    handler
                        .send(Err(Error::data(ErrorCode::UnknownCmd)))
                        .await
                        .map_err(|_| Error::data(ErrorCode::HandlerClosedWhileStreaming(packet_type.to_string())))?;
                },
                None => {
                    return Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())));
                },
            },
            packet_type @ PacketType::EventConfirm => match self.event_queue.pop_front() {
                Some((event, Registration::Register, handler)) => {
                    handler
                        .send(Ok(packet))
                        .await
                        .map_err(|e| Error::data(ErrorCode::HandlerClosedWhileStreaming(e.0.unwrap().packet_type().to_string())))?;

                    self.event_subscriptions.insert(event, handler);
                },
                Some((event, Registration::Unregister, handler)) => {
                    self.event_subscriptions.remove(&event);

                    handler
                        .send(Ok(packet))
                        .await
                        .map_err(|e| Error::data(ErrorCode::HandlerClosedWhileStreaming(e.0.unwrap().packet_type().to_string())))?;
                },
                None => {
                    return Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())));
                },
            },
            packet_type @ PacketType::EventUnknown => match self.event_queue.pop_front() {
                Some((event, Registration::Register, handler)) => {
                    handler
                        .send(Err(Error::data(ErrorCode::UnknownEvent(event))))
                        .await
                        .map_err(|_| Error::data(ErrorCode::HandlerClosedWhileStreaming(packet_type.to_string())))?;
                },
                Some((event, Registration::Unregister, handler)) => {
                    self.event_subscriptions.remove(&event);

                    handler
                        .send(Err(Error::data(ErrorCode::UnknownEvent(event))))
                        .await
                        .map_err(|_| Error::data(ErrorCode::HandlerClosedWhileStreaming(packet_type.to_string())))?;
                },
                None => {
                    return Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())));
                },
            },
            packet_type @ PacketType::Event(name) => match self.event_subscriptions.get(name) {
                Some(handler) => {
                    handler
                        .send(Ok(packet))
                        .await
                        .map_err(|e| Error::data(ErrorCode::HandlerClosedWhileStreaming(e.0.unwrap().packet_type().to_string())))?;
                },
                None => {
                    return Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())));
                },
            },
            packet_type => {
                return Err(Error::data(ErrorCode::UnexpectedPacket(packet_type.to_string())));
            },
        }

        Ok(())
    }
}
