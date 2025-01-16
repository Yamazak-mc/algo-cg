// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

type InboundEvent = protocol::client_to_server::ClientToServerEvent;
type OutboundEvent = protocol::server_to_client::ServerToClientEvent;

const ADDR: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 54345;

const SERVER_MAX_CONNECTION: u16 = 2;

mod server;
use server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    Server::new(ADDR, DEFAULT_PORT, SERVER_MAX_CONNECTION)?
        .run()
        .await
}
