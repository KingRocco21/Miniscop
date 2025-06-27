use bincode::config::Configuration;
use bincode::{config, decode_from_slice, Decode};
use bincode::{encode_to_vec, Encode};
use quinn::{RecvStream, SendStream};

pub const PACKET_CONFIG: Configuration = config::standard();
#[derive(Encode, Decode, Debug, Copy, Clone, PartialEq)]
pub enum Packet {
    /// Client will be kicked if it sends this.
    /// Its current purpose is to signal to the client that it can start sending packets.
    ClientConnect,
    /// Client will be disconnected if they send this regardless of the ID inside, so they might as well send None.
    ClientDisconnect(Option<u64>),
    /// Client should send None for id because it doesn't know its own id.
    PlayerMovement {
        id: Option<u64>,
        x: f32,
        y: f32,
        z: f32,
        velocity_x: f32,
        velocity_y: f32,
        velocity_z: f32,
    },
}

/// Note: This future finishes when the packet sent, not when it is received by the server.
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
