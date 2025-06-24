use anyhow;
use clap::Parser;
use miniscop::networking::Packet::PlayerPosition;
use miniscop::networking::{receive_packet, send_packet};
use quinn::{Endpoint, ServerConfig};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::path::PathBuf;
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
    /// If you increase this past 100, you risk overwhelming your players with packets.
    #[clap(short, long, default_value = "100")]
    max_players: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new())?;

    let args = Args::parse();

    let certificate_chain = CertificateDer::pem_file_iter(args.certificate)?
        .map(|cert| cert.unwrap())
        .collect();
    let key = PrivateKeyDer::from_pem_file(args.key)?;
    let server_config = ServerConfig::with_single_cert(certificate_chain, key)?;
    let endpoint = Endpoint::server(server_config, args.address)?;

    info!("Waiting for connections...");
    while let Some(incoming) = endpoint.accept().await {
        if endpoint.open_connections() > args.max_players {
            info!(
                "Refusing {}. Max player-count was reached.",
                incoming.remote_address()
            );
            incoming.refuse();
        } else if !incoming.remote_address_validated() {
            info!(
                "Requiring {} to validate its address",
                incoming.remote_address()
            );
            incoming.retry()?;
        } else {
            info!("Accepting connection from {}", incoming.remote_address());
            tokio::spawn(async move {
                if let Err(e) = handle_connection(incoming).await {
                    error!("Connection error: {:?}", e)
                }
            });
        }
    }

    Ok(())
}

#[tracing::instrument(skip(incoming), fields(address = %incoming.remote_address()))]
async fn handle_connection(incoming: quinn::Incoming) -> anyhow::Result<()> {
    let connection = incoming.await?;
    info!("Established connection");

    // This loop ends when an error occurs.
    loop {
        let recv = connection.accept_uni().await?;
        let packet = receive_packet(recv).await?;
        match packet {
            PlayerPosition { id, x, y, z } => {
                let new_packet = PlayerPosition {
                    id,
                    x: x + 2.0,
                    y,
                    z,
                };
                let send = connection.open_uni().await?;
                send_packet(send, new_packet).await?;
            }
        }
    }
    // Todo: Identify which client disconnected when they disconnect
}
