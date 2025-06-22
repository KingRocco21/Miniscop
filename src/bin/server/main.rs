use anyhow;
use clap::Parser;
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
    #[clap(short, long, default_value = "100")]
    max_players: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .unwrap();

    let args = Args::parse();

    let certificate_chain = CertificateDer::pem_file_iter(args.certificate)?
        .map(|cert| cert.unwrap())
        .collect();
    let key = PrivateKeyDer::from_pem_file(args.key)?;
    let server_config = ServerConfig::with_single_cert(certificate_chain, key)?;
    let endpoint = Endpoint::server(server_config, args.address)?;

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
            incoming.retry().unwrap();
        } else {
            info!("Accepting connection from {}", incoming.remote_address());
            tokio::spawn(async move {
                if let Err(e) = handle_connection(incoming).await {
                    error!("Connection failed: {:?}", e)
                }
            });
        }
    }

    Ok(())
}

#[tracing::instrument(name = "connection", skip(incoming), fields(address = %incoming.remote_address()))]
async fn handle_connection(incoming: quinn::Incoming) -> anyhow::Result<()> {
    let connection = incoming.await?;
    info!("Established connection");

    let (mut send, mut recv) = match connection.open_bi().await {
        Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
            info!("Connection closed by client");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
        Ok(s) => s,
    };

    send.write_all("Hello from server".as_bytes()).await?;
    send.finish()?;

    let mut buf = [0u8; "Hello from client".len()];
    recv.read_exact(&mut buf).await?;
    let msg = String::from_utf8_lossy(&buf);
    info!("Received: {}", msg);

    Ok(())
}
