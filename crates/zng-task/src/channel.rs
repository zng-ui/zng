//! Communication channels.
//!
//! Use [`bounded`], [`unbounded`] and [`rendezvous`] to create channels for use across threads in the same process.
//! Use [`ipc_unbounded`] to create channels that work across processes.
//!
//! # Examples
//!
//! ```no_run
//! use zng_task::{self as task, channel};
//! # use zng_unit::*;
//!
//! let (sender, receiver) = channel::bounded(5);
//!
//! task::spawn(async move {
//!     task::deadline(5.secs()).await;
//!     if let Err(e) = sender.send("Data!").await {
//!         eprintln!("no receiver connected, did not send message: '{e}'")
//!     }
//! });
//! task::spawn(async move {
//!     match receiver.recv().await {
//!         Ok(msg) => println!("{msg}"),
//!         Err(_) => eprintln!("no message in channel and no sender connected"),
//!     }
//! });
//! ```
//!
//! [`flume`]: https://docs.rs/flume
//! [`ipc-channel`]: https://docs.rs/ipc-channel

use std::{fmt, sync::Arc, time::Duration};

use zng_time::{Deadline, INSTANT};

mod ipc;
pub use ipc::{IpcReceiver, IpcSender, IpcValue, NamedIpcReceiver, NamedIpcSender, ipc_unbounded};

mod ipc_bytes;
pub use ipc_bytes::{
    IpcBytes, IpcBytesCast, IpcBytesCastIntoIter, IpcBytesIntoIter, IpcBytesMut, IpcBytesMutCast, IpcBytesWriter, IpcBytesWriterBlocking,
    WeakIpcBytes,
};

#[cfg(ipc)]
pub use ipc_bytes::{is_ipc_serialization, with_ipc_serialization};
use zng_txt::ToTxt;

/// The transmitting end of a channel.
///
/// Use [`unbounded`], [`bounded`] or [`rendezvous`] to create a channel.
pub struct Sender<T>(flume::Sender<T>);
impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sender<{}>", pretty_type_name::pretty_type_name::<T>())
    }
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender(self.0.clone())
    }
}
impl<T> From<flume::Sender<T>> for Sender<T> {
    fn from(s: flume::Sender<T>) -> Self {
        Sender(s)
    }
}
impl<T> From<Sender<T>> for flume::Sender<T> {
    fn from(s: Sender<T>) -> Self {
        s.0
    }
}
impl<T> Sender<T> {
    /// Send a value into the channel.
    ///
    /// Waits until there is space in the channel buffer.
    ///
    /// Returns an error if all receivers have been dropped.
    pub async fn send(&self, msg: T) -> Result<(), ChannelError> {
        self.0.send_async(msg).await?;
        Ok(())
    }

    /// Send a value into the channel.
    ///
    /// Waits until there is space in the channel buffer or the `deadline` is reached.
    ///
    /// Returns an error if all receivers have been dropped or the `deadline` is reached. The `msg` is lost in case of timeout.
    pub async fn send_deadline(&self, msg: T, deadline: impl Into<Deadline>) -> Result<(), ChannelError> {
        match super::with_deadline(self.send(msg), deadline).await {
            Ok(r) => match r {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            },
            Err(_) => Err(ChannelError::Timeout),
        }
    }

    /// Send a value into the channel.
    ///
    /// Blocks until there is space in the channel buffer.
    ///
    /// Returns an error if all receivers have been dropped.
    pub fn send_blocking(&self, msg: T) -> Result<(), ChannelError> {
        self.0.send(msg)?;
        Ok(())
    }

    /// Send a value into the channel.
    ///
    /// Blocks until there is space in the channel buffer or the `deadline` is reached.
    ///
    /// Returns an error if all receivers have been dropped or the `deadline` is reached. The `msg` is lost in case of timeout.
    pub fn send_deadline_blocking(&self, msg: T, deadline: impl Into<Deadline>) -> Result<(), ChannelError> {
        super::block_on(self.send_deadline(msg, deadline))
    }

