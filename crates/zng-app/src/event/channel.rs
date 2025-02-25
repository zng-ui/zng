use std::time::Duration;

use crate::{AppEventSender, update::UpdatesTrace};

use super::*;

pub(crate) struct EventUpdateMsg {
    args: Box<dyn FnOnce() -> EventUpdate + Send>,
}
impl EventUpdateMsg {
    pub(crate) fn get(self) -> EventUpdate {
        (self.args)()
    }
}
impl fmt::Debug for EventUpdateMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventUpdateMsg").finish_non_exhaustive()
    }
}

/// An event update sender that can be used from any thread and without access to [`EVENTS`].
///
/// Use [`Event::sender`] to create a sender.
pub struct EventSender<A>
where
    A: EventArgs + Send,
{
    pub(super) sender: AppEventSender,
    pub(super) event: Event<A>,
}
impl<A> Clone for EventSender<A>
where
    A: EventArgs + Send,
{
    fn clone(&self) -> Self {
        EventSender {
            sender: self.sender.clone(),
            event: self.event,
        }
    }
}
impl<A> fmt::Debug for EventSender<A>
where
    A: EventArgs + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventSender({:?})", &self.event)
    }
}
impl<A> EventSender<A>
where
    A: EventArgs + Send,
{
    /// Send an event update.
    pub fn send(&self, args: A) -> Result<(), AppDisconnected<A>> {
        UpdatesTrace::log_event(self.event.as_any());

        let event = self.event;
        let msg = EventUpdateMsg {
            args: Box::new(move || event.new_update(args)),
        };

        self.sender.send_event(msg).map_err(|e| {
            if let Some(args) = (e.0.args)().args.as_any().downcast_ref::<A>() {
                AppDisconnected(args.clone())
            } else {
                unreachable!()
            }
        })
    }

    /// Event that receives from this sender.
    pub fn event(&self) -> Event<A> {
        self.event
    }
}

/// An event channel receiver.
///
/// Use [`Event::receiver`] to create a receiver, drop to stop listening.
#[must_use = "stops receiving on drop"]
pub struct EventReceiver<A>
where
    A: EventArgs + Send,
{
    pub(super) event: Event<A>,
    pub(super) receiver: flume::Receiver<A>,
}
impl<A> Clone for EventReceiver<A>
where
    A: EventArgs + Send,
{
    fn clone(&self) -> Self {
        EventReceiver {
            event: self.event,
            receiver: self.receiver.clone(),
        }
    }
}
impl<A> fmt::Debug for EventReceiver<A>
where
    A: EventArgs + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventSender({:?})", &self.event)
    }
}
impl<A> EventReceiver<A>
where
    A: EventArgs + Send,
{
    /// Receives the oldest update, blocks until the event updates.
    ///
    /// Note that *oldest* here refers to send order (FIFO), not the args creation timestamp.
    pub fn recv(&self) -> Result<A, AppDisconnected<()>> {
        self.receiver.recv().map_err(|_| AppDisconnected(()))
    }

    /// Tries to receive the oldest sent update not received, returns `Ok(args)` if there was at least
    /// one update, or returns `Err(None)` if there was no update or returns `Err(AppDisconnected)` if the connected
    /// app has exited.
    pub fn try_recv(&self) -> Result<A, Option<AppDisconnected<()>>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppDisconnected(())),
        })
    }

    /// Receives the oldest send update, blocks until the event updates or until the `deadline` is reached.
    pub fn recv_deadline(&self, deadline: Instant) -> Result<A, TimeoutOrAppDisconnected> {
        self.receiver.recv_deadline(deadline).map_err(TimeoutOrAppDisconnected::from)
    }

    /// Receives the oldest send update, blocks until the event updates or until timeout.
    pub fn recv_timeout(&self, dur: Duration) -> Result<A, TimeoutOrAppDisconnected> {
        self.receiver.recv_timeout(dur).map_err(TimeoutOrAppDisconnected::from)
    }

    /// Returns a future that receives the oldest send update, awaits until an event update occurs.
    pub async fn recv_async(&self) -> Result<A, AppDisconnected<()>> {
        RecvFut::from(self.receiver.recv_async()).await
    }

    /// Turns into a future that receives the oldest send update, awaits until an event update occurs.
    pub fn into_recv_async(self) -> impl Future<Output = Result<A, AppDisconnected<()>>> + Send + Sync + 'static {
        RecvFut::from(self.receiver.into_recv_async())
    }

    /// Creates a blocking iterator over event updates, if there are no updates sent the iterator blocks,
    /// the iterator only finishes when the app shuts-down.
    pub fn iter(&self) -> flume::Iter<A> {
        self.receiver.iter()
    }

    /// Create a non-blocking iterator over event updates, the iterator finishes if
    /// there are no more updates sent.
    pub fn try_iter(&self) -> flume::TryIter<A> {
        self.receiver.try_iter()
    }

    /// Event that sends to this receiver.
    pub fn event(&self) -> Event<A> {
        self.event
    }
}
impl<A> From<EventReceiver<A>> for flume::Receiver<A>
where
    A: EventArgs + Send,
{
    fn from(e: EventReceiver<A>) -> Self {
        e.receiver
    }
}
impl<'a, A> IntoIterator for &'a EventReceiver<A>
where
    A: EventArgs + Send,
{
    type Item = A;

    type IntoIter = flume::Iter<'a, A>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.iter()
    }
}
impl<A> IntoIterator for EventReceiver<A>
where
    A: EventArgs + Send,
{
    type Item = A;

    type IntoIter = flume::IntoIter<A>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.into_iter()
    }
}

