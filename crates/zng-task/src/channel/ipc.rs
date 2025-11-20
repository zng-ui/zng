#![cfg_attr(not(ipc), allow(unused))]

use std::{fmt, io};

use serde::{Deserialize, Serialize};
use zng_time::Deadline;

use crate::channel::ChannelError;

/// The transmitting end of an IPC channel.
///
/// Use [`ipc_unbounded`] to declare a new channel.
pub struct IpcSender<T> {
    #[cfg(ipc)]
    sender: Option<ipc_channel::ipc::IpcSender<T>>,
    #[cfg(not(ipc))]
    sender: super::Sender<T>,
}
impl<T: IpcValue> fmt::Debug for IpcSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IpcSender").finish_non_exhaustive()
    }
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
    /// IPC channels are unbounded, this never blocks in the current release.
    pub fn send_blocking(&mut self, msg: T) -> Result<(), ChannelError> {
        #[cfg(ipc)]
        {
            let sender = match self.sender.take() {
                Some(s) => s,
                None => return Err(ChannelError::disconnected()),
            };
            let r = crate::channel::with_ipc_serialization(|| sender.send(msg).map_err(ChannelError::disconnected_by));
            if r.is_ok() {
                self.sender = Some(sender);
            }
            r
        }
        #[cfg(not(ipc))]
        {
            self.sender.send_blocking(msg)
        }
    }
}
impl<T: IpcValue> Serialize for IpcSender<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(ipc)]
        {
            if !crate::channel::is_ipc_serialization() {
                return Err(serde::ser::Error::custom("cannot serialize `IpcSender` outside IPC"));
            }
            self.sender.serialize(serializer)
        }
        #[cfg(not(ipc))]
        {
            let _ = serializer;
            Err(serde::ser::Error::custom("cannot serialize `IpcSender` outside IPC"))
        }
    }
}
impl<'de, T: IpcValue> Deserialize<'de> for IpcSender<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[cfg(ipc)]
        {
            Ok(Self {
                sender: Option::<ipc_channel::ipc::IpcSender<T>>::deserialize(deserializer)?,
            })
        }
        #[cfg(not(ipc))]
        {
            let _ = deserializer;
            Err(serde::de::Error::custom("cannot deserialize `IpcSender` outside IPC"))
        }
    }
}

/// The receiving end of an IPC channel.
///
/// Use [`ipc_unbounded`] to declare a new channel.
pub struct IpcReceiver<T> {
    #[cfg(ipc)]
    recv: Option<ipc_channel::ipc::IpcReceiver<T>>,
    #[cfg(not(ipc))]
    recv: super::Receiver<T>,
}
impl<T: IpcValue> fmt::Debug for IpcReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IpcReceiver").finish_non_exhaustive()
    }
}
impl<T: IpcValue> IpcReceiver<T> {
    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped.
    pub async fn recv(&mut self) -> Result<T, ChannelError> {
        #[cfg(ipc)]
        {
            let recv = match self.recv.take() {
                Some(r) => r,
                None => return Err(ChannelError::disconnected()),
            };
            let (recv, r) = crate::wait(move || {
                let r = recv.recv();
                (recv, r)
            })
            .await;
            let r = r?;
            self.recv = Some(recv);
            Ok(r)
        }
        #[cfg(not(ipc))]
        {
            self.recv.recv().await
        }
    }

    /// Block for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `deadline` is reached.
    pub async fn recv_deadline(&mut self, deadline: impl Into<Deadline>) -> Result<T, ChannelError> {
        #[cfg(ipc)]
        {
            match crate::with_deadline(self.recv(), deadline).await {
                Ok(r) => r,
                Err(_) => Err(ChannelError::Timeout),
            }
        }
        #[cfg(not(ipc))]
        {
            self.recv.recv_deadline(deadline).await
        }
    }

    /// Block for an incoming value from the channel associated with this receiver.
    pub fn recv_blocking(&mut self) -> Result<T, ChannelError> {
        #[cfg(ipc)]
        {
            let recv = match self.recv.take() {
                Some(r) => r,
                None => return Err(ChannelError::disconnected()),
            };
            let r = recv.recv()?;
            self.recv = Some(recv);
            Ok(r)
        }
        #[cfg(not(ipc))]
        {
            self.recv.recv_blocking()
        }
    }