    /// Gets if the channel has no pending messages.
    ///
    /// Note that [`rendezvous`] channels are always empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The receiving end of a channel.
///
/// Use [`unbounded`], [`bounded`] or [`rendezvous`] to create a channel.
///
/// # Work Stealing
///
/// Cloning the receiver **does not** turn this channel into a broadcast channel.
/// Each message will only be received by a single receiver. You can use this to
/// to implement work stealing.
pub struct Receiver<T>(flume::Receiver<T>);
impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Receiver<{}>", pretty_type_name::pretty_type_name::<T>())
    }
}
impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Receiver(self.0.clone())
    }
}
impl<T> Receiver<T> {
    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped.
    pub async fn recv(&self) -> Result<T, ChannelError> {
        let r = self.0.recv_async().await?;
        Ok(r)
    }

    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `deadline` is reached.
    pub async fn recv_deadline(&self, deadline: impl Into<Deadline>) -> Result<T, ChannelError> {
        match super::with_deadline(self.recv(), deadline).await {
            Ok(r) => match r {
                Ok(m) => Ok(m),
                e => e,
            },
            Err(_) => Err(ChannelError::Timeout),
        }
    }

    /// Wait for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped.
    pub fn recv_blocking(&self) -> Result<T, ChannelError> {
        let r = self.0.recv()?;
        Ok(r)
    }

    /// Block for an incoming value from the channel associated with this receiver.
    ///
    /// Returns an error if all senders have been dropped or the `deadline` is reached.
    pub fn recv_deadline_blocking(&self, deadline: impl Into<Deadline>) -> Result<T, ChannelError> {
        self.recv_deadline_blocking_impl(deadline.into())
    }
    fn recv_deadline_blocking_impl(&self, deadline: Deadline) -> Result<T, ChannelError> {
        // Improve timeout precision because this is used in the app main loop and timers are implemented using it

        const WORST_SLEEP_ERR: Duration = Duration::from_millis(if cfg!(windows) { 20 } else { 10 });
        const WORST_SPIN_ERR: Duration = Duration::from_millis(if cfg!(windows) { 2 } else { 1 });

        loop {
            if let Some(d) = deadline.0.checked_duration_since(INSTANT.now()) {
                if matches!(INSTANT.mode(), zng_time::InstantMode::Manual) {
                    // manual time is probably desynced from `Instant`, so we use `recv_timeout` that
                    // is slightly less precise, but an app in manual mode probably does not care.
                    match self.0.recv_timeout(d.checked_sub(WORST_SLEEP_ERR).unwrap_or_default()) {
                        Err(flume::RecvTimeoutError::Timeout) => continue, // continue to try_recv spin
                        interrupt => return interrupt.map_err(ChannelError::from),
                    }
                } else if d > WORST_SLEEP_ERR {
                    // probably sleeps here.
                    #[cfg(not(target_arch = "wasm32"))]
                    match self.0.recv_deadline(deadline.0.checked_sub(WORST_SLEEP_ERR).unwrap().into()) {
                        Err(flume::RecvTimeoutError::Timeout) => continue, // continue to try_recv spin
                        interrupt => return interrupt.map_err(ChannelError::from),
                    }

                    #[cfg(target_arch = "wasm32")] // this actually panics because flume tries to use Instant::now
                    match self.0.recv_timeout(d.checked_sub(WORST_SLEEP_ERR).unwrap_or_default()) {
                        Err(flume::RecvTimeoutError::Timeout) => continue, // continue to try_recv spin
                        interrupt => return interrupt.map_err(ChannelError::from),
                    }
                } else if d > WORST_SPIN_ERR {
                    let spin_deadline = Deadline(deadline.0.checked_sub(WORST_SPIN_ERR).unwrap());

                    // try_recv spin
                    while !spin_deadline.has_elapsed() {
                        match self.0.try_recv() {
                            Err(flume::TryRecvError::Empty) => std::thread::yield_now(),
                            interrupt => return interrupt.map_err(ChannelError::from),
                        }
                    }
                    continue; // continue to timeout spin
                } else {
                    // last millis spin for better timeout precision
                    while !deadline.has_elapsed() {
                        std::thread::yield_now();
                    }
                    return Err(ChannelError::Timeout);
                }
            } else {
                return Err(ChannelError::Timeout);
            }
        }
    }

