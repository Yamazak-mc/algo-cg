use log::{error, info};
use protocol::WithMetadata;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, TcpStream},
    sync::Semaphore,
};

use super::{InboundEvent, OutboundEvent};

pub struct Server {
    socket: Option<TcpSocket>,
    port: u16,
    semaphore: Arc<Semaphore>,
}

impl Server {
    pub fn new(addr: &str, port: u16, max_connections: u16) -> anyhow::Result<Self> {
        let socket = TcpSocket::new_v4()?;
        let addr = format!("{}:{}", addr, port).parse()?;
        socket.bind(addr)?;

        let ret = Self {
            socket: Some(socket),
            port,
            semaphore: Arc::new(Semaphore::new(max_connections.into())),
        };
        Ok(ret)

        // let listener = socket.listen(1024).unwrap();
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let listener = self
            .socket
            .take()
            .expect("socket should exist")
            .listen(1024)?;

        info!("Server listening on port {}", self.port);

        loop {
            let (stream, socket_addr) = listener.accept().await.unwrap();
            info!("connected to: {}", socket_addr);

            tokio::spawn(async move {
                Connection::new(stream, socket_addr).run().await;
            });
        }
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
