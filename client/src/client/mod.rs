use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use protocol::{EventHandlerPlugin, WithMetadata};
use std::{
    net::{IpAddr, SocketAddr},
    thread,
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
};
use tokio_util::sync::CancellationToken;

pub type InboundEvent = protocol::server_to_client::ServerToClientEvent;
pub type OutboundEvent = protocol::client_to_server::ClientToServerEvent;

mod event_relay;
use event_relay::EventRelay;

pub fn client_connection_plugin(app: &mut App) {
    app.add_plugins(EventHandlerPlugin::<InboundEvent, OutboundEvent>::default())
        .add_event::<SpawnClientResult>()
        .add_event::<CancelSpawnClientEvent>()
        .add_event::<ReceivedRequest>()
        .add_event::<ReceivedResponse>()
        .add_systems(Update, ConnectionHandle::system)
        .add_systems(Update, CancelConnTask::system);
}

/// Tries initializing a client.
///
/// When a result becomes ready, [`SpawnClientResult`] is sent.
/// Additionally, if succeeded in connecting to the server,
/// [`Client`] resource is added to the world.
///
/// [`client_connection_plugin`] is required.
pub fn spawn_client(commands: &mut Commands, addr: IpAddr, port: u16) {
    let conn_handle = connect(addr, port);
    commands.spawn(ConnectionHandle(Some(conn_handle)));
}

#[derive(Debug, Event)]
pub struct SpawnClientResult(pub Result<(), Box<str>>);

#[derive(Event)]
pub struct CancelSpawnClientEvent;

pub type EventHandler = protocol::EventHandler<
    mpsc::UnboundedReceiver<WithMetadata<InboundEvent>>,
    mpsc::UnboundedSender<WithMetadata<OutboundEvent>>,
>;

pub type ReceivedRequest = protocol::ReceivedRequest<InboundEvent>;
pub type ReceivedResponse = protocol::ReceivedResponse<InboundEvent>;

#[derive(Resource)]
struct ShutdownClientOnDrop {
    shutdown_token: CancellationToken,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    /// Send signal through this to tell runtime to shutdown.
    oneshot_tx: Option<oneshot::Sender<()>>,
}