    /// Returns the next incoming message in the channel or `None`.
    pub fn try_recv(&self) -> Result<Option<T>, ChannelError> {
        match self.0.try_recv() {
            Ok(r) => Ok(Some(r)),
            Err(e) => match e {
                flume::TryRecvError::Empty => Ok(None),
                flume::TryRecvError::Disconnected => Err(ChannelError::disconnected()),
            },
        }
    }

    /// Create a blocking iterator that receives until a channel error.
    pub fn iter(&self) -> impl Iterator<Item = T> {
        self.0.iter()
    }

    /// Iterate over all the pending incoming messages in the channel, until the channel is empty or error.
    pub fn try_iter(&self) -> impl Iterator<Item = T> {
        self.0.try_iter()
    }

    /// Gets if the channel has no pending messages.
    ///
    /// Note that [`rendezvous`] channels are always empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Create a channel with no maximum capacity.
///
/// Unbound channels always [`send`] messages immediately, never yielding on await.
/// If the messages are not [received] they accumulate in the channel buffer.
///
/// # Examples
///
/// The example [spawns] two parallel tasks, the receiver task takes a while to start receiving but then
/// rapidly consumes all messages in the buffer and new messages as they are send.
///
/// ```no_run
/// use zng_task::{self as task, channel};
/// # use zng_unit::*;
///
/// let (sender, receiver) = channel::unbounded();
///
/// task::spawn(async move {
///     for msg in ["Hello!", "Are you still there?"].into_iter().cycle() {
///         task::deadline(300.ms()).await;
///         if let Err(e) = sender.send(msg).await {
///             eprintln!("no receiver connected, the message `{e}` was not send");
///             break;
///         }
///     }
/// });
/// task::spawn(async move {
///     task::deadline(5.secs()).await;
///
///     loop {
///         match receiver.recv().await {
///             Ok(msg) => println!("{msg}"),
///             Err(_) => {
///                 eprintln!("no message in channel and no sender connected");
///                 break;
///             }
///         }
///     }
/// });
/// ```
///
/// Note that you don't need to `.await` on [`send`] as there is always space in the channel buffer.
///
/// [`send`]: Sender::send
/// [received]: Receiver::recv
/// [spawns]: crate::spawn
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    let (s, r) = flume::unbounded();
    (Sender(s), Receiver(r))
}

/// Create a channel with a maximum capacity.
///
/// Bounded channels [`send`] until the channel reaches its capacity then it awaits until a message
/// is [received] before sending another message.
///
/// # Examples
///
/// The example [spawns] two parallel tasks, the receiver task takes a while to start receiving but then
/// rapidly consumes the 2 messages in the buffer and unblocks the sender to send more messages.
///
/// ```no_run
/// use zng_task::{self as task, channel};
/// # use zng_unit::*;
///
/// let (sender, receiver) = channel::bounded(2);
///
/// task::spawn(async move {
///     for msg in ["Hello!", "Data!"].into_iter().cycle() {
///         task::deadline(300.ms()).await;
///         if let Err(e) = sender.send(msg).await {
///             eprintln!("no receiver connected, the message `{e}` was not send");
///             break;
///         }
///     }
/// });
/// task::spawn(async move {
///     task::deadline(5.secs()).await;
///
///     loop {
///         match receiver.recv().await {
///             Ok(msg) => println!("{msg}"),
///             Err(_) => {
///                 eprintln!("no message in channel and no sender connected");
///                 break;
///             }
///         }
///     }
/// });
/// ```
///
/// [`send`]: Sender::send
/// [received]: Receiver::recv
/// [spawns]: crate::spawn
pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (s, r) = flume::bounded(capacity);
    (Sender(s), Receiver(r))
}

