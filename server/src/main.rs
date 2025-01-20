// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]
use tracing_subscriber::EnvFilter;

type InboundEvent = protocol::client_to_server::ClientToServerEvent;
type OutboundEvent = protocol::server_to_client::ServerToClientEvent;

const ADDR: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 54345;

const SERVER_MAX_CONNECTION: u16 = 2;

mod server;
use server::Server;

mod game;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("debug"))
        .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
        .event_format(
            tracing_subscriber::fmt::format()
                .compact()
                .with_source_location(true),
        )
        .init();

    Server::new(ADDR, DEFAULT_PORT, SERVER_MAX_CONNECTION)?
        .run()
        .await
}
