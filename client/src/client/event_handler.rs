use bevy::prelude::*;
use protocol::{EventBox, EventKind, NextEventId, WithMetadata};
use std::marker::PhantomData;
use tokio::sync::mpsc::{error::SendError, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Resource)]
pub struct EventHandler<I, O> {
    in_rx: UnboundedReceiver<WithMetadata<I>>,
    out_tx: UnboundedSender<WithMetadata<O>>,
    pub storage: EventBox<I>,
    next_id: NextEventId,
}

impl<I, O> EventHandler<I, O> {
    pub fn new(
        in_rx: UnboundedReceiver<WithMetadata<I>>,
        out_tx: UnboundedSender<WithMetadata<O>>,
    ) -> Self {
        Self {
            in_rx,
            out_tx,
            storage: EventBox::default(),
            next_id: NextEventId::default(),
        }
    }

    pub fn send_request(
        &mut self,
        event: O,
    ) -> Result<protocol::EventId, SendError<WithMetadata<O>>> {
        debug!("send_request");
        let id = self.next_id.produce();
        self.out_tx
            .send(WithMetadata {
                kind: EventKind::Request,
                id,
                event,
            })
            .map(|_| id)
    }

    pub fn send_response(
        &mut self,
        id: protocol::EventId,
        event: O,
    ) -> Result<(), SendError<WithMetadata<O>>> {
        debug!("send_response");
        // TODO: Validate EventId
        self.out_tx.send(WithMetadata {
            kind: EventKind::Response,
            id,
            event,
        })
    }
}

pub struct EventHandlerPlugin<I, O> {
    _marker: PhantomData<fn(&I, &O)>,
}

impl<I, O> Default for EventHandlerPlugin<I, O> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<I, O> Plugin for EventHandlerPlugin<I, O>
where
    I: Send + Sync + 'static + std::fmt::Debug,
    O: Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_event::<ReceivedRequest<I>>()
            .add_event::<ReceivedResponse<I>>()
            .add_systems(
                FixedUpdate,
                recv_inbound_events::<I, O>.run_if(resource_exists::<EventHandler<I, O>>),
            );
    }
}

#[derive(Debug, Clone, Event)]
pub struct ReceivedEvent<K, E>(pub protocol::EventId, PhantomData<fn(&K, &E)>);

impl<K, E> From<protocol::EventId> for ReceivedEvent<K, E> {
    fn from(id: protocol::EventId) -> Self {
        Self(id, PhantomData)
    }
}

impl<K, E> ReceivedEvent<K, E> {
    pub fn id(&self) -> protocol::EventId {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventKindRequest;

pub type ReceivedRequest<E> = ReceivedEvent<EventKindRequest, E>;

#[derive(Debug, Clone, Default)]
pub struct EventKindResponse;

pub type ReceivedResponse<E> = ReceivedEvent<EventKindResponse, E>;

fn recv_inbound_events<I, O>(
    mut ev_handler: ResMut<EventHandler<I, O>>,
    // mut request_w: EventWriter<ReceivedRequest<I>>,
    // mut response_w: EventWriter<ReceivedResponse<I>>,
    mut commands: Commands,
) where
    I: Send + Sync + 'static + std::fmt::Debug,
    O: Send + Sync + 'static,
{
    let Ok(event) = ev_handler.in_rx.try_recv() else {
        return;
    };
    debug!("handler received: {:?}", event);

    let (kind, id) = event.metadata();

    if let Some(ev) = ev_handler.storage.store(event) {
        warn!("EventId collision occured: {:?}, {:?}, {:?}", kind, id, ev);
    }

    match kind {
        EventKind::Request => {
            // request_w.send(id.into());
            commands.trigger(ReceivedRequest::<I>::from(id));
        }
        EventKind::Response => {
            // response_w.send(id.into());
            commands.trigger(ReceivedResponse::<I>::from(id));
        }
    }
}
