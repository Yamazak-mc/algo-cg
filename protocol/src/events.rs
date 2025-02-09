use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WithMetadata<E> {
    pub kind: EventKind,
    pub id: EventId,
    pub event: E,
}

impl<E: fmt::Debug> fmt::Debug for WithMetadata<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({}) {:?}", self.kind, self.id, self.event)
    }
}

impl<E> WithMetadata<E> {
    #[inline]
    pub fn response_to<R>(&self, event: R) -> WithMetadata<R> {
        WithMetadata {
            kind: EventKind::Response,
            id: self.id,
            event,
        }
    }

    #[inline]
    pub fn metadata(&self) -> (EventKind, EventId) {
        (self.kind, self.id)
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

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ev{}", self.0)
    }
}

impl EventId {
    pub const fn from_raw(id: u32) -> Self {
        Self(id)
    }
}

/// A storage for received events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBox<E> {
    requests: BTreeMap<EventId, E>,
    responses: BTreeMap<EventId, E>,
}

impl<E> Default for EventBox<E> {
    fn default() -> Self {
        Self {
            requests: BTreeMap::new(),
            responses: BTreeMap::new(),
        }
    }
}

impl<E> EventBox<E> {
    /// Marks that the user has sent a request with the specified ID
    /// and is waiting for the corresponding response from the peer.
    pub fn expect_response(&mut self, _id: EventId) {
        // TODO
        unimplemented!();
    }

    /// Stores an event with a metadata to the storage.
    ///
    /// If a spot for the event is already taken, returns `Some(_)`.
    pub fn store(&mut self, event: WithMetadata<E>) -> Option<E> {
        match event.kind {
            EventKind::Request => &mut self.requests,
            EventKind::Response => &mut self.responses,
        }
        .insert(event.id, event.event)
    }

    pub fn take_request(&mut self, id: EventId) -> Option<E> {
        self.requests.remove(&id)
    }

    pub fn take_response(&mut self, id: EventId) -> Option<E> {
        self.responses.remove(&id)
    }

    pub fn get_request(&mut self, id: EventId) -> Option<&E> {
        self.requests.get(&id)
    }

    pub fn get_response(&mut self, id: EventId) -> Option<&E> {
        self.requests.get(&id)
    }

    pub fn find_request_id(&mut self, pred: impl Fn(&E) -> bool) -> Option<EventId> {
        self.requests.iter().find(|(_, v)| (pred)(*v)).map(|v| *v.0)
    }

    pub fn take_request_if(&mut self, pred: impl Fn(&E) -> bool) -> Option<(EventId, E)> {
        self.find_request_id(pred)
            .map(|id| (id, self.take_request(id).expect("request should exist")))
    }

    pub fn get_request_if(&mut self, pred: impl Fn(&E) -> bool) -> Option<(EventId, &E)> {
        self.find_request_id(pred)
            .map(|id| (id, self.get_request(id).expect("request should exist")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct NextEventId(EventId);

impl Default for NextEventId {
    fn default() -> Self {
        Self(EventId(0))
    }
}

impl NextEventId {
    pub fn produce(&mut self) -> EventId {
        self.0 .0 += 1;
        self.0
    }
}
