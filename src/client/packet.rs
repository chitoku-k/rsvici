use std::io;

use serde::{de::DeserializeOwned, Serialize};

use super::PacketType;

pub(crate) struct Packet {
    packet_type: PacketType,
    payload: Vec<u8>,
}

impl Packet {
    pub fn new(packet_type: PacketType, payload: Vec<u8>) -> Self {
        Self { packet_type, payload }
    }

    pub fn from<T>(packet_type: PacketType, message: T) -> io::Result<Self>
    where
        T: Serialize,
    {
        let payload = serde_vici::to_vec(&message)?;

        Ok(Self { packet_type, payload })
    }

    pub fn message<T>(&self) -> io::Result<T>
    where
        T: DeserializeOwned,
    {
        let message = serde_vici::from_slice(&self.payload)?;

        Ok(message)
    }

    pub fn packet_type(&self) -> &PacketType {
        &self.packet_type
    }

    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut buf = vec![];

        self.packet_type.marshal(&mut buf)?;
        buf.extend_from_slice(&self.payload);

        Ok(buf)
    }

    pub fn deserialize(slice: &[u8]) -> io::Result<Packet> {
        let (buf, packet_type) = PacketType::unmarshal(slice)?;

        Ok(Packet::new(packet_type, buf.to_vec()))
    }
}