    /// Block for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `deadline` is reached.
    pub fn recv_deadline_blocking(&mut self, deadline: impl Into<Deadline>) -> Result<T, ChannelError> {
        #[cfg(ipc)]
        {
            let recv = match self.recv.take() {
                Some(r) => r,
                None => return Err(ChannelError::disconnected()),
            };
            match deadline.into().time_left() {
                Some(d) => match recv.try_recv_timeout(d) {
                    Ok(r) => {
                        self.recv = Some(recv);
                        Ok(r)
                    }
                    Err(e) => match e {
                        ipc_channel::ipc::TryRecvError::IpcError(e) => Err(ChannelError::disconnected_by(e)),
                        ipc_channel::ipc::TryRecvError::Empty => {
                            self.recv = Some(recv);
                            Err(ChannelError::Timeout)
                        }
                    },
                },
                None => {
                    self.recv = Some(recv);
                    Err(ChannelError::Timeout)
                }
            }
        }
        #[cfg(not(ipc))]
        {
            self.recv.recv_deadline_blocking(deadline)
        }
    }

    /// Create a blocking iterator that receives until a channel error.
    pub fn iter(&mut self) -> impl Iterator<Item = T> {
        #[cfg(ipc)]
        {
            std::iter::from_fn(|| self.recv_blocking().ok()).fuse()
        }
        #[cfg(not(ipc))]
        {
            self.recv.iter()
        }
    }

    /// Returns the next incoming message in the channel or `None`.
    pub fn try_recv(&mut self) -> Result<Option<T>, ChannelError> {
        #[cfg(ipc)]
        {
            let recv = match self.recv.take() {
                Some(r) => r,
                None => return Err(ChannelError::disconnected()),
            };
            match recv.try_recv() {
                Ok(r) => {
                    self.recv = Some(recv);
                    Ok(Some(r))
                }
                Err(e) => match e {
                    ipc_channel::ipc::TryRecvError::IpcError(e) => Err(ChannelError::disconnected_by(e)),
                    ipc_channel::ipc::TryRecvError::Empty => Ok(None),
                },
            }
        }
        #[cfg(not(ipc))]
        {
            self.recv.try_recv()
        }
    }

    /// Iterate over all the pending incoming messages in the channel, until the channel is empty or error.
    pub fn try_iter(&mut self) -> impl Iterator<Item = T> {
        #[cfg(ipc)]
        {
            std::iter::from_fn(|| self.try_recv().ok().flatten()).fuse()
        }
        #[cfg(not(ipc))]
        {
            self.recv.try_iter()
        }
    }
}
impl<T: IpcValue> Serialize for IpcReceiver<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[cfg(ipc)]
        {
            if !crate::channel::is_ipc_serialization() {
                return Err(serde::ser::Error::custom("cannot serialize `IpcReceiver` outside IPC"));
            }
            self.recv.serialize(serializer)
        }
        #[cfg(not(ipc))]
        {
            let _ = serializer;
            Err(serde::ser::Error::custom("cannot serialize `IpcReceiver` outside IPC"))
        }
    }
}
impl<'de, T: IpcValue> Deserialize<'de> for IpcReceiver<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[cfg(ipc)]
        {
            Ok(Self {
                recv: Option::<ipc_channel::ipc::IpcReceiver<T>>::deserialize(deserializer)?,
            })
        }
        #[cfg(not(ipc))]
        {
            let _ = deserializer;
            Err(serde::de::Error::custom("cannot deserialize `IpcReceiver` outside IPC"))
        }
    }
}

/// Create an unbounded IPC channel.
///
/// Note that the channel endpoints can also be send over IPC, the first channel is setup by [`process::Worker`]. You
/// can also use the [`NamedIpcReceiver`] or [`NamedIpcSender`] to create the first channel with a custom process.
///
/// Note that the channel is only IPC if build with `"ipc"` crate feature, otherwise it will falls back to [`channel::unbounded`].
///
/// [`channel::unbounded`]: crate::channel::unbounded
/// [`process::Worker`]: crate::process::Worker
pub fn ipc_unbounded<T: IpcValue>() -> io::Result<(IpcSender<T>, IpcReceiver<T>)> {
    #[cfg(ipc)]
    {
        let (s, r) = ipc_channel::ipc::channel()?;
        Ok((IpcSender { sender: Some(s) }, IpcReceiver { recv: Some(r) }))
    }
    #[cfg(not(ipc))]
    {
        let (sender, recv) = super::unbounded();
        Ok((IpcSender { sender }, IpcReceiver { recv }))
    }
}

