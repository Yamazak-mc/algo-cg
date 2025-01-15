// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

use std::net::SocketAddr;

use log::{error, info};
use protocol::WithMetadata;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, TcpStream},
};

type InboundEvent = protocol::client_to_server::ClientToServerEvent;
type OutboundEvent = protocol::server_to_client::ServerToClientEvent;

const ADDR: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 54345;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let socket = TcpSocket::new_v4().unwrap();
    let addr = format!("{}:{}", ADDR, DEFAULT_PORT).parse().unwrap();
    socket.bind(addr).unwrap();

    info!("Server listening on port {}", addr.port());

    let listener = socket.listen(1024).unwrap();

    loop {
        let (stream, socket_addr) = listener.accept().await.unwrap();
        info!("connected to: {}", socket_addr);

        tokio::spawn(async move {
            Connection::new(stream, socket_addr).run().await;
        });
    }
}

struct Connection {
    stream: TcpStream,
    socket_addr: SocketAddr,
}

impl Connection {
    fn new(stream: TcpStream, socket_addr: SocketAddr) -> Self {
        Self {
            stream,
            socket_addr,
        }
    }

    async fn run(&mut self) {
        let stream = &mut self.stream;
        let mut buf = vec![0; 1024];

        loop {
            let Ok(_) = stream.readable().await else {
                break;
            };

            match stream.try_read(&mut buf) {
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    error!("read error: {}", e);
                    break;
                }
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    // Received an event
                    let data: WithMetadata<InboundEvent> =
                        bincode::deserialize(&buf[0..n]).unwrap();
                    info!("{:?}", data);

                    match &data.event {
                        InboundEvent::RequestJoin => {
                            // TODO
                            // For now, just accept all requests
                            let resp = data.response_to(OutboundEvent::RequestJoinAccepted);
                            let resp = bincode::serialize(&resp).unwrap();
                            stream.write_all(&resp).await.unwrap();
                        }
                    }
                }
            }
        }

        info!("disconnected from: {}", self.socket_addr);
    }
}
