use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use miniscop::networking::{receive_packet, send_packet, Packet};
use quinn::{rustls, ClientConfig, Connection, Endpoint};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::lookup_host;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

// Resources
/// This resource keeps the async server connection alive.
#[derive(Resource)]
pub(crate) struct ServerConnection {
    runtime: Runtime,
    pub connection_handle:
        JoinHandle<anyhow::Result<(Endpoint, Connection, JoinHandle<()>, JoinHandle<()>)>>,
    pub to_client: Sender<Packet>,
    pub from_server: Receiver<Packet>,
}
// Todo: Add reconnecting support
impl ServerConnection {
    /// Try to gracefully disconnect from the server.
    ///
    /// You can force a disconnection by removing the ServerConnection resource.
    #[tracing::instrument(skip(self))]
    pub(crate) fn try_disconnect(&mut self) -> anyhow::Result<()> {
        self.to_client.try_send(Packet::ClientDisconnect(None))?;

        let connect_to_server_output = self.runtime.block_on(&mut self.connection_handle)?;
        match connect_to_server_output {
            Ok((_endpoint, connection, bevy_handle, server_handle)) => {
                self.runtime.block_on(bevy_handle)?;
                self.runtime.block_on(server_handle)?;
                self.runtime.block_on(connection.closed());
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Client cannot disconnect due to an error that was already reported."
                ));
            }
        }
        Ok(())
    }
}

// Systems
pub(crate) fn setup_client_runtime(mut commands: Commands) {
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (to_client, from_bevy) = mpsc::channel::<Packet>(128);
    let (to_bevy, from_server) = mpsc::channel::<Packet>(128);
    // Connect to server
    let connection_handle = runtime.spawn(async move {
        match connect_to_server(from_bevy, to_bevy).await {
            Ok(output) => Ok(output),
            Err(e) => {
                // Report the error immediately, rather than waiting for the join handle to read it
                error!("Unable to connect to server: {e:#?}");
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

/// A system that tries to disconnect from the server when the window is closed.
pub(crate) fn stop_client_runtime_on_window_close(
    mut commands: Commands,
    server_connection: Option<ResMut<ServerConnection>>,
    mut window_close_requested: EventReader<WindowCloseRequested>,
) {
    if let Some(mut server_connection) = server_connection {
        for _ in window_close_requested.read() {
            match server_connection.try_disconnect() {
                Ok(()) => {
                    info!("Successfully disconnected from server.");
                }
                Err(e) => {
                    error!("Unable to disconnect from server: {e:#?}");
                }
            }
            commands.remove_resource::<ServerConnection>();
        }
    }
}

// Non-system functions
#[tracing::instrument(skip(from_bevy, to_bevy))]
pub(crate) async fn connect_to_server(
    from_bevy: Receiver<Packet>,
    to_bevy: Sender<Packet>,
) -> anyhow::Result<(Endpoint, Connection, JoinHandle<()>, JoinHandle<()>)> {
    let endpoint = Endpoint::client(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))?;

    // Todo: Let player choose server to connect to
    const URL: &str = "miniscop.twilightparadox.com";
    let server_address = lookup_host((URL, 4433))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve the server's IP address"))?;
    info!("Connecting to {server_address}");

    // Rustls needs to get the computer's crypto provider first, or else Quinn will panic.
    // https://github.com/quinn-rs/quinn/issues/2275
    rustls::client::ClientConfig::builder();

    let connection = endpoint
        .connect_with(ClientConfig::with_platform_verifier(), server_address, URL)
        .map_err(|e| anyhow::anyhow!("Connection configuration error: {e:?}"))?
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to server: {e:?}"))?;
    info!("Connected to {server_address}");

    let connection_handle = connection.clone();
    let bevy_task = tokio::spawn(async move {
        if let Err(e) = await_bevy_packets(connection_handle, from_bevy).await {
            error!("Packet sending error: {e:#?}. No longer sending packets.");
        }
    });

    let connection_handle = connection.clone();
    let server_task = tokio::spawn(async move {
        if let Err(e) = await_server_packets(connection_handle, to_bevy.clone()).await {
            error!("Packet receiving error: {e:#?}. No longer receiving packets.");
        }
        let _ = to_bevy.send(Packet::ClientDisconnect(None)).await;
    });

    Ok((endpoint, connection, bevy_task, server_task))
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
                error!("Failed to send packet to server: {e:#?}");
            }
        });

        if packet == Packet::ClientDisconnect(None) {
            return Ok(());
        }
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
                    if let Err(TrySendError::Full(_)) = to_bevy_clone.try_send(packet) {
                        error!(
                            "Failed to send packet to Bevy because channel is full.\nIf you see this, please report this error so the dev can consider increasing channel size.\nAwaiting space in the channel..."
                        );
                        if let Err(_) = to_bevy_clone.send(packet).await {
                            info!("Channel to Bevy closed, async loop will close next iteration");
                        }
                    }
                }
                Err(e) => error!("Failed to receive packet from server: {e:?}"),
            }
        });
    }
    Ok(())
}
