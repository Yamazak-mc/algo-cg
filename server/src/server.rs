use std::{net::SocketAddr, thread};

use bevy::prelude::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpSocket, TcpStream},
    sync::oneshot,
};

#[derive(Resource)]
pub struct ServerHandle {
    inner: Option<ServerHandleImpl>,
}

impl ServerHandle {
    /// Starts the server and returns a handle for it.
    pub fn start(addr: &'static str) -> Self {
        let server_handle = ServerHandleImpl::start(addr);

        Self {
            inner: Some(server_handle),
        }
    }

    /// Returns `true` if the server is on.
    pub fn is_alive(&self) -> bool {
        self.inner.is_some()
    }

    /// Shutdowns the server.
    ///
    /// If the server is not active, this function does nothing.
    pub fn shutdown(&mut self) {
        if self.is_alive() {
            self.inner.take().unwrap().shutdown();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(server) = self.inner.take() {
            server.shutdown();
        }
    }
}

struct ServerHandleImpl {
    shutdown_signal: oneshot::Sender<()>,
    thread_handle: thread::JoinHandle<()>,
}

impl ServerHandleImpl {
    fn start(addr: &'static str) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .unwrap();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let thread_handle = thread::spawn(move || {
            let _ = rt.block_on(async move {
                let socket = TcpSocket::new_v4().unwrap();
                let addr = addr.parse().unwrap();
                socket.bind(addr).unwrap();

                info!("Server listening on port {}", addr.port());

                let listener = socket.listen(1024).unwrap();
                let server_impl = async move {
                    loop {
                        let (mut stream, socket_addr) = listener.accept().await.unwrap();
                        info!("socket_addr = {}", socket_addr);

                        let mut buf = vec![0; 1024];
                        let n = stream.read(&mut buf).await.unwrap();

                        // TEST: Convert received bytes to string
                        info!("Received: {}", String::from_utf8_lossy(&buf[0..n]));

                        // TEST: TODO: Send some message back
                        stream
                            .write_all("HTTP/1.1 200 OK".as_bytes())
                            .await
                            .unwrap();
                    }
                };

                tokio::select! {
                    _ = server_impl => {}
                    _ = shutdown_rx => {
                        info!("shutting down server...");
                    }
                }
            });
        });

        Self {
            shutdown_signal: shutdown_tx,
            thread_handle,
        }
    }

    fn shutdown(self) {
        self.shutdown_signal.send(()).ok();
        self.thread_handle.join().ok();
    }
}

// struct Server {
//     listener: TcpListener,
//     game_state: GameState,
//     clients: dashmap::DashMap<SocketAddr, Client>,
// }

// struct Client {
//     stream: TcpStream,
//     player_state: PlayerState,
// }

use std::sync::{Arc, Mutex};

struct Server {
    listener: TcpListener,
    game_state: GameState,
    clients: dashmap::DashMap<SocketAddr, Client>,
}

struct GameState {
    data: Vec<u8>,
}

struct Client {
    stream: TcpStream,
    player: Option<Entity>,

}