/// Error when the app connected to a sender/receiver channel has disconnected.
///
/// Contains the value that could not be send or `()` for receiver errors.
pub struct AppDisconnected<T>(pub T);
impl From<flume::RecvError> for AppDisconnected<()> {
    fn from(_: flume::RecvError) -> Self {
        AppDisconnected(())
    }
}
impl<T> From<flume::SendError<T>> for AppDisconnected<T> {
    fn from(e: flume::SendError<T>) -> Self {
        AppDisconnected(e.0)
    }
}
impl<T> fmt::Debug for AppDisconnected<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppDisconnected<{}>", pretty_type_name::pretty_type_name::<T>())
    }
}
impl<T> fmt::Display for AppDisconnected<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot send/receive because the app has disconnected")
    }
}
impl<T> std::error::Error for AppDisconnected<T> {}

/// Error when the app connected to a sender channel has disconnected or taken to long to respond.
pub enum TimeoutOrAppDisconnected {
    /// Connected app has not responded.
    Timeout,
    /// Connected app has disconnected.
    AppDisconnected,
}
impl From<flume::RecvTimeoutError> for TimeoutOrAppDisconnected {
    fn from(e: flume::RecvTimeoutError) -> Self {
        match e {
            flume::RecvTimeoutError::Timeout => TimeoutOrAppDisconnected::Timeout,
            flume::RecvTimeoutError::Disconnected => TimeoutOrAppDisconnected::AppDisconnected,
        }
    }
}
impl fmt::Debug for TimeoutOrAppDisconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TimeoutOrAppDisconnected::")?;
        }
        match self {
            TimeoutOrAppDisconnected::Timeout => write!(f, "Timeout"),
            TimeoutOrAppDisconnected::AppDisconnected => write!(f, "AppDisconnected"),
        }
    }
}
impl fmt::Display for TimeoutOrAppDisconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeoutOrAppDisconnected::Timeout => write!(f, "failed send, timeout"),
            TimeoutOrAppDisconnected::AppDisconnected => write!(f, "cannot send because the app has disconnected"),
        }
    }
}
impl std::error::Error for TimeoutOrAppDisconnected {}

/// A future that receives a single message from a running app.
struct RecvFut<'a, M>(flume::r#async::RecvFut<'a, M>);
impl<'a, M> From<flume::r#async::RecvFut<'a, M>> for RecvFut<'a, M> {
    fn from(f: flume::r#async::RecvFut<'a, M>) -> Self {
        Self(f)
    }
}
impl<M> Future for RecvFut<'_, M> {
    type Output = Result<M, AppDisconnected<()>>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match std::pin::Pin::new(&mut self.0).poll(cx) {
            std::task::Poll::Ready(r) => std::task::Poll::Ready(r.map_err(|_| AppDisconnected(()))),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
