use bitcode::{decode, encode, Buffer, Decode, Encode};
use quinn::{RecvStream, SendStream};

/// Bitcode-encoded packet types to be sent and received by the client and server.
///
/// Read each enum variant for more information.
#[derive(Encode, Decode, Debug, Copy, Clone, PartialEq)]
pub enum Packet {
    /// Server should send this upon a successful connection to signal to the client that it can start sending packets.
    ///
    /// Client should be kicked if it sends this.
    ServerReady,
    /// Server should send Some() whenever another client disconnects. Server should never send None.
    ///
    /// Client should send None when it wants to disconnect. Client should never send Some().
    ClientDisconnect(Option<u64>),
    /// Server should always send an id.
    ///
    /// Client should send None for id because it doesn't know its own id.
    PlayerMovement {
        id: Option<u64>,
        x: f32,
        y: f32,
        z: f32,
        animation_frame: u8,
    },
}

/// Note: This future finishes when the packet is sent, not when it is received by the other endpoint.
#[tracing::instrument(skip(send, buffer))]
pub async fn send_packet(
    mut send: SendStream,
    packet: Packet,
    buffer: Option<&mut Buffer>,
) -> anyhow::Result<()> {
    match buffer {
        Some(buffer) => {
            let packet = buffer.encode(&packet);
            send.write_all(packet).await?;
        }
        None => {
            let packet = encode(&packet);
            send.write_all(packet.as_slice()).await?;
        }
    };
    send.finish()?;

    Ok(())
}

#[tracing::instrument(skip(recv, buffer))]
pub async fn receive_packet(
    mut recv: RecvStream,
    buffer: Option<&mut Buffer>,
) -> anyhow::Result<Packet> {
    let packet = recv.read_to_end(64).await?;
    let packet: Packet = match buffer {
        Some(buffer) => buffer.decode(packet.as_slice())?,
        None => decode(packet.as_slice())?,
    };
    Ok(packet)
}
