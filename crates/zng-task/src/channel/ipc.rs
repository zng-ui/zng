use std::io;

use serde::{Deserialize, Serialize};
use zng_time::Deadline;

use crate::channel::ChannelError;

/// The transmitting end of an IPC channel.
///
/// Use [`ipc_channel`](self::ipc_channel) to declare a new channel.
#[derive(Serialize, Deserialize)]
pub struct IpcSender<T> {
    sender: ipc_channel::ipc::IpcSender<T>,
}
impl<T: IpcValue> Clone for IpcSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
impl<T: IpcValue> IpcSender<T> {
    /// Send a value into the channel.
    /// 
    /// IPC channels are unbounded, this never blocks.
    pub fn send(&self, msg: T) -> Result<(), ChannelError> {
        self.sender.send(msg).map_err(ChannelError::other)
    }
}

/// The receiving end of an IPC channel.
///
/// Use [`ipc_channel`](self::ipc_channel) to declare a new channel.
#[derive(Serialize, Deserialize)]
pub struct IpcReceiver<T> {
    recv: Option<ipc_channel::ipc::IpcReceiver<T>>,
}
impl<T: IpcValue> IpcReceiver<T> {
    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped.
    pub async fn recv(&mut self) -> Result<T, ChannelError> {
        let recv = self.recv.take().unwrap();
        let (recv, r) = crate::wait(move || {
            let r = recv.recv();
            (recv, r)
        }).await;
        self.recv = Some(recv);
        Ok(r?)
    }
    
    /// Block for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `deadline` is reached.
    pub async fn recv_deadline(&mut self, deadline: impl Into<Deadline>) -> Result<T, ChannelError> {
        match crate::with_deadline(self.recv(), deadline).await {
            Ok(r) => r,
            Err(_) => Err(ChannelError::Timeout),
        }
    }

    /// Block for an incoming value from the channel associated with this receiver.
    pub fn recv_blocking(&self) -> Result<T, ChannelError> {
        let r = self.recv.as_ref().unwrap().recv()?;
        Ok(r)
    }

    /// Block for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `deadline` is reached.
    pub fn recv_deadline_blocking(&self, deadline: impl Into<Deadline>) -> Result<T, ChannelError> {
        match deadline.into().time_left() {
            Some(d) => Ok(self.recv.as_ref().unwrap().try_recv_timeout(d)?),
            None => Err(ChannelError::Timeout),
        }
    }
}

/// Create an IPC channel.
///
/// Channel is unbounded, that is, equivalent to [`channel::unbounded`], but capable of communication with another process
/// by serializing and deserializing messages.
///
/// Note that the channel endpoints can also be send over IPC, the first channel is setup by [`process::Worker`]. You
/// can also use the [`ipc_channel`] crate to setup the first channel with a custom worker process.
///
/// [`channel::unbounded`]: crate::channel::unbounded
/// [`process::Worker`]: crate::process::Worker
/// [`ipc_channel`]: https://docs.rs/ipc-channel/latest/ipc_channel/ipc/struct.IpcOneShotServer.html
pub fn ipc_channel<T>() -> io::Result<(IpcSender<T>, IpcReceiver<T>)>
where
    T: IpcValue,
{
    let (s, r) = ipc_channel::ipc::channel()?;
    Ok((IpcSender { sender: s }, IpcReceiver { recv: Some(r) }))
}

/// Represents a type that can be an input and output of IPC channels.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
///
/// # Implementing
///
/// Types need to be `serde::Serialize + serde::de::Deserialize + Send + 'static` to auto-implement this trait,
/// if you want to send an external type in that does not implement all the traits
/// you may need to declare a *newtype* wrapper.
#[diagnostic::on_unimplemented(note = "`IpcValue` is implemented for all `T: Serialize + Deserialize + Send + 'static`")]
pub trait IpcValue: serde::Serialize + for<'d> serde::de::Deserialize<'d> + Send + 'static {}

impl<T: serde::Serialize + for<'d> serde::de::Deserialize<'d> + Send + 'static> IpcValue for T {}

impl From<ipc_channel::ipc::IpcError> for ChannelError {
    fn from(value: ipc_channel::ipc::IpcError) -> Self {
        match value {
            ipc_channel::ipc::IpcError::Disconnected => ChannelError::Disconnected,
            e => ChannelError::other(e),
        }
    }
}
impl From<ipc_channel::ipc::TryRecvError> for ChannelError {
    fn from(value: ipc_channel::ipc::TryRecvError) -> Self {
        match value {
            ipc_channel::ipc::TryRecvError::IpcError(ipc_channel::ipc::IpcError::Disconnected) => ChannelError::Disconnected,
            ipc_channel::ipc::TryRecvError::Empty => ChannelError::Timeout,
            e => ChannelError::other(e),
        }
    }
}