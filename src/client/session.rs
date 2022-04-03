use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::Packet;

impl Packet {
    pub async fn send<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        let buf = self.serialize()?;
        let len = buf.len() as u32;

        writer.write_all(&len.to_be_bytes()).await?;
        writer.write_all(&buf).await?;

        Ok(())
    }

    pub async fn receive<R>(reader: &mut R) -> io::Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let len = reader.read_u32().await?;
        let mut buf = vec![0; len as usize];

        reader.read_exact(&mut buf).await?;
        let packet = Packet::deserialize(&buf)?;

        Ok(packet)
    }
}