impl Drop for ShutdownClientOnDrop {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl ShutdownClientOnDrop {
    fn shutdown(&mut self) {
        info!("shutting down the client...");
        self.oneshot_tx.take().unwrap().send(()).ok();
        self.thread_handle.take().unwrap().join().ok();
        info!("client is shut down");
    }
}

fn connect(addr: IpAddr, port: u16) -> ConnectionHandleImpl {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime");

    let (out_tx, out_rx) = mpsc::unbounded_channel();
    let (in_tx, in_rx) = mpsc::unbounded_channel();
    let (conn_res_tx, conn_res_rx) = oneshot::channel();

    let shutdown_token = CancellationToken::new();
    let shutdown_token_cloned = shutdown_token.clone();

    let cancel_token = shutdown_token.child_token();
    let cancel_token_cloned = cancel_token.clone();

    let (oneshot_tx, oneshot_rx) = oneshot::channel();

    let thread_handle = thread::spawn(move || {
        // MAYBE: handle panics?

        rt.block_on(async move {
            let socket_addr = SocketAddr::new(addr, port);

            let stream = match connect_tcp(socket_addr, cancel_token_cloned).await {
                Ok(v) => {
                    conn_res_tx
                        .send(Ok(()))
                        .expect("failed to send connection result");
                    v
                }
                Err(e) => {
                    if !conn_res_tx.is_closed() {
                        conn_res_tx
                            .send(Err(e.to_string().into()))
                            .expect("failed to send connection result");
                    }
                    return Err(e.into());
                }
            };
            let mut event_relay = EventRelay::new(stream, out_rx, in_tx, shutdown_token_cloned);

            tokio::select! {
                ret = event_relay.run() => ret,
                _ = oneshot_rx => Err(anyhow::anyhow!("interrupted")),
            }
        })
        .ok(); // FIXME: Not ok
    });

    let ev_handler = EventHandler::new(in_rx, out_tx);
    let shutdown_handler = ShutdownClientOnDrop {
        thread_handle: Some(thread_handle),
        oneshot_tx: Some(oneshot_tx),
    };

    ConnectionHandleImpl {
        client: Some((ev_handler, shutdown_handler)),
        conn_res_rx: Some(conn_res_rx),
        cancel_token,
    }
}

#[derive(Component)]
struct ConnectionHandle(Option<ConnectionHandleImpl>);

impl ConnectionHandle {
    /// Checks if the TcpStream is established and if so,
    /// add client resources to the world and send [`SpawnClientResult`] event.
    fn system(
        mut commands: Commands,
        mut cancel_ev_rdr: EventReader<CancelSpawnClientEvent>,
        mut conn_handle: Query<(Entity, &mut Self)>,
    ) {
        let Ok((entity, mut conn_handle)) = conn_handle.get_single_mut() else {
            return;
        };

        if !cancel_ev_rdr.is_empty() {
            // Cancel the connection
            let conn_handle = conn_handle.0.take().unwrap();

            let thread_pool = AsyncComputeTaskPool::get();
            let res = thread_pool.spawn(async move { conn_handle.cancel() });
            commands.spawn(CancelConnTask(res));

            // Cleanup
            cancel_ev_rdr.clear();
            commands.entity(entity).despawn();

            return;
        }

        let conn_handle = conn_handle.0.as_mut().unwrap();
        let Some(conn_res) = conn_handle.check_progress() else {
            return;
        };

        // The result of the connection is finalized at this point.

        let spawn_client_res = match conn_res {
            Ok((ev_handler, shutdown_handler)) => {
                commands.insert_resource(ev_handler);
                commands.insert_resource(shutdown_handler);
                Ok(())
            }
            Err(e) => Err(e),
        };
        commands.send_event(SpawnClientResult(spawn_client_res));

        // Cleanup
        commands.entity(entity).despawn();
    }
}

struct ConnectionHandleImpl {
    client: Option<(EventHandler, ShutdownClientOnDrop)>,
    conn_res_rx: Option<oneshot::Receiver<Result<(), Box<str>>>>,
    cancel_token: CancellationToken,
}

impl Drop for ConnectionHandleImpl {
    fn drop(&mut self) {
        if self.client.is_some() {
            if let Some(ref mut conn_res_rx) = self.conn_res_rx {
                conn_res_rx.close();
            }
            self.cancel_token.cancel();
            drop(self.client.take().unwrap());
        }
    }
}

impl ConnectionHandleImpl {
    fn cancel(mut self) -> SpawnClientResult {
        self.cancel_token.cancel();

        let ret = match self.conn_res_rx.take().unwrap().blocking_recv() {
            Ok(Ok(_)) => Err("connection is cancelled".into()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e.to_string().into()),
        };

        SpawnClientResult(ret)
    }

    /// Checks if any progress on connection is made.
    ///
    /// If the returned value is `Some`,
    /// it means that the connection is either succeeded or failed.
    ///
    /// Returns `None` if no progress are made.
    fn check_progress(&mut self) -> Option<Result<(EventHandler, ShutdownClientOnDrop), Box<str>>> {
        match self.conn_res_rx.as_mut().unwrap().try_recv() {
            Ok(res) => Some(res.map(|_| self.client.take().unwrap())),
            Err(_) => None,
        }
    }
}

#[derive(Component)]
struct CancelConnTask(bevy::tasks::Task<SpawnClientResult>);

impl CancelConnTask {
    fn system(mut commands: Commands, mut tasks: Query<(Entity, &mut CancelConnTask)>) {
        use bevy::tasks::{block_on, futures_lite::future::poll_once};

        for (entity, mut task) in &mut tasks {
            if let Some(res) = block_on(poll_once(&mut task.0)) {
                commands.send_event(res);
                commands.entity(entity).despawn();
            }
        }
    }
}

async fn connect_tcp(
    socket_addr: SocketAddr,
    cancel_token: CancellationToken,
) -> Result<TcpStream, ConnectTcpError> {
    let mut intvl = 0;
    let mut attempts = 0;
    let max_attempts = 4;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(intvl)) => {
                match TcpStream::connect(socket_addr).await {
                    Ok(stream) => {
                        return Ok(stream);
                    }
                    Err(_) => {
                        attempts += 1;
                        if attempts >= max_attempts {
                            return Err(ConnectTcpError::Timedout);
                        }

                        intvl = next_backoff_intvl(intvl);
                        info!("failed to connect, retrying in {:?}", intvl);
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                return Err(ConnectTcpError::Cancelled);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ConnectTcpError {
    #[error("connection is cancelled")]
    Cancelled,
    #[error("timed out")]
    Timedout,
}

fn next_backoff_intvl(prev_intvl: u64) -> u64 {
    if prev_intvl == 0 {
        1
    } else {
        prev_intvl * 2
    }
}
