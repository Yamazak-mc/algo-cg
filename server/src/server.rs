use super::{InboundEvent, OutboundEvent};
use crate::game::{ServerInternalEvent, WaitingRoom};
use algo_core::player::PlayerId;
use async_write_bincode::AsyncWriteSerdeBincode as _;
use anyhow::{bail, Context};
use protocol::WithMetadata;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpSocket, TcpStream},
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        Semaphore,
    },
};
use tracing::{error, info, warn};

macro_rules! unexpected_event {
    ($event:expr $(,)?) => {
        bail!("server internal error: unexpected event: {:?}", $event)
    };
}

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
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let listener: TcpListener = self
            .socket
            .take()
            .expect("socket should exist")
            .listen(1024)?;

        // Start the game server
        let (tx, rx) = mpsc::unbounded_channel();
        let mut game_server_task = tokio::spawn(async move { WaitingRoom::new(rx).run().await });

        info!("Server listening on port {}", self.port);

        loop {
            tokio::select! {
                res = &mut game_server_task => {
                    let res = res?;
                    warn!("game server closed: {:?}", res);
                    return res;
                }
                Ok((stream, socket_addr)) = listener.accept(), if self.semaphore.available_permits() > 0 => {
                    info!("connected to: {}", socket_addr);

                    let semaphore = self.semaphore.clone();
                    let tx_cloned = tx.clone();

                    tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.unwrap();

                        if let Err(e) = PendingConnection::new(stream, socket_addr, tx_cloned)
                            .run()
                            .await
                        {
                            warn!("disconnected from peer {}: {}", socket_addr, e);
                        }
                    });
                }
            }
        }
    }
}

struct PendingConnection {
    stream: TcpStream,
    socket_addr: SocketAddr,
    internal_tx: UnboundedSender<ServerInternalEvent>,
}

impl PendingConnection {
    fn new(
        stream: TcpStream,
        socket_addr: SocketAddr,
        internal_tx: UnboundedSender<ServerInternalEvent>,
    ) -> Self {
        Self {
            stream,
            socket_addr,
            internal_tx,
        }
    }

    async fn run(mut self) -> anyhow::Result<()> {
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
                    let data: WithMetadata<InboundEvent> = bincode::deserialize(&buf[0..n])?;
                    info!("{:?}", data);

                    match &data.event {
                        InboundEvent::RequestJoin => {
                            // Create a channel to communicate with the game server.
                            let (tx, mut rx) = mpsc::unbounded_channel();

                            // Send a request to join the game.
                            self.internal_tx
                                .send(ServerInternalEvent::RequestJoin(tx))?;

                            let resp = rx.recv().await.context("server internal error")?;
                            match resp {
                                ServerInternalEvent::RequestJoinAccepted(info) => {
                                    let player_id = info.player_id;
                                    stream
                                        .write_bincode(&data.response_to(
                                            OutboundEvent::RequestJoinAccepted(info),
                                        ))
                                        .await?;

                                    return Connection::from_pending(self, rx, player_id)
                                        .relay_events()
                                        .await;
                                }
                                unexpected => unexpected_event!(unexpected),
                            }
                        }
                        unexpected => {
                            warn!("ignoring unexpected event: {:?}", unexpected);
                            continue;
                        }
                    }
                }
            }
        }

        info!("disconnected from: {}", self.socket_addr);
        Ok(())
    }
}

struct Connection {
    stream: TcpStream,
    _socket_addr: SocketAddr,
    internal_tx: UnboundedSender<ServerInternalEvent>,
    internal_rx: UnboundedReceiver<ServerInternalEvent>,
    player_id: PlayerId,
}

impl Connection {
    fn from_pending(
        conn: PendingConnection,
        internal_rx: UnboundedReceiver<ServerInternalEvent>,
        player_id: PlayerId,
    ) -> Self {
        Self {
            stream: conn.stream,
            _socket_addr: conn.socket_addr,
            internal_tx: conn.internal_tx,
            internal_rx,
            player_id,
        }
    }

    async fn relay_events(mut self) -> anyhow::Result<()> {
        let mut buf = [0; 1024];

        loop {
            tokio::select! {
                readable = self.stream.readable() => {
                    readable?;

                    match self.stream.try_read(&mut buf) {
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            self.notify_disconnected()?;
                            return Err(e.into());
                        },
                        Ok(0) => {
                            self.notify_disconnected()?;
                            bail!("peer shutdown");
                        },
                        Ok(n) => {
                            let ev = bincode::deserialize(&buf[0..n])?;
                            self.internal_tx.send(ServerInternalEvent::In(self.player_id, ev))?;
                        }
                    }
                }
                received = self.internal_rx.recv() => {
                    let received = received.context("server internal error")?;

                    match received {
                        ServerInternalEvent::Out(ev) => {
                            self.stream.write_bincode(&ev).await?;
                        }
                        unexpected => unexpected_event!(unexpected),
                    }
                }
            }
        }
    }

    fn notify_disconnected(&self) -> Result<(), mpsc::error::SendError<ServerInternalEvent>> {
        self.internal_tx
            .send(ServerInternalEvent::ConnectionLost(self.player_id))
    }
}
