use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::common_conditions::on_timer;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, marker::PhantomData, time::Duration};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WithMetadata<E> {
    pub kind: EventKind,
    pub id: EventId,
    pub event: E,
}

impl<E> WithMetadata<E> {
    pub fn response_to<R>(&self, event: R) -> WithMetadata<R> {
        WithMetadata {
            kind: EventKind::Response,
            id: self.id,
            event,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum EventKind {
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[repr(transparent)]
pub struct EventId(u32);

impl EventId {
    pub const fn from_raw(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Resource)]
pub struct EventHandler<R, S>
where
    R: EventReceiver,
    S: EventSender,
{
    in_rx: R,
    out_tx: S,
    requests: BTreeMap<EventId, R::Data>,
    responses: BTreeMap<EventId, R::Data>, // TODO: Track IDs like: Map<EventId, Option<R::Data>>
    next_id: NextEventId,
}

impl<R, S> EventHandler<R, S>
where
    R: EventReceiver,
    S: EventSender,
{
    pub fn new(in_rx: R, out_tx: S) -> Self {
        Self {
            in_rx,
            out_tx,
            requests: BTreeMap::new(),
            responses: BTreeMap::new(),
            next_id: NextEventId(EventId(0)),
        }
    }

    pub fn send(&mut self, event: S::Data) -> anyhow::Result<EventId> {
        let id = self.next_id.next();
        self.out_tx
            .send_event(WithMetadata {
                kind: EventKind::Request,
                id,
                event,
            })
            .map(|_| id)
    }

    pub fn get_request(&mut self, id: EventId) -> Option<R::Data> {
        self.requests.remove(&id)
    }

    pub fn get_response(&mut self, id: EventId) -> Option<R::Data> {
        self.responses.remove(&id)
    }

    fn try_recv(&mut self) -> anyhow::Result<(EventKind, EventId)> {
        let WithMetadata { kind, id, event } = self.in_rx.recv_event()?;

        match kind {
            EventKind::Request => &mut self.requests,
            EventKind::Response => &mut self.responses,
        }
        .insert(id, event);

        Ok((kind, id))
    }
}

pub trait EventSender {
    type Data: Send + Sync + 'static;

    fn send_event(&mut self, data: WithMetadata<Self::Data>) -> anyhow::Result<()>;
}

impl<T: Send + Sync + 'static> EventSender for mpsc::UnboundedSender<WithMetadata<T>> {
    type Data = T;

    fn send_event(&mut self, data: WithMetadata<Self::Data>) -> anyhow::Result<()> {
        self.send(data)?;
        Ok(())
    }
}

pub trait EventReceiver {
    type Data: Send + Sync + 'static;

    fn recv_event(&mut self) -> anyhow::Result<WithMetadata<Self::Data>>;
}

impl<T: Send + Sync + 'static> EventReceiver for mpsc::UnboundedReceiver<WithMetadata<T>> {
    type Data = T;

    fn recv_event(&mut self) -> anyhow::Result<WithMetadata<Self::Data>> {
        let data = self.try_recv()?;
        Ok(data)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct NextEventId(EventId);

impl NextEventId {
    fn next(&mut self) -> EventId {
        self.0 .0 += 1;
        self.0
    }
}

pub struct EventHandlerPlugin<I, O> {
    pub recv_interval: Duration,
    _marker: PhantomData<fn(&I, &O)>,
}

impl<I, O> Default for EventHandlerPlugin<I, O> {
    fn default() -> Self {
        Self {
            recv_interval: Duration::from_secs_f32(0.05),
            _marker: PhantomData,
        }
    }
}

impl<I, O> Plugin for EventHandlerPlugin<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_event::<ReceivedRequest<I>>()
            .add_event::<ReceivedResponse<I>>();

        add_recv_inbound_events_system::<
            mpsc::UnboundedReceiver<WithMetadata<I>>,
            mpsc::UnboundedSender<WithMetadata<O>>,
        >(app, self.recv_interval);
    }
}

#[derive(Debug, Clone, Event)]
pub struct ReceivedEvent<K, E>(pub EventId, PhantomData<fn(&K, &E)>);

impl<K, E> From<EventId> for ReceivedEvent<K, E> {
    fn from(id: EventId) -> Self {
        Self(id, PhantomData)
    }
}

impl<K, E> ReceivedEvent<K, E> {
    pub fn id(&self) -> EventId {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventKindRequest;

pub type ReceivedRequest<E> = ReceivedEvent<EventKindRequest, E>;

#[derive(Debug, Clone, Default)]
pub struct EventKindResponse;

pub type ReceivedResponse<E> = ReceivedEvent<EventKindResponse, E>;

fn add_recv_inbound_events_system<R, S>(app: &mut App, interval: Duration)
where
    R: EventReceiver + Send + Sync + 'static,
    S: EventSender + Send + Sync + 'static,
{
    app.add_systems(
        Update,
        recv_inbound_events::<R, S>
            .run_if(resource_exists::<EventHandler<R, S>>.and(on_timer(interval))),
    );
}

fn recv_inbound_events<R, S>(
    mut ev_handler: ResMut<EventHandler<R, S>>,
    mut request_w: EventWriter<ReceivedRequest<R::Data>>,
    mut response_w: EventWriter<ReceivedResponse<R::Data>>,
) where
    R: EventReceiver + Send + Sync + 'static,
    S: EventSender + Send + Sync + 'static,
{
    let Ok((kind, id)) = ev_handler.try_recv() else {
        return;
    };

    match kind {
        EventKind::Request => {
            request_w.send(id.into());
        }
        EventKind::Response => {
            response_w.send(id.into());
        }
    }
}