/// Init named IPC connection with another process, the receiver end is in the first process.
///
/// Note that this is less efficient than [`ipc_unbounded`], it is only recommended for creating the first channel,
/// you can send other channels using the first channel.
///
/// See also [`NamedIpcSender`].
pub struct NamedIpcReceiver<T: IpcValue> {
    #[cfg(ipc)]
    server: ipc_channel::ipc::IpcOneShotServer<IpcReceiver<T>>,
    #[cfg(ipc)]
    name: String,

    #[cfg(not(ipc))]
    inner: named_channel_fallback::NamedReceiver<T>,
}
impl<T: IpcValue> fmt::Debug for NamedIpcReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedIpcReceiver")
            .field("name", &self.name())
            .finish_non_exhaustive()
    }
}
impl<T: IpcValue> NamedIpcReceiver<T> {
    /// New initial IPC connection.
    pub fn new() -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let (server, name) = ipc_channel::ipc::IpcOneShotServer::new()?;
            Ok(Self { server, name })
        }
        #[cfg(not(ipc))]
        {
            Ok(Self {
                inner: named_channel_fallback::NamedReceiver::new(),
            })
        }
    }

    /// Unique name that must be used by the other process to [`IpcSender::connect`].
    ///
    /// You can share the name with the other process using a command argument or environment variable.
    pub fn name(&self) -> &str {
        #[cfg(ipc)]
        {
            &self.name
        }
        #[cfg(not(ipc))]
        {
            self.inner.name()
        }
    }

    /// Await until other process connects.
    pub async fn connect(self) -> Result<IpcReceiver<T>, ChannelError> {
        crate::wait(move || self.connect_blocking()).await
    }

    /// Await until other process connects or `deadline` elapses.
    pub async fn connect_deadline(self, deadline: impl Into<Deadline>) -> Result<IpcReceiver<T>, ChannelError> {
        match crate::with_deadline(self.connect(), deadline).await {
            Ok(r) => r,
            Err(_) => Err(ChannelError::Timeout),
        }
    }

    /// Blocks until other process connects.
    pub fn connect_blocking(self) -> Result<IpcReceiver<T>, ChannelError> {
        #[cfg(ipc)]
        {
            let (_, recv) = self.server.accept().map_err(ChannelError::disconnected_by)?;
            Ok(recv)
        }
        #[cfg(not(ipc))]
        {
            self.inner.connect_blocking()
        }
    }

    /// Blocks until other process connects or `deadline` elapses.
    pub fn connect_deadline_blocking(self, deadline: impl Into<Deadline>) -> Result<IpcReceiver<T>, ChannelError> {
        crate::block_on(self.connect_deadline(deadline))
    }
}
impl<T: IpcValue> IpcSender<T> {
    /// Connect with a named receiver created in another process with [`NamedIpcReceiver`].
    ///
    /// This must only be called once for the `ipc_receiver_name`.
    pub fn connect(ipc_receiver_name: impl Into<String>) -> Result<Self, ChannelError> {
        Self::connect_impl(ipc_receiver_name.into())
    }
    #[cfg(ipc)]
    fn connect_impl(ipc_receiver_name: String) -> Result<Self, ChannelError> {
        let sender = ipc_channel::ipc::IpcSender::<IpcReceiver<T>>::connect(ipc_receiver_name).map_err(ChannelError::disconnected_by)?;
        let (s, r) = ipc_unbounded().map_err(ChannelError::disconnected_by)?;
        crate::channel::with_ipc_serialization(|| sender.send(r)).map_err(ChannelError::disconnected_by)?;
        Ok(s)
    }
    #[cfg(not(ipc))]
    fn connect_impl(ipc_receiver_name: String) -> Result<Self, ChannelError> {
        named_channel_fallback::sender_connect_blocking(&ipc_receiver_name)
    }
}

