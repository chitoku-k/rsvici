use std::{fmt::Display, io};

pub(crate) enum PacketType {
    /// A named request message.
    CmdRequest(String),

    /// An unnamed response message for a request.
    CmdResponse,

    /// An unnamed response if requested command is unknown.
    CmdUnknown,

    /// A named event registration request.
    EventRegister(String),

    /// A named event deregistration request.
    EventUnregister(String),

    /// An unnamed response for successful event (de-)registration.
    EventConfirm,

    /// An unnamed response if event (de-)registration failed.
    EventUnknown,

    /// A named event message.
    Event(String),
}

impl PacketType {
    pub fn tag(&self) -> u8 {
        match self {
            Self::CmdRequest(_) => 0,
            Self::CmdResponse => 1,
            Self::CmdUnknown => 2,
            Self::EventRegister(_) => 3,
            Self::EventUnregister(_) => 4,
            Self::EventConfirm => 5,
            Self::EventUnknown => 6,
            Self::Event(_) => 7,
        }
    }

    pub fn from_tag(tag: &u8) -> Option<Self> {
        match tag {
            0 => Some(Self::CmdRequest(Default::default())),
            1 => Some(Self::CmdResponse),
            2 => Some(Self::CmdUnknown),
            3 => Some(Self::EventRegister(Default::default())),
            4 => Some(Self::EventUnregister(Default::default())),
            5 => Some(Self::EventConfirm),
            6 => Some(Self::EventUnknown),
            7 => Some(Self::Event(Default::default())),
            _ => None,
        }
    }

    pub fn marshal<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_all(&[self.tag()])?;

        match &self {
            PacketType::CmdRequest(name) | PacketType::EventRegister(name) | PacketType::EventUnregister(name) | PacketType::Event(name) => {
                writer.write_all(&[name.len() as u8])?;
                writer.write_all(name.as_bytes())?;
            },
            _ => {},
        }

        Ok(())
    }

    pub fn unmarshal(input: &[u8]) -> io::Result<(&[u8], Self)> {
        let (packet_type, input) = input.split_first().ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"))?;

        let mut packet_type = PacketType::from_tag(packet_type).ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "unknown type"))?;
        match packet_type {
            PacketType::CmdRequest(ref mut name)
            | PacketType::EventRegister(ref mut name)
            | PacketType::EventUnregister(ref mut name)
            | PacketType::Event(ref mut name) => {
                let (name_len, input) = input.split_first().ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"))?;

                let name_len = *name_len as usize;
                if input.len() < name_len {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
                }

                let (v, input) = input.split_at(name_len);
                *name = String::from_utf8_lossy(v).into();

                Ok((input, packet_type))
            },
            _ => Ok((input, packet_type)),
        }
    }
}

impl Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            PacketType::CmdRequest(name) => f.write_fmt(format_args!("CMD_REQUEST({name})")),
            PacketType::CmdResponse => f.write_str("CMD_RESPONSE"),
            PacketType::CmdUnknown => f.write_str("CMD_UNKNOWN"),
            PacketType::EventRegister(name) => f.write_fmt(format_args!("EVENT_REGISTER({name})")),
            PacketType::EventUnregister(name) => f.write_fmt(format_args!("EVENT_UNREGISTER({name})")),
            PacketType::EventConfirm => f.write_str("EVENT_CONFIRM"),
            PacketType::EventUnknown => f.write_str("EVENT_UNKNOWN"),
            PacketType::Event(name) => f.write_fmt(format_args!("EVENT({name})")),
        }
    }
}
