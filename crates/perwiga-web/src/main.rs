use std::{error::Error, io, net::SocketAddr, path::PathBuf};

use clap::Parser;
use perwiga_core::Store;

#[derive(Parser)]
#[command(
    name = "perwiga-web",
    version,
    about = "Localhost UAT interface for Arknights: Endfield"
)]
struct Arguments {
    /// Explicit SQLite database path.
    #[arg(long)]
    database: PathBuf,

    /// Loopback address used by the local UAT server.
    #[arg(long, default_value = "127.0.0.1:5178")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();
    perwiga_web::validate_bind_address(arguments.bind)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;

    let store = Store::open(&arguments.database)?;
    let application = perwiga_web::router_with_store(store)?;
    let listener = tokio::net::TcpListener::bind(arguments.bind).await?;

    println!("Perwiga Endfield UAT: http://{}", arguments.bind);
    println!("SQLite database: {}", arguments.database.display());

    axum::serve(listener, application)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
