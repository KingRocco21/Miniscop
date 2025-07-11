use bitcode::{decode, encode, Decode, Encode};
use quinn::{RecvStream, SendStream};

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
        animation_frame: u8,
    },
}

/// Note: This future finishes when the packet sent, not when it is received by the other endpoint.
#[tracing::instrument(skip(send))]
pub async fn send_packet(mut send: SendStream, packet: Packet) -> anyhow::Result<()> {
    let packet = encode(&packet);
    send.write_all(packet.as_slice()).await?;
    send.finish()?;

    Ok(())
}

#[tracing::instrument(skip(recv))]
pub async fn receive_packet(mut recv: RecvStream) -> anyhow::Result<Packet> {
    let packet = recv.read_to_end(64).await?;
    let packet = decode::<Packet>(packet.as_slice())?;
    Ok(packet)
}
