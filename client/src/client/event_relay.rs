use super::{InboundEvent, OutboundEvent, DISCONNECTED_EV_ID};
use bevy::prelude::{debug, info};
use protocol::WithMetadata;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::sync::CancellationToken;

type TcpStreamWrapper =
    bincode_io::TcpStreamWrapper<WithMetadata<InboundEvent>, WithMetadata<OutboundEvent>>;

pub struct EventRelay {
    stream: TcpStreamWrapper,
    out_rx: mpsc::UnboundedReceiver<WithMetadata<OutboundEvent>>,
    in_tx: mpsc::UnboundedSender<WithMetadata<InboundEvent>>,
    shutdown_token: CancellationToken,
}

impl Drop for EventRelay {
    fn drop(&mut self) {
        info!("dropping EventRelay");
    }
}

const INTERNAL_DISCONNECTED_EV: WithMetadata<InboundEvent> = WithMetadata {
    kind: protocol::EventKind::Request,
    id: DISCONNECTED_EV_ID,
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
            stream: TcpStreamWrapper::new(stream, 1024),
            out_rx,
            in_tx,
            shutdown_token,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            debug!("entering select");
            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
                    info!("received a shutdown token");
                    break;
                }

                Ok(_) = self.stream.readable() => {
                    self.relay_inbound_ev()?;
                }

                Some(outbound_ev) = self.out_rx.recv() => {
                    self.stream.write(&outbound_ev).await?;
                }
            }
        }

        info!("finish relaying events");
        Ok(())
    }

    fn relay_inbound_ev(&mut self) -> anyhow::Result<()> {
        match self.stream.try_read() {
            Err(e) if e.would_block() => {
                debug!("relaying inbound ev: exiting due to WouldBlock");
                Ok(())
            }
            Err(e) => {
                self.in_tx.send(INTERNAL_DISCONNECTED_EV)?;
                Err(e.into())
            }
            Ok(ev) => {
                info!("read: {:?}", ev);

                self.in_tx.send(ev)?;
                Ok(())
            }
        }
    }
}
