use anyhow::{anyhow, Context as _};
use bevy::{
    ecs::world::CommandQueue,
    prelude::*,
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
};
use std::{
    net::{IpAddr, SocketAddr},
    thread,
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt, net::TcpStream, sync::{mpsc, oneshot}
};

pub mod event;
pub use event::Event;

pub mod response;
pub use response::Response;

pub fn client_connection_plugin(app: &mut App) {
    app.add_event::<ConnectionResult>()
        .add_systems(Update, handle_task);
}

pub fn spawn_client(commands: &mut Commands, addr: IpAddr, port: u16) {
    let thread_pool = AsyncComputeTaskPool::get();

    let entity = commands.spawn_empty().id();

    let task = thread_pool.spawn(async move { ClientImpl::connect(addr, port) });

    commands.entity(entity).insert(ConnectionTask(task));
}

fn handle_task(
    mut commands: Commands,
    mut evw: EventWriter<ConnectionResult>,
    mut task: Query<(Entity, &mut ConnectionTask)>,
) {
    if let Ok((entity, mut task)) = task.get_single_mut() {
        if let Some(res) = bevy::tasks::block_on(future::poll_once(&mut task.0)) {
            evw.send(ConnectionResult(match res {
                Ok(v) => {
                    commands.insert_resource(Client { inner: Some(v) });
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }));

            // Cleanup temp entity
            let mut entity = commands.entity(entity);
            entity.clear();
            entity.despawn();
        }
    }
}

#[derive(Component)]
pub struct ConnectionTask(Task<anyhow::Result<ClientImpl>>);

#[derive(Event)]
pub struct ConnectionResult(pub Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>);

#[derive(Resource)]
pub struct Client {
    inner: Option<ClientImpl>,
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(client) = self.inner.take() {
            client.shutdown();
        }
    }
}

// MAYBE: #[derive(Resource)]?
struct ClientImpl {
    ev_tx: mpsc::UnboundedSender<Event>,
    resp_rx: mpsc::UnboundedReceiver<Response>,
    thread_handle: std::thread::JoinHandle<()>,
}

impl ClientImpl {
    fn connect(addr: IpAddr, port: u16) -> anyhow::Result<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .context("failed to build runtime")?;

        let (ev_tx, ev_rx) = mpsc::unbounded_channel();
        let (resp_tx, mut resp_rx) = mpsc::unbounded_channel();

        let thread_handle = thread::spawn(move || {
            // MAYBE: handle panics?

            async fn client_impl(
                mut stream: TcpStream,
                mut ev_rx: mpsc::UnboundedReceiver<Event>,
            ) -> anyhow::Result<()> {
                // TODO
                stream.write("Greetings from client!".as_bytes()).await?;

                while let Some(event) = ev_rx.recv().await {
                    // TODO
                    info!("event received: {:?}", event);
                }

                // Runtime will shutdown when all senders are dropped.
                Ok(())
            }

            rt.block_on(async move {
                let stream = TcpStream::connect(SocketAddr::new(addr, port)).await?;
                resp_tx.send(Response::NotifyEstablished)?;

                client_impl(stream, ev_rx).await
            })
            .ok(); // FIXME: Not ok
        });

        // Exponential back-off
        let mut intvl = 1;
        loop {
            if let Ok(resp) = resp_rx.try_recv() {
                if matches!(resp, Response::NotifyEstablished) {
                    break;
                }
            }

            if intvl >= 8 {
                return Err(anyhow!("failed to connect to the server"));
            }

            info!("server is not yet ready. retrying in {}s", intvl);

            std::thread::sleep(Duration::from_secs(intvl));
            intvl *= 2;
        }

        Ok(Self {
            ev_tx,
            resp_rx,
            thread_handle,
        })
    }

    fn shutdown(self) {
        drop(self.ev_tx);
        drop(self.resp_rx);
        self.thread_handle.join().ok();
    }
}