/// Init named IPC connection with another process, the sender end is in the first process.
///
/// Note that this is less efficient than [`ipc_unbounded`], it is only recommended for creating the first channel,
/// you can send other channels using the first channel.
///
/// See also [`NamedIpcReceiver`].
pub struct NamedIpcSender<T: IpcValue> {
    #[cfg(ipc)]
    server: ipc_channel::ipc::IpcOneShotServer<IpcSender<T>>,
    #[cfg(ipc)]
    name: String,
    #[cfg(not(ipc))]
    inner: named_channel_fallback::NamedSender<T>,
}
impl<T: IpcValue> fmt::Debug for NamedIpcSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedIpcSender").field("name", &self.name()).finish_non_exhaustive()
    }
}
impl<T: IpcValue> NamedIpcSender<T> {
    /// New initial IPC connection.
    pub fn new() -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let (server, name) = ipc_channel::ipc::IpcOneShotServer::new()?;
            Ok(Self { server, name })
        }
        #[cfg(not(ipc))]
        {
            Ok(Self {
                inner: named_channel_fallback::NamedSender::new(),
            })
        }
    }

    /// Unique name that must be used by the other process to [`IpcReceiver::connect`].
    ///
    /// You can share the name with the other process using a command argument or environment variable.
    pub fn name(&self) -> &str {
        #[cfg(ipc)]
        {
            &self.name
        }
        #[cfg(not(ipc))]
        {
            self.inner.name()
        }
    }

    /// Await until other process connects.
    pub async fn connect(self) -> Result<IpcSender<T>, ChannelError> {
        crate::wait(move || self.connect_blocking()).await
    }

    /// Await until other process connects or `deadline` elapses.
    pub async fn connect_deadline(self, deadline: impl Into<Deadline>) -> Result<IpcSender<T>, ChannelError> {
        match crate::with_deadline(self.connect(), deadline).await {
            Ok(r) => r,
            Err(_) => Err(ChannelError::Timeout),
        }
    }

    /// Blocks until other process connects.
    pub fn connect_blocking(self) -> Result<IpcSender<T>, ChannelError> {
        #[cfg(ipc)]
        {
            let (_, sender) = self.server.accept().map_err(ChannelError::disconnected_by)?;
            Ok(sender)
        }
        #[cfg(not(ipc))]
        {
            self.inner.connect_blocking()
        }
    }

    /// Blocks until other process connects or `deadline` elapses.
    pub fn connect_deadline_blocking(self, deadline: impl Into<Deadline>) -> Result<IpcSender<T>, ChannelError> {
        crate::block_on(self.connect_deadline(deadline))
    }
}
impl<T: IpcValue> IpcReceiver<T> {
    /// Connect with a named sender created in another process with [`NamedIpcSender`].
    ///
    /// This must only be called once for the `ipc_sender_name`.
    pub fn connect(ipc_sender_name: impl Into<String>) -> Result<Self, ChannelError> {
        Self::connect_impl(ipc_sender_name.into())
    }
    #[cfg(ipc)]
    fn connect_impl(ipc_sender_name: String) -> Result<Self, ChannelError> {
        let sender = ipc_channel::ipc::IpcSender::<IpcSender<T>>::connect(ipc_sender_name).map_err(ChannelError::disconnected_by)?;
        let (s, r) = ipc_unbounded().map_err(ChannelError::disconnected_by)?;
        crate::channel::with_ipc_serialization(|| sender.send(s)).map_err(ChannelError::disconnected_by)?;
        Ok(r)
    }
    #[cfg(not(ipc))]
    fn connect_impl(ipc_sender_name: String) -> Result<Self, ChannelError> {
        named_channel_fallback::receiver_connect_blocking(&ipc_sender_name)
    }
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

#[cfg(ipc)]
impl From<ipc_channel::ipc::IpcError> for ChannelError {
    fn from(value: ipc_channel::ipc::IpcError) -> Self {
        match value {
            ipc_channel::ipc::IpcError::Disconnected => ChannelError::disconnected(),
            e => ChannelError::disconnected_by(e),
        }
    }
}
#[cfg(ipc)]
impl From<ipc_channel::ipc::TryRecvError> for ChannelError {
    fn from(value: ipc_channel::ipc::TryRecvError) -> Self {
        match value {
            ipc_channel::ipc::TryRecvError::IpcError(ipc_channel::ipc::IpcError::Disconnected) => ChannelError::disconnected(),
            ipc_channel::ipc::TryRecvError::Empty => ChannelError::Timeout,
            e => ChannelError::disconnected_by(e),
        }
    }
}

#[cfg(not(ipc))]
mod named_channel_fallback {
    use std::{
        any::Any,
        collections::HashMap,
        error::Error,
        fmt, mem,
        sync::{Arc, Weak, atomic::AtomicU64},
    };

