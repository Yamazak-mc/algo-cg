use super::{InboundEvent, OutboundEvent};
use bevy::prelude::info;
use protocol::WithMetadata;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

pub struct EventRelay {
    stream: TcpStream,
    out_rx: mpsc::UnboundedReceiver<WithMetadata<OutboundEvent>>,
    in_tx: mpsc::UnboundedSender<WithMetadata<InboundEvent>>,
    buf: [u8; 1024],
    shutdown_token: CancellationToken,
}

const INTERNAL_DISCONNECTED_EV: WithMetadata<InboundEvent> = WithMetadata {
    kind: protocol::EventKind::Request,
    id: protocol::EventId::from_raw(0),
    event: InboundEvent::ServerShutdown,
};

impl EventRelay {
    pub fn new(
        stream: TcpStream,
        out_rx: mpsc::UnboundedReceiver<WithMetadata<OutboundEvent>>,
        in_tx: mpsc::UnboundedSender<WithMetadata<InboundEvent>>,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            stream,
            out_rx,
            in_tx,
            buf: [0; 1024],
            shutdown_token,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
                    info!("received a shutdown token");
                    break;
                },

                outbound_ev = self.out_rx.recv() => {
                    let Some(outbound_ev) = outbound_ev else {
                        // Inbound event sender is dropped.
                        break;
                    };

                    let data = bincode::serialize(&outbound_ev)?;
                    self.stream.write_all(&data).await?;
                }

                readable = self.stream.readable() => {
                    readable?;

                    self.relay_inbound_ev().await?;
                }
            }
        }

        Ok(())
    }

    async fn relay_inbound_ev(&mut self) -> anyhow::Result<()> {
        let inbound_ev = match self.stream.read(&mut self.buf).await {
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(());
            }
            Err(_) | Ok(0) => INTERNAL_DISCONNECTED_EV,
            Ok(n) => bincode::deserialize(&self.buf[0..n])?,
        };

        self.in_tx.send(inbound_ev)?;
        Ok(())
    }
}
