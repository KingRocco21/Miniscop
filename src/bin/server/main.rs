use anyhow;
use clap::Parser;
use miniscop::networking::{receive_packet, send_packet, Packet};
use quinn::{Connection, Endpoint, ServerConfig};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The file path of your TLS certificate in PEM format.
    /// This can come from a .pem, .cert, or .crt file.
    #[clap(short, long, value_name = "PATH")]
    certificate: PathBuf,
    /// The file path of your TLS private key in PEM format.
    /// This can come from a .pem or .key file.
    #[clap(short, long, value_name = "PATH")]
    key: PathBuf,
    /// An optional IP address and port to use when hosting your server.
    /// This defaults to your computer's IP on port 4433.
    #[clap(short, long, default_value = "127.0.0.1:4433")]
    address: SocketAddr,
    // Todo: Add optional file path to .txt file with banned client IPs
    /// Maximum number of allowed players.
    /// If you increase this past 100, you accept the of risk overwhelming your players with packets and/or running out of memory on your computer.
    #[clap(short, long, default_value = "100")]
    max_players: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new())?;

    let certificate_chain = CertificateDer::pem_file_iter(args.certificate)?
        .map(|cert| cert.unwrap())
        .collect();
    let key = PrivateKeyDer::from_pem_file(args.key)?;
    let server_config = ServerConfig::with_single_cert(certificate_chain, key)?;
    let endpoint = Endpoint::server(server_config, args.address)?;

    // Create packet broadcaster.
    // Capacity is enough to handle all connections sending up to 4 packets at the exact same time.
    let (to_all_connections, _) = broadcast::channel::<Packet>(args.max_players * 4);

    info!("Waiting for connections...");
    while let Some(incoming) = endpoint.accept().await {
        let address = incoming.remote_address();
        if endpoint.open_connections() > args.max_players {
            info!("Refusing {address}. Max player-count was reached.");
            incoming.refuse();
        } else if !incoming.remote_address_validated() {
            info!("Requiring {address} to validate its address");
            incoming.retry()?;
        } else {
            info!("Accepting connection from {address}...");
            match incoming.await {
                Ok(connection) => {
                    let client_id = connection.stable_id() as u64;
                    info!("Established connection. Client ID is {client_id}.");

                    let to_all_connections_clone = to_all_connections.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(connection, to_all_connections_clone.clone()).await
                        {
                            error!("Connection error from {address}: {e:#?}")
                        }
                        let _ = to_all_connections_clone
                            .send(Packet::ClientDisconnect(Some(client_id)));
                    });
                }
                Err(connection_error) => {
                    error!("Failed to connect: {connection_error:?}");
                }
            }
        }
    }

    Ok(())
}

/// This function is essentially the first half of a connection.
///
/// It receives packets from the connection, and broadcasts the packets to every other connection.
///
/// 1. Spawn a task to handle the second half of the connection.
/// 2. Tell the client its ID
/// 3. Await packets from the client in a loop
#[tracing::instrument(skip(connection, to_all_connections), fields(address = %connection.remote_address()
))]
async fn handle_connection(
    connection: Connection,
    to_all_connections: Sender<Packet>,
) -> anyhow::Result<()> {
    // Start a broadcast receiver
    let connection_handle = connection.clone();
    let from_all_connections = to_all_connections.subscribe();
    tokio::spawn(async move {
        if let Err(e) = receive_broadcasts(connection_handle, from_all_connections).await {
            error!("Broadcast receiver error: {e:#?}");
        }
    });

    // Tell the client its ID
    let client_id = connection.stable_id() as u64;
    let send = connection.open_uni().await?;
    let packet = Packet::ClientConnect;
    send_packet(send, packet).await?;

    // Start awaiting packets.
    // This loop ends when an error occurs.
    loop {
        let recv = connection.accept_uni().await?;
        let packet = receive_packet(recv).await?;
        match packet {
            Packet::ClientConnect => {
                return Err(anyhow::anyhow!(
                    "Client tried to send Packet::ClientConnect."
                ));
            }
            Packet::ClientDisconnect(_) => {
                info!("Client is disconnecting.");
                return Ok(());
            }
            Packet::PlayerMovement {
                id,
                x,
                y,
                z,
                velocity_x,
                velocity_y,
                velocity_z,
            } => {
                if id.is_some() {
                    return Err(anyhow::anyhow!("Client sent PlayerMovement with an ID."));
                }
                to_all_connections.send(Packet::PlayerMovement {
                    id: Some(client_id),
                    x,
                    y,
                    z,
                    velocity_x,
                    velocity_y,
                    velocity_z,
                })?;
            }
        }
    }
}

/// This function is essentially the second half of a connection.
///
/// It receives packets from every other connection, and sends the relevant ones to this connection.
#[tracing::instrument(skip(connection, from_all_connections), fields(address = %connection.remote_address()
))]
async fn receive_broadcasts(
    connection: Connection,
    mut from_all_connections: Receiver<Packet>,
) -> anyhow::Result<()> {
    let client_id = connection.stable_id() as u64;

    // Start awaiting packets.
    // This loop must run extremely fast, so if any packets need to be sent, they should be sent in a separate task.
    loop {
        match from_all_connections.recv().await {
            Ok(packet) => match packet {
                Packet::ClientConnect => {
                    panic!(
                        "Server broadcasted a client connect. This should never happen. Please report this to the dev."
                    )
                }
                Packet::ClientDisconnect(id) => {
                    if id.expect("Server broadcasted Packet::ClientDisconnect with no id. This should never happen. Please report this to the dev.") == client_id {
                        return Ok(());
                    } else {
                        let send = connection.open_uni().await?;
                        tokio::spawn(async move {
                            if let Err(e) = send_packet(send, packet).await {
                                error!("Error sending packet: {e:#?}");
                            }
                        });
                    }
                }
                Packet::PlayerMovement { id, .. } => {
                    if id.is_some_and(|id| id != client_id) {
                        let send = connection.open_uni().await?;
                        tokio::spawn(async move {
                            if let Err(e) = send_packet(send, packet).await {
                                error!("Error sending packet: {e:#?}");
                            }
                        });
                    }
                }
            },
            Err(RecvError::Closed) => return Err(anyhow::anyhow!("All broadcasters closed")),
            Err(RecvError::Lagged(skipped_messages)) => {
                error!(
                    "Server is behind by {skipped_messages} messages! Please report this error to the dev so they can consider increasing channel capacity."
                );
            }
        }
    }
}