    use parking_lot::Mutex;
    use zng_txt::{Txt, formatx};

    use crate::channel::{ChannelError, IpcReceiver, IpcSender, IpcValue, Receiver, Sender, ipc_unbounded, rendezvous};

    static NAME_COUNT: AtomicU64 = AtomicU64::new(0);

    type P = (Mutex<Box<dyn Any + Send>>, Sender<()>);
    static PENDING: Mutex<Option<HashMap<Txt, Weak<P>>>> = Mutex::new(None);

    pub struct NamedSender<T: IpcValue> {
        sender: IpcSender<T>,
        name: Txt,
        pending_entry: Arc<P>,
        sig_recv: Receiver<()>,
    }
    impl<T: IpcValue> NamedSender<T> {
        pub fn new() -> Self {
            let i = NAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let name = formatx!("<not-ipc-{}-{i}>", std::process::id());

            let (sender, receiver) = ipc_unbounded::<T>().unwrap();
            let (sig_sender, sig_recv) = rendezvous();

            let s: Box<dyn Any + Send> = Box::new(receiver);
            let pending_entry = Arc::new((Mutex::new(s), sig_sender));
            PENDING
                .lock()
                .get_or_insert_default()
                .insert(name.clone(), Arc::downgrade(&pending_entry));

            Self {
                sender,
                name,
                pending_entry,
                sig_recv,
            }
        }

        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn connect_blocking(self) -> Result<IpcSender<T>, ChannelError> {
            self.sig_recv.recv_blocking()?;
            Ok(self.sender)
        }
    }

    pub fn receiver_connect_blocking<T: IpcValue>(name: &str) -> Result<IpcReceiver<T>, ChannelError> {
        let mut p = PENDING.lock();
        let p = p.get_or_insert_default();
        p.retain(|_, v| v.strong_count() > 0);
        match p.remove(name) {
            Some(e) => match e.upgrade() {
                Some(e) => {
                    let recv = mem::replace(&mut *e.0.lock(), Box::new(()));
                    e.1.send_blocking(());
                    match recv.downcast::<IpcReceiver<T>>() {
                        Ok(r) => Ok(*r),
                        Err(_) => Err(ChannelError::disconnected_by(TypeMismatchError)),
                    }
                }
                None => Err(ChannelError::disconnected()),
            },
            None => Err(ChannelError::disconnected()),
        }
    }

    pub struct NamedReceiver<T: IpcValue> {
        receiver: IpcReceiver<T>,
        name: Txt,
        pending_entry: Arc<P>,
        sig_recv: Receiver<()>,
    }
    impl<T: IpcValue> NamedReceiver<T> {
        pub fn new() -> Self {
            let i = NAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let name = formatx!("<not-ipc-{}-{i}>", std::process::id());

            let (sender, receiver) = ipc_unbounded::<T>().unwrap();
            let (sig_sender, sig_recv) = rendezvous();

            let s: Box<dyn Any + Send> = Box::new(sender);
            let pending_entry = Arc::new((Mutex::new(s), sig_sender));
            PENDING
                .lock()
                .get_or_insert_default()
                .insert(name.clone(), Arc::downgrade(&pending_entry));

            Self {
                receiver,
                name,
                pending_entry,
                sig_recv,
            }
        }

        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn connect_blocking(self) -> Result<IpcReceiver<T>, ChannelError> {
            self.sig_recv.recv_blocking()?;
            Ok(self.receiver)
        }
    }

    pub fn sender_connect_blocking<T: IpcValue>(name: &str) -> Result<IpcSender<T>, ChannelError> {
        let mut p = PENDING.lock();
        let p = p.get_or_insert_default();
        p.retain(|_, v| v.strong_count() > 0);
        match p.remove(name) {
            Some(e) => match e.upgrade() {
                Some(e) => {
                    let recv = mem::replace(&mut *e.0.lock(), Box::new(()));
                    e.1.send(());
                    match recv.downcast::<IpcSender<T>>() {
                        Ok(r) => Ok(*r),
                        Err(_) => Err(ChannelError::disconnected_by(TypeMismatchError)),
                    }
                }
                None => Err(ChannelError::disconnected()),
            },
            None => Err(ChannelError::disconnected()),
        }
    }

    #[derive(Debug)]
    struct TypeMismatchError;
    impl fmt::Display for TypeMismatchError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "named channel type does not match")
        }
    }
    impl Error for TypeMismatchError {}
}
