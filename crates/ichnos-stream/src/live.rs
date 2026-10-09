//! Synchronous counterparts of pm4py LiveEventStream and LiveTraceStream.

use crate::{Error, Result};
use ichnos_core::{Event, Trace};
use std::{cell::RefCell, collections::VecDeque, fmt, rc::Rc};

/// A push-based consumer. Algorithms expose their own typed result accessors.
pub trait StreamSink<T = Event> {
    /// Consume an item in input order. Errors are returned to the caller.
    fn push(&mut self, item: &T) -> Result<()>;

    /// Optional local identity for deduplicating shared observers. Owned values
    /// return None by default and register as distinct consumers.
    fn registration_key(&self) -> Option<usize> {
        None
    }
}

impl<T, S: StreamSink<T>> StreamSink<T> for Rc<RefCell<S>> {
    fn registration_key(&self) -> Option<usize> {
        Some(Rc::as_ptr(self) as usize)
    }
    fn push(&mut self, item: &T) -> Result<()> {
        self.try_borrow_mut()
            .map_err(|_| Error::ObserverBorrowed)?
            .push(item)
    }
}

/// The lifecycle shared by event and trace streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamState {
    /// Appends queue items until delivery starts.
    #[default]
    Inactive,
    /// Appends deliver immediately to registered observers.
    Active,
    /// Delivery is complete; later appends are ignored.
    Finished,
}

/// Registration identity, local to a stream and never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverId(usize);

/// FIFO live delivery without a worker thread or implicit executor.
///
/// Register an `Rc<RefCell<S>>` to retain access to an algorithm's result.
/// Every observer receives each item once, in registration order. Delivery
/// continues after observer errors and returns the first error; it is not
/// rolled back or retried. Stopping drains even an inactive stream's queue.
pub struct LiveStream<T> {
    state: StreamState,
    queue: VecDeque<T>,
    observers: Vec<(ObserverId, Box<dyn StreamSink<T>>)>,
    next_id: usize,
}

impl<T> fmt::Debug for LiveStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveStream")
            .field("state", &self.state)
            .field("queued", &self.queue.len())
            .field("observers", &self.observers.len())
            .finish()
    }
}

impl<T> Default for LiveStream<T> {
    fn default() -> Self {
        Self {
            state: StreamState::Inactive,
            queue: VecDeque::new(),
            observers: Vec::new(),
            next_id: 0,
        }
    }
}

impl<T> LiveStream<T> {
    /// Create an inactive stream with no observers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current lifecycle state.
    pub fn state(&self) -> StreamState {
        self.state
    }

    /// Register a consumer and return its deregistration identity. Registering
    /// the same shared consumer again returns its existing identity.
    pub fn register(&mut self, sink: impl StreamSink<T> + 'static) -> ObserverId {
        if let Some(key) = sink.registration_key()
            && let Some((id, _)) = self
                .observers
                .iter()
                .find(|(_, observer)| observer.registration_key() == Some(key))
        {
            return *id;
        }
        let id = ObserverId(self.next_id);
        self.next_id += 1;
        self.observers.push((id, Box::new(sink)));
        id
    }

    /// Remove this stream's observer; return whether it was registered.
    pub fn deregister(&mut self, id: ObserverId) -> bool {
        let before = self.observers.len();
        self.observers.retain(|(registered, _)| *registered != id);
        self.observers.len() != before
    }

    fn deliver(&mut self, item: &T) -> Result<()> {
        let mut first_error = None;
        for (_, observer) in &mut self.observers {
            if let Err(error) = observer.push(item) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn drain(&mut self) -> Result<()> {
        let mut first_error = None;
        while let Some(item) = self.queue.pop_front() {
            if let Err(error) = self.deliver(&item) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Queue or deliver an owned item; return false after the stream finishes.
    pub fn append(&mut self, item: T) -> Result<bool> {
        match self.state {
            StreamState::Inactive => self.queue.push_back(item),
            StreamState::Active => self.deliver(&item)?,
            StreamState::Finished => return Ok(false),
        }
        Ok(true)
    }

    /// Start delivery and drain buffered items. Starting twice is an error.
    pub fn start(&mut self) -> Result<()> {
        if self.state != StreamState::Inactive {
            return Err(Error::StreamState(self.state));
        }
        self.state = StreamState::Active;
        self.drain()
    }

    /// Drain queued items and finish. Repeated stops are harmless.
    pub fn stop(&mut self) -> Result<()> {
        let result = self.drain();
        self.state = StreamState::Finished;
        result
    }
}

impl<T: Clone> StreamSink<T> for LiveStream<T> {
    fn push(&mut self, item: &T) -> Result<()> {
        self.append(item.clone()).map(|_| ())
    }
}

/// A live stream of canonical events.
pub type LiveEventStream = LiveStream<Event>;
/// A live stream of canonical traces.
pub type LiveTraceStream = LiveStream<Trace>;

/// A live-to-static consumer retaining items in delivery order.
#[derive(Debug, Clone)]
pub struct Collector<T> {
    items: Vec<T>,
}

impl<T> Default for Collector<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> Collector<T> {
    /// Create an empty collector.
    pub fn new() -> Self {
        Self::default()
    }
    /// Borrow all items delivered so far.
    pub fn get(&self) -> &[T] {
        &self.items
    }
    /// Take the collected items without cloning them.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }
}

impl<T: Clone> StreamSink<T> for Collector<T> {
    fn push(&mut self, item: &T) -> Result<()> {
        self.items.push(item.clone());
        Ok(())
    }
}
