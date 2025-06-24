use bincode::config::Configuration;
use bincode::{config, decode_from_slice, Decode};
use bincode::{encode_to_vec, Encode};
use quinn::{RecvStream, SendStream};

pub const PACKET_CONFIG: Configuration = config::standard();
#[derive(Encode, Decode, Debug)]
pub enum Packet {
    PlayerJoin { id: u32 },
    PlayerPosition { x: f32, y: f32, z: f32 },
}

#[tracing::instrument]
pub async fn send_packet(mut send: SendStream, packet: Packet) -> anyhow::Result<()> {
    let packet = encode_to_vec(packet, PACKET_CONFIG)?;
    send.write_all(packet.as_slice()).await?;
    send.finish()?;

    Ok(())
}

#[tracing::instrument]
pub async fn receive_packet(mut recv: RecvStream) -> anyhow::Result<Packet> {
    let packet = recv.read_to_end(64).await?;
    let (packet, _): (Packet, usize) = decode_from_slice(packet.as_slice(), PACKET_CONFIG)?;
    Ok(packet)
}
