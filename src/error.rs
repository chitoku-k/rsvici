//! Failures to interact with the IKE daemon.

use core::result;
use std::{
    error,
    fmt::{self, Debug, Display},
    io,
};

/// A structure representing all possible errors in rsvici.
pub struct Error {
    err: Box<ErrorImpl>,
}

struct ErrorImpl {
    code: ErrorCode,
}

/// Alias for a `Result` with the error type `rsvici::Error`.
pub type Result<T> = result::Result<T, Error>;

impl Error {
    /// Categorizes the cause of this error.
    ///
    /// - `Category::Io` - failure to read or write bytes on an IO stream
    /// - `Category::Data` - invalid data
    /// - `Category::Closed` - a listener or handler has already been closed
    /// - `Category::CmdFailure` - failure to execute a command
    /// - `Category::UnknownCmd` - an unknown command request
    /// - `Category::UnknownEvent` - an unknown event request
    pub fn classify(&self) -> Category {
        match self.err.code {
            ErrorCode::Io(_) => Category::Io,
            ErrorCode::InvalidData(_) | ErrorCode::UnexpectedPacket(_) => Category::Data,
            ErrorCode::ListenerClosed
            | ErrorCode::HandlerClosedWhileCommandRequest
            | ErrorCode::HandlerClosedWhileEventRequest(_)
            | ErrorCode::HandlerClosedWhileStreaming(_) => Category::Closed,
            ErrorCode::CommandFailed(_) => Category::CmdFailure,
            ErrorCode::UnknownCmd => Category::UnknownCmd,
            ErrorCode::UnknownEvent(_) => Category::UnknownEvent,
        }
    }

    /// Returns true if this error was caused by a failure to read or write bytes on an IO stream.
    pub fn is_io(&self) -> bool {
        self.classify() == Category::Io
    }

    /// Returns true if this error was caused by invalid data.
    pub fn is_data(&self) -> bool {
        self.classify() == Category::Data
    }

    /// Returns true if this error was caused by a listener or handler being already closed.
    pub fn is_closed(&self) -> bool {
        self.classify() == Category::Closed
    }

    /// Returns true if this error was caused by a failure to execute a command.
    pub fn is_command_failed(&self) -> bool {
        self.classify() == Category::CmdFailure
    }

    /// Returns true if this error was caused by an unknown command request.
    pub fn is_unknown_cmd(&self) -> bool {
        self.classify() == Category::UnknownCmd
    }

    /// Returns true if this error was caused by an unknown event request.
    pub fn is_unknown_event(&self) -> bool {
        self.classify() == Category::UnknownEvent
    }

    pub(crate) fn io(e: io::Error) -> Self {
        Self {
            err: Box::new(ErrorImpl { code: ErrorCode::Io(e) }),
        }
    }

    pub(crate) fn data(code: ErrorCode) -> Self {
        Self {
            err: Box::new(ErrorImpl { code }),
        }
    }
}

/// Categorizes the cause of an `rsvici::Error`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Category {
    /// The error was caused by a failure to read or write bytes on an IO stream.
    Io,

    /// The error was caused by invalid data.
    Data,

    /// The error was caused by a listener or handler being already closed.
    Closed,

    /// The error was caused by a failure to execute a command.
    CmdFailure,

    /// The error was caused by an unknown command request.
    UnknownCmd,

    /// The error was caused by an unknown event request.
    UnknownEvent,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::io(e)
    }
}

impl From<Error> for io::Error {
    /// Convert an `rsvici::Error` into an `io::Error`.
    fn from(e: Error) -> Self {
        match e.classify() {
            Category::Io => {
                if let ErrorCode::Io(e) = e.err.code {
                    e
                } else {
                    unreachable!()
                }
            },
            Category::Data => io::Error::new(io::ErrorKind::InvalidData, e),
            Category::Closed => io::Error::new(io::ErrorKind::BrokenPipe, e),
            Category::CmdFailure => io::Error::other(e),
            Category::UnknownCmd | Category::UnknownEvent => io::Error::new(io::ErrorKind::Unsupported, e),
        }
    }
}

impl From<serde_vici::Error> for Error {
    fn from(e: serde_vici::Error) -> Self {
        Error::data(ErrorCode::InvalidData(e))
    }
}

pub(crate) enum ErrorCode {
    /// Some IO error occurred in rsvici.
    Io(io::Error),

    /// Invalid data when serializing/deserializing payload.
    InvalidData(serde_vici::Error),

    /// Listener has already been closed.
    ListenerClosed,

    /// Handler has already been closed while processing a command request.
    HandlerClosedWhileCommandRequest,

    /// Handler has already been closed while processing a named event request.
    HandlerClosedWhileEventRequest(String),

    /// Handler has already been closed while processing the stream.
    HandlerClosedWhileStreaming(String),

    /// Unexpected packet has been received, such as no handler is registered or unknown type is encountered.
    UnexpectedPacket(String),

    /// Issued command failed.
    CommandFailed(Option<String>),

    /// Unknown command has been requested.
    UnknownCmd,

    /// Unknown event has been requested.
    UnknownEvent(String),
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorCode::Io(ref err) => Display::fmt(err, f),
            ErrorCode::InvalidData(ref err) => Display::fmt(err, f),
            ErrorCode::ListenerClosed => f.write_str("listener has been closed"),
            ErrorCode::HandlerClosedWhileCommandRequest => f.write_str("handler has been closed while processing command request"),
            ErrorCode::HandlerClosedWhileEventRequest(ref event) => f.write_fmt(format_args!("handler has been closed while processing event: {event}")),
            ErrorCode::HandlerClosedWhileStreaming(ref packet_type) => f.write_fmt(format_args!("handler has been closed while processing {packet_type}")),
            ErrorCode::UnexpectedPacket(ref packet_type) => f.write_fmt(format_args!("unexpected packet type {packet_type}")),
            ErrorCode::CommandFailed(ref reason) => match reason {
                Some(errmsg) => f.write_fmt(format_args!("command failed: {errmsg}")),
                None => f.write_str("command failed"),
            },
            ErrorCode::UnknownCmd => f.write_str("unknown command"),
            ErrorCode::UnknownEvent(ref event) => f.write_fmt(format_args!("unknown event {event}")),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.err.code {
            ErrorCode::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.err.code, f)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error({:?})", self.err.code.to_string())
    }
}