/// Create a [`bounded`] channel with `0` capacity.
///
/// Rendezvous channels always awaits until the message is [received] to *return* from [`send`], there is no buffer.
///
/// # Examples
///
/// The example [spawns] two parallel tasks, the sender and receiver *handshake* when transferring the message, the
/// receiver takes 2 seconds to receive, so the sender takes 2 seconds to send.
///
/// ```no_run
/// use zng_task::{self as task, channel};
/// # use zng_unit::*;
/// # use std::time::*;
/// # use zng_time::*;
///
/// let (sender, receiver) = channel::rendezvous();
///
/// task::spawn(async move {
///     loop {
///         let t = INSTANT.now();
///
///         if let Err(e) = sender.send("the stuff").await {
///             eprintln!(r#"failed to send "{}", no receiver connected"#, e);
///             break;
///         }
///
///         assert!(t.elapsed() >= 2.secs());
///     }
/// });
/// task::spawn(async move {
///     loop {
///         task::deadline(2.secs()).await;
///
///         match receiver.recv().await {
///             Ok(msg) => println!(r#"got "{msg}""#),
///             Err(_) => {
///                 eprintln!("no sender connected");
///                 break;
///             }
///         }
///     }
/// });
/// ```
///
/// [`send`]: Sender::send
/// [received]: Receiver::recv
/// [spawns]: crate::spawn
pub fn rendezvous<T>() -> (Sender<T>, Receiver<T>) {
    bounded::<T>(0)
}

/// Error during channel send or receive.
#[derive(Debug, Clone)]
pub enum ChannelError {
    /// Channel has disconnected.
    Disconnected {
        /// Inner error that caused disconnection.
        ///
        /// Is `None` if disconnection was due to endpoint dropping or if the error happened at the other endpoint.
        cause: Option<Arc<dyn std::error::Error + Send + Sync + 'static>>,
    },
    /// Deadline elapsed before message could be send/received.
    Timeout,
}
impl ChannelError {
    /// Channel has disconnected due to endpoint drop.
    pub fn disconnected() -> Self {
        ChannelError::Disconnected { cause: None }
    }

    /// New from other `error`.
    pub fn disconnected_by(cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        ChannelError::Disconnected {
            cause: Some(Arc::new(cause)),
        }
    }
}
impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelError::Disconnected { cause: source } => match source {
                Some(e) => write!(f, "channel disconnected due to, {e}"),
                None => write!(f, "channel disconnected"),
            },
            ChannelError::Timeout => write!(f, "deadline elapsed before message could be transferred"),
        }
    }
}
impl std::error::Error for ChannelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Disconnected { cause: Some(e) } = self {
            Some(e)
        } else {
            None
        }
    }
}
impl PartialEq for ChannelError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Disconnected { cause: l_cause }, Self::Disconnected { cause: r_cause }) => match (l_cause, r_cause) {
                (None, None) => true,
                (Some(a), Some(b)) => a.to_txt() == b.to_txt(),
                _ => false,
            },
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for ChannelError {}
impl From<flume::RecvError> for ChannelError {
    fn from(value: flume::RecvError) -> Self {
        match value {
            flume::RecvError::Disconnected => ChannelError::disconnected(),
        }
    }
}
impl From<flume::RecvTimeoutError> for ChannelError {
    fn from(value: flume::RecvTimeoutError) -> Self {
        match value {
            flume::RecvTimeoutError::Timeout => ChannelError::Timeout,
            flume::RecvTimeoutError::Disconnected => ChannelError::disconnected(),
        }
    }
}
impl<T> From<flume::SendError<T>> for ChannelError {
    fn from(_: flume::SendError<T>) -> Self {
        ChannelError::disconnected()
    }
}
impl From<flume::TryRecvError> for ChannelError {
    fn from(value: flume::TryRecvError) -> Self {
        match value {
            flume::TryRecvError::Empty => ChannelError::Timeout,
            flume::TryRecvError::Disconnected => ChannelError::disconnected(),
        }
    }
}
