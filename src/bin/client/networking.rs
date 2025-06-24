use bevy::prelude::*;
use miniscop::networking::{receive_packet, send_packet, Packet};
use quinn::{rustls, ClientConfig, Connection, Endpoint, VarInt};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::lookup_host;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

// Resources
/// This resource keeps the async server connection alive.
#[derive(Resource)]
pub(crate) struct ServerConnection {
    runtime: Runtime,
    connection_handle: JoinHandle<anyhow::Result<(Endpoint, Connection)>>,
    to_client: Sender<Packet>,
    from_server: Receiver<Packet>,
}
impl ServerConnection {
    /// Try to gracefully disconnect from the server, printing info if the method fails.
    ///
    /// You can force a disconnection by removing the ServerConnection resource.
    pub(crate) fn try_disconnect(&mut self) {
        match self.runtime.block_on(&mut self.connection_handle) {
            Ok(output) => match output {
                Ok((endpoint, connection)) => {
                    connection.close(VarInt::from_u32(0), b"Client disconnected normally");
                    self.runtime.block_on(endpoint.wait_idle());
                }
                Err(e) => {
                    info!(
                        "Client will not disconnect due to an error that was already reported: {}",
                        e
                    )
                }
            },
            Err(e) => error!(
                "Failed to await connection handle, client will not disconnect: {}",
                e
            ),
        }
    }
}

// Systems
pub(crate) fn setup_client_runtime(mut commands: Commands) {
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (to_client, from_bevy) = mpsc::channel::<Packet>(128);
    let (to_bevy, from_server) = mpsc::channel::<Packet>(128);
    // Connect to server
    let connection_handle: JoinHandle<anyhow::Result<(Endpoint, Connection)>> =
        runtime.spawn(async move {
            match connect_to_server(from_bevy, to_bevy).await {
                Ok(output) => Ok(output),
                Err(e) => {
                    // Report the error immediately, rather than waiting for the join handle to read it
                    error!("Connection error: {}", e);
                    Err(e)
                }
            }
        });

    commands.insert_resource(ServerConnection {
        runtime,
        connection_handle,
        to_client,
        from_server,
    });
}

pub(crate) fn stop_client_runtime(
    mut commands: Commands,
    mut server_connection: ResMut<ServerConnection>,
) {
    server_connection.try_disconnect();
    commands.remove_resource::<ServerConnection>();
}

// Non-system functions
#[tracing::instrument(skip(from_bevy, to_bevy))]
pub(crate) async fn connect_to_server(
    from_bevy: Receiver<Packet>,
    to_bevy: Sender<Packet>,
) -> anyhow::Result<(Endpoint, Connection)> {
    let endpoint = Endpoint::client(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))?;

    // Todo: Let player choose server to connect to
    const URL: &str = "miniscop.twilightparadox.com";
    let server_address = lookup_host((URL, 4433))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve the server's IP address"))?;
    info!("Connecting to {}", server_address);

    // Rustls needs to get the computer's crypto provider first, or else Quinn will panic.
    // https://github.com/quinn-rs/quinn/issues/2275
    rustls::client::ClientConfig::builder();

    let connection = endpoint
        .connect_with(ClientConfig::with_platform_verifier(), server_address, URL)
        .map_err(|e| anyhow::anyhow!("Connection configuration error: {:?}", e))?
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to server: {:?}", e))?;
    info!("Connected to {}", server_address);

    let connection_handle = connection.clone();
    tokio::spawn(async move {
        if let Err(e) = await_bevy_packets(connection_handle, from_bevy).await {
            error!("Packet sending error: {:?}. No longer sending packets.", e);
        }
    });

    let connection_handle = connection.clone();
    tokio::spawn(async move {
        if let Err(e) = await_server_packets(connection_handle, to_bevy).await {
            error!(
                "Packet receiving error: {:?}. No longer receiving packets.",
                e
            );
        }
    });

    Ok((endpoint, connection))
}

/// Awaits packets from Bevy to send to the server.
#[tracing::instrument(skip(connection_handle, from_bevy))]
pub(crate) async fn await_bevy_packets(
    connection_handle: Connection,
    mut from_bevy: Receiver<Packet>,
) -> anyhow::Result<()> {
    // This loop ends when the channel is closed.
    while let Some(packet) = from_bevy.recv().await {
        // Could not find a way to move the open_uni() future into send_packet(), so we await here.
        // Since streams are "instantaneous to open", this shouldn't fill up the channel.
        let send = connection_handle.open_uni().await?;
        tokio::spawn(async move {
            if let Err(e) = send_packet(send, packet).await {
                error!("Failed to send packet to server: {:?}", e);
            }
        });
    }

    Ok(())
}

/// Awaits packets from the server to send to Bevy.
#[tracing::instrument(skip(connection_handle, to_bevy))]
pub(crate) async fn await_server_packets(
    connection_handle: Connection,
    to_bevy: Sender<Packet>,
) -> anyhow::Result<()> {
    while !to_bevy.is_closed() {
        let recv = connection_handle.accept_uni().await?;
        let to_bevy_clone = to_bevy.clone();

        tokio::spawn(async move {
            match receive_packet(recv).await {
                Ok(packet) => {
                    if let Err(e) = to_bevy_clone.send(packet).await {
                        error!("Failed to send packet to bevy: {:?}", e);
                    }
                }
                Err(e) => error!("Failed to receive packet from server: {:?}", e),
            }
        });
    }
    Ok(())
}
