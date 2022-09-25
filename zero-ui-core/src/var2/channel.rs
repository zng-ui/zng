use std::time::{Duration, Instant};

use response::Response;

use crate::{
    app::{AppDisconnected, AppEventSender, RecvFut, TimeoutOrAppDisconnected},
    context::UpdatesTrace,
    crate_util::PanicPayload,
};

use super::*;

/// A variable update receiver that can be used from any thread and without access to [`Vars`].
///
/// Use [`Var::receiver`] to create a receiver, drop to stop listening.
pub struct VarReceiver<T: VarValue + Send> {
    receiver: flume::Receiver<T>,
}
impl<T: VarValue + Send> Clone for VarReceiver<T> {
    fn clone(&self) -> Self {
        VarReceiver {
            receiver: self.receiver.clone(),
        }
    }
}
impl<T: VarValue + Send> fmt::Debug for VarReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarReceiver")
    }
}
impl<T: VarValue + Send> VarReceiver<T> {
    /// Receives the oldest sent update not received, blocks until the variable updates.
    pub fn recv(&self) -> Result<T, AppDisconnected<()>> {
        self.receiver.recv().map_err(|_| AppDisconnected(()))
    }

    /// Tries to receive the oldest sent update, returns `Ok(args)` if there was at least
    /// one update, or returns `Err(None)` if there was no update or returns `Err(AppDisconnected)` if the connected
    /// app has exited.
    pub fn try_recv(&self) -> Result<T, Option<AppDisconnected<()>>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppDisconnected(())),
        })
    }

    /// Receives the oldest sent update, blocks until the event updates or until the `deadline` is reached.
    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, TimeoutOrAppDisconnected> {
        self.receiver.recv_deadline(deadline).map_err(TimeoutOrAppDisconnected::from)
    }

    /// Receives the oldest sent update, blocks until the event updates or until timeout.
    pub fn recv_timeout(&self, dur: Duration) -> Result<T, TimeoutOrAppDisconnected> {
        self.receiver.recv_timeout(dur).map_err(TimeoutOrAppDisconnected::from)
    }

    /// Returns a future that receives the oldest sent update, awaits until an event update occurs.
    pub fn recv_async(&self) -> RecvFut<T> {
        self.receiver.recv_async().into()
    }

    /// Turns into a future that receives the oldest sent update, awaits until an event update occurs.
    pub fn into_recv_async(self) -> RecvFut<'static, T> {
        self.receiver.into_recv_async().into()
    }

    /// Creates a blocking iterator over event updates, if there are no updates in the buffer the iterator blocks,
    /// the iterator only finishes when the app shuts-down.
    pub fn iter(&self) -> flume::Iter<T> {
        self.receiver.iter()
    }

    /// Create a non-blocking iterator over event updates, the iterator finishes if
    /// there are no more updates in the buffer.
    pub fn try_iter(&self) -> flume::TryIter<T> {
        self.receiver.try_iter()
    }
}
impl<T: VarValue + Send> From<VarReceiver<T>> for flume::Receiver<T> {
    fn from(e: VarReceiver<T>) -> Self {
        e.receiver
    }
}
impl<'a, T: VarValue + Send> IntoIterator for &'a VarReceiver<T> {
    type Item = T;

    type IntoIter = flume::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.iter()
    }
}
impl<T: VarValue + Send> IntoIterator for VarReceiver<T> {
    type Item = T;

    type IntoIter = flume::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.into_iter()
    }
}

/// A variable update sender that can set a variable from any thread and without access to [`Vars`].
///
/// Use [`Var::sender`] to create a sender, drop to stop holding the paired variable in the UI thread.
pub struct VarSender<T>
where
    T: VarValue + Send,
{
    wake: AppEventSender,
    sender: flume::Sender<T>,
}
impl<T: VarValue + Send> Clone for VarSender<T> {
    fn clone(&self) -> Self {
        VarSender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue + Send> fmt::Debug for VarSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarSender")
    }
}
impl<T> VarSender<T>
where
    T: VarValue + Send,
{
    /// Sends a new value for the variable, unless the connected app has exited.
    ///
    /// If the variable is read-only when the `new_value` is received it is silently dropped, if more then one
    /// value is sent before the app can process then, only the last value shows as an update in the UI thread.
    pub fn send(&self, new_value: T) -> Result<(), AppDisconnected<T>> {
        UpdatesTrace::log_var::<T>();
        self.sender.send(new_value).map_err(AppDisconnected::from)?;
        let _ = self.wake.send_var();
        Ok(())
    }

    /// Resume a panic in the app thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), AppDisconnected<PanicPayload>> {
        self.wake.send_resume_unwind(payload)
    }
}

/// A variable modification sender that can be used to modify a variable from any thread and without access to [`Vars`].
///
/// Use [`Var::modify_sender`] to create a sender, drop to stop holding the paired variable in the UI thread.
pub struct VarModifySender<T>
where
    T: VarValue,
{
    wake: AppEventSender,
    sender: flume::Sender<Box<dyn FnOnce(&mut VarModifyValue<T>) + Send>>,
}
impl<T: VarValue> Clone for VarModifySender<T> {
    fn clone(&self) -> Self {
        VarModifySender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue> fmt::Debug for VarModifySender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarModifySender")
    }
}
impl<T> VarModifySender<T>
where
    T: VarValue,
{
    /// Sends a modification for the variable, unless the connected app has exited.
    ///
    /// If the variable is read-only when the `modify` is received it is silently dropped, if more then one
    /// modification is sent before the app can process then, they all are applied in order sent.
    pub fn send<F>(&self, modify: F) -> Result<(), AppDisconnected<()>>
    where
        F: FnOnce(&mut VarModifyValue<T>) + Send + 'static,
    {
        self.sender.send(Box::new(modify)).map_err(|_| AppDisconnected(()))?;
        let _ = self.wake.send_var();
        Ok(())
    }

    /// Resume a panic in the app thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), AppDisconnected<PanicPayload>> {
        self.wake.send_resume_unwind(payload)
    }
}

/// Variable sender used to notify the completion of an operation from any thread.
///
/// Use [`response_channel`] to init.
pub type ResponseSender<T> = VarSender<Response<T>>;
impl<T: VarValue + Send> ResponseSender<T> {
    /// Send the one time response.
    pub fn send_response(&self, response: T) -> Result<(), AppDisconnected<T>> {
        self.send(Response::Done(response)).map_err(|e| {
            if let Response::Done(r) = e.0 {
                AppDisconnected(r)
            } else {
                unreachable!()
            }
        })
    }
}

/// New paired [`ResponseSender`] and [`ResponseVar`] in the waiting state.
pub fn response_channel<T: VarValue + Send, Vw: WithVars>(vars: &Vw) -> (ResponseSender<T>, ResponseVar<T>) {
    let (responder, response) = response_var();
    vars.with_vars(|vars| (responder.sender(vars), response))
}
