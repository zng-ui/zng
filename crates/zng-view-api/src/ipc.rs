//! IPC types.

use std::{fmt, ops::Deref, time::Duration};

use crate::{AnyResult, Event, Request, Response};

#[cfg(ipc)]
use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender, channel};

#[cfg(not(ipc))]
use flume::unbounded as channel;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use zng_txt::Txt;

pub(crate) type IpcResult<T> = std::result::Result<T, ViewChannelError>;

/// Bytes sender.
///
/// Use [`bytes_channel`] to create.
#[cfg_attr(ipc, derive(serde::Serialize, serde::Deserialize))]
pub struct IpcBytesSender {
    #[cfg(ipc)]
    sender: ipc_channel::ipc::IpcBytesSender,
    #[cfg(not(ipc))]
    sender: flume::Sender<Vec<u8>>,
}
impl IpcBytesSender {
    /// Send a byte package.
    pub fn send(&self, bytes: Vec<u8>) -> IpcResult<()> {
        #[cfg(ipc)]
        {
            self.sender.send(&bytes).map_err(handle_io_error)
        }

        #[cfg(not(ipc))]
        self.sender.send(bytes).map_err(handle_send_error)
    }
}
impl fmt::Debug for IpcBytesSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytesSender")
    }
}

/// Bytes receiver.
///
/// Use [`bytes_channel`] to create.
#[cfg_attr(ipc, derive(serde::Serialize, serde::Deserialize))]
pub struct IpcBytesReceiver {
    #[cfg(ipc)]
    recv: ipc_channel::ipc::IpcBytesReceiver,
    #[cfg(not(ipc))]
    recv: flume::Receiver<Vec<u8>>,
}
impl IpcBytesReceiver {
    /// Receive a bytes package.
    pub fn recv(&self) -> IpcResult<Vec<u8>> {
        self.recv.recv().map_err(handle_recv_error)
    }
}
impl fmt::Debug for IpcBytesReceiver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytesReceiver")
    }
}

/// Create a bytes channel.
#[cfg(ipc)]
pub fn bytes_channel() -> (IpcBytesSender, IpcBytesReceiver) {
    let (sender, recv) = ipc_channel::ipc::bytes_channel().unwrap();
    (IpcBytesSender { sender }, IpcBytesReceiver { recv })
}

/// Create a bytes channel.
#[cfg(not(ipc))]
pub fn bytes_channel() -> (IpcBytesSender, IpcBytesReceiver) {
    let (sender, recv) = flume::unbounded();
    (IpcBytesSender { sender }, IpcBytesReceiver { recv })
}

#[cfg(not(ipc))]
mod arc_bytes {
    pub fn serialize<S>(bytes: &std::sync::Arc<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_bytes::serialize(&bytes[..], serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::sync::Arc<Vec<u8>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(std::sync::Arc::new(serde_bytes::deserialize(deserializer)?))
    }
}

/// Immutable shared memory that can be send fast over IPC.
///
/// # Parts
///
/// The bytes allocation can be split into multiple parts, in builds with `"ipc"` feature this is required
/// if parts are `> u32::MAX - 1` and [`from_slice`] and [`from_vec`] automatically split. The [`from_split`]
/// constructor can also be used to instantiate with custom split length, this can be used if the data
/// needs contiguous sequencies of a specific size.
///
/// [`from_slice`]: IpcBytes::from_slice
/// [`from_vec`]: IpcBytes::from_vec
/// [`from_split`]: IpcBytes::from_split
#[derive(Clone, Serialize, Deserialize)]
pub struct IpcBytes {
    // `IpcSharedMemory` length must be < 0 on all platforms and < u32::MAX on Windows
    #[cfg(ipc)]
    bytes: Vec<ipc_channel::ipc::IpcSharedMemory>,
    // `IpcSharedMemory` only clones a pointer.
    #[cfg(not(ipc))]
    #[serde(with = "arc_bytes")]
    bytes: std::sync::Arc<Vec<u8>>,
    #[cfg(not(ipc))]
    part_lens: Vec<u32>,
}
/// Pointer equal.
impl PartialEq for IpcBytes {
    #[cfg(not(ipc))]
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.bytes, &other.bytes) && self.part_lens == other.part_lens
            || self.bytes.is_empty() && other.bytes.is_empty()
    }

    #[cfg(ipc)]
    fn eq(&self, other: &Self) -> bool {
        self.bytes.len() == other.bytes.len() && self.bytes.iter().zip(&other.bytes).all(|(s, o)| s.as_ptr() == o.as_ptr())
    }
}
impl IpcBytes {
    const PART_MAX: usize = (u32::MAX - 1) as usize;

    /// Copy the `bytes` to a new shared memory allocation.
    pub fn from_slice(bytes: &[u8]) -> Self {
        #[cfg(ipc)]
        {
            IpcBytes {
                bytes: {
                    let parts = bytes.len().div_ceil(Self::PART_MAX);
                    (0..parts)
                        .map(|p| {
                            let p = p * Self::PART_MAX;
                            ipc_channel::ipc::IpcSharedMemory::from_bytes(&bytes[p..])
                        })
                        .collect()
                },
            }
        }
        #[cfg(not(ipc))]
        {
            if bytes.len() <= Self::PART_MAX {
                IpcBytes {
                    part_lens: vec![bytes.len() as u32],
                    bytes: std::sync::Arc::new(bytes.to_vec()),
                }
            } else {
                let mut part_lens = Vec::with_capacity(2);
                let mut len = bytes.len();
                loop {
                    if len > Self::PART_MAX {
                        part_lens.push(Self::PART_MAX as u32);
                        len = len.saturating_sub(Self::PART_MAX);
                    } else {
                        part_lens.push(len as u32);
                        break;
                    }
                }
                IpcBytes {
                    part_lens,
                    bytes: std::sync::Arc::new(bytes.to_vec()),
                }
            }
        }
    }

    /// If the `"ipc"` feature is enabled copy the bytes to a new shared memory region, if not
    /// just wraps the `bytes` in a shared pointer.
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        #[cfg(ipc)]
        {
            Self::from_slice(&bytes)
        }

        #[cfg(not(ipc))]
        {
            if bytes.len() <= Self::PART_MAX {
                IpcBytes {
                    part_lens: vec![bytes.len() as u32],
                    bytes: std::sync::Arc::new(bytes),
                }
            } else {
                let mut part_lens = Vec::with_capacity(2);
                let mut len = bytes.len();
                loop {
                    if len > Self::PART_MAX {
                        part_lens.push(Self::PART_MAX as u32);
                        len = len.saturating_sub(Self::PART_MAX);
                    } else {
                        part_lens.push(len as u32);
                        break;
                    }
                }
                IpcBytes {
                    part_lens,
                    bytes: std::sync::Arc::new(bytes),
                }
            }
        }
    }

    /// Copy the `bytes` split by `part_lens`.
    ///
    /// Length of each part must be `<=u32::MAX - 1`.
    pub fn from_split(bytes: &[u8], part_lens: Vec<u32>) -> Self {
        #[cfg(ipc)]
        {
            let mut start = 0;
            let mut r = Vec::with_capacity(part_lens.len());
            for len in part_lens {
                let len = len as usize;
                assert!(len <= Self::PART_MAX);
                let end = start + len;
                r.push(ipc_channel::ipc::IpcSharedMemory::from_bytes(&bytes[start..end]));
                start = end;
            }
            Self { bytes: r }
        }
        #[cfg(not(ipc))]
        {
            let mut sum = 0;
            for &len in &part_lens {
                let len = len as usize;
                assert!(len <= Self::PART_MAX);
                sum += len;
            }
            assert_eq!(bytes.len(), sum);
            Self {
                bytes: std::sync::Arc::new(bytes.to_vec()),
                part_lens,
            }
        }
    }

    /// Copy the shared bytes to a new vec.
    ///
    /// If the `"ipc"` feature is not enabled and `self` is the only reference this operation is zero-cost.
    pub fn to_vec(self) -> Vec<u8> {
        #[cfg(ipc)]
        {
            let mut r = Vec::with_capacity(self.len());
            for part in &self.bytes {
                r.extend(&part[..]);
            }
            r
        }
        #[cfg(not(ipc))]
        {
            match std::sync::Arc::try_unwrap(self.bytes) {
                Ok(d) => d,
                Err(a) => a.to_vec(),
            }
        }
    }

    /// Deprecated.
    #[cfg(ipc)]
    #[deprecated = "underlying memory no longer available"]
    pub fn ipc_shared_memory(&self) -> Option<ipc_channel::ipc::IpcSharedMemory> {
        self.bytes.first().cloned()
    }

    /// Deprecated.
    #[cfg(not(ipc))]
    #[deprecated = "underlying memory no longer available"]
    pub fn arc(&self) -> std::sync::Arc<Vec<u8>> {
        self.bytes.clone()
    }

    /// Sum of length of all byte blocks.
    pub fn len(&self) -> usize {
        #[cfg(ipc)]
        {
            self.bytes.iter().map(|p| p.len()).sum()
        }
        #[cfg(not(ipc))]
        {
            self.bytes.len()
        }
    }

    /// If has no bytes.
    ///
    /// Note that empty `IpcBytes` are always equal, but non-empty are equal by reference comparison.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Count of byte blocks.
    ///
    /// Shared memory is segmented in blocks of `u32::MAX - 1` length. In 64-bit builds with `"ipc"` feature large allocations
    /// are split, in builds without `"ipc"` the allocation is a single block.
    ///
    /// Empty is always `0` blocks.
    pub fn parts_len(&self) -> usize {
        #[cfg(ipc)]
        {
            self.bytes.len()
        }
        #[cfg(not(ipc))]
        {
            self.part_lens.len()
        }
    }

    /// Reference the shared memory block.
    pub fn part(&self, i: usize) -> &[u8] {
        #[cfg(ipc)]
        {
            &self.bytes[i]
        }
        #[cfg(not(ipc))]
        {
            let start = self.part_lens[..i].iter().map(|&l| l as usize).sum();
            let end = start + self.part_lens[i] as usize;
            &self.bytes[start..end]
        }
    }

    /// Iterate over shared memory segments.
    pub fn parts(&self, range: impl std::ops::RangeBounds<usize>) -> impl ExactSizeIterator<Item = &[u8]> {
        let start = match range.start_bound() {
            std::ops::Bound::Included(i) => *i,
            std::ops::Bound::Excluded(i) => *i + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let len = self.len();
        let end = match range.end_bound() {
            std::ops::Bound::Included(i) => *i + 1,
            std::ops::Bound::Excluded(i) => *i,
            std::ops::Bound::Unbounded => len,
        };
        assert!(start < end);
        assert!(end <= len);

        #[cfg(ipc)]
        {
            IpcBytesIterator {
                start,
                end,
                parts: &self.bytes,
            }
        }
        #[cfg(not(ipc))]
        {
            IpcBytesIterator {
                start,
                end,
                bytes: &self.bytes,
                part_lens: &self.part_lens,
            }
        }
    }
}

struct IpcBytesIterator<'a> {
    #[cfg(ipc)]
    parts: &'a [ipc_channel::ipc::IpcSharedMemory],
    #[cfg(not(ipc))]
    bytes: &'a [u8],
    #[cfg(not(ipc))]
    part_lens: &'a [u32],

    // inclusive start
    start: usize,
    // exclusive end
    end: usize,
}
#[cfg(ipc)]
impl<'a> IpcBytesIterator<'a> {
    // (part_i, i_in_part)
    fn locate(&self, i: usize) -> Option<(usize, usize)> {
        if i >= self.end {
            return None;
        }
        let mut base = 0;
        for (pi, p) in self.parts.iter().enumerate() {
            let next = base + p.len();
            if i < next {
                return Some((pi, i - base));
            }
            base = next;
        }
        None
    }

    fn part(&self, pi: usize) -> &'a [u8] {
        &self.parts[pi][..]
    }
}
#[cfg(not(ipc))]
impl<'a> IpcBytesIterator<'a> {
    // (part_i, i_in_part)
    fn locate(&self, i: usize) -> Option<(usize, usize)> {
        if i >= self.end {
            return None;
        }
        let mut base = 0;
        for (pi, &p_len) in self.part_lens.iter().enumerate() {
            let next = base + p_len as usize;
            if i < next {
                return Some((pi, i - base));
            }
            base = next;
        }
        None
    }

    fn part(&self, pi: usize) -> &'a [u8] {
        let start = self.part_lens[..pi].iter().map(|l| *l as usize).sum();
        let end = start + self.part_lens[pi] as usize;
        &self.bytes[start..end]
    }
}
impl<'a> Iterator for IpcBytesIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let (start, start_i) = self.locate(self.start)?;
        let (end, end_i) = self.locate(self.end.checked_sub(1)?)?;
        let next = if start == end {
            &self.part(start)[start_i..=end_i]
        } else {
            &self.part(start)[start_i..]
        };
        self.start += next.len();
        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a> ExactSizeIterator for IpcBytesIterator<'a> {
    fn len(&self) -> usize {
        if let Some((s, _)) = self.locate(self.start)
            && let Some((e, _)) = self.locate(self.end - 1)
        {
            e - s + 1
        } else {
            0
        }
    }
}

/// **Deprecated** use `parts_len` and `part`.
impl Deref for IpcBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        #[cfg(ipc)]
        return if let Some(bytes) = self.bytes.first() { bytes } else { &[] };

        #[cfg(not(ipc))]
        &self.bytes
    }
}
impl fmt::Debug for IpcBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytes(<{} bytes>)", self.len())
    }
}

#[cfg(not(ipc))]
type IpcSender<T> = flume::Sender<T>;
#[cfg(not(ipc))]
type IpcReceiver<T> = flume::Receiver<T>;

/// IPC channel with view-process error.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ViewChannelError {
    /// IPC channel disconnected.
    Disconnected,
}
impl fmt::Display for ViewChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ipc channel disconnected")
    }
}
impl std::error::Error for ViewChannelError {}

/// Call `new`, then spawn the view-process using the `name` then call `connect`.
#[cfg(ipc)]
pub(crate) struct AppInit {
    server: IpcOneShotServer<AppInitMsg>,
    name: Txt,
}
#[cfg(ipc)]
impl AppInit {
    pub fn new() -> Self {
        let (server, name) = IpcOneShotServer::new().expect("failed to create init channel");
        AppInit {
            server,
            name: Txt::from_str(&name),
        }
    }

    /// Unique name for the view-process to find this channel.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Tries to connect to the view-process and receive the actual channels.
    pub fn connect(self) -> AnyResult<(RequestSender, ResponseReceiver, EventReceiver)> {
        use crate::view_timeout;

        let (init_sender, init_recv) = flume::bounded(1);
        let handle = std::thread::Builder::new()
            .name("connection-init".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                let r = self.server.accept();
                let _ = init_sender.send(r);
            })
            .expect("failed to spawn thread");

        let timeout = view_timeout();
        let (_, (req_sender, chan_sender)) = init_recv.recv_timeout(Duration::from_secs(timeout)).map_err(|e| match e {
            flume::RecvTimeoutError::Timeout => format!("timeout, did not connect in {timeout}s"),
            flume::RecvTimeoutError::Disconnected => {
                std::panic::resume_unwind(handle.join().unwrap_err());
            }
        })??;
        let (rsp_sender, rsp_recv) = channel()?;
        let (evt_sender, evt_recv) = channel()?;
        chan_sender.send((rsp_sender, evt_sender))?;
        Ok((
            RequestSender(Mutex::new(req_sender)),
            ResponseReceiver(Mutex::new(rsp_recv)),
            EventReceiver(Mutex::new(evt_recv)),
        ))
    }
}

/// Start the view-process server and waits for `(request, response, event)`.
#[cfg(ipc)]
pub fn connect_view_process(server_name: Txt) -> IpcResult<ViewChannels> {
    let _s = tracing::trace_span!("connect_view_process").entered();

    let app_init_sender = IpcSender::connect(server_name.into_owned()).expect("failed to connect to init channel");

    let (req_sender, req_recv) = channel().map_err(handle_io_error)?;
    // Large messages can only be received in a receiver created in the same process that is receiving (on Windows)
    // so we create a channel to transfer the response and event senders.
    // See issue: https://github.com/servo/ipc-channel/issues/277
    let (chan_sender, chan_recv) = channel().map_err(handle_io_error)?;

    app_init_sender.send((req_sender, chan_sender)).map_err(handle_send_error)?;
    let (rsp_sender, evt_sender) = chan_recv.recv().map_err(handle_recv_error)?;

    Ok(ViewChannels {
        request_receiver: RequestReceiver(Mutex::new(req_recv)),
        response_sender: ResponseSender(Mutex::new(rsp_sender)),
        event_sender: EventSender(Mutex::new(evt_sender)),
    })
}

/// (
///    RequestSender,
///    Workaround-sender-for-response-channel,
///    EventReceiver,
/// )
type AppInitMsg = (IpcSender<Request>, IpcSender<(IpcSender<Response>, IpcSender<Event>)>);

#[cfg(not(ipc))]
pub(crate) struct AppInit {
    // (
    //    RequestSender,
    //    Workaround-sender-for-response-channel,
    //    EventReceiver,
    // )
    init: flume::Receiver<AppInitMsg>,
    name: Txt,
}
#[cfg(not(ipc))]
mod name_map {
    use std::{
        collections::HashMap,
        sync::{Mutex, OnceLock},
    };

    use zng_txt::Txt;

    use super::AppInitMsg;

    type Map = Mutex<HashMap<Txt, flume::Sender<AppInitMsg>>>;

    pub fn get() -> &'static Map {
        static MAP: OnceLock<Map> = OnceLock::new();
        MAP.get_or_init(Map::default)
    }
}
#[cfg(not(ipc))]
impl AppInit {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        use zng_txt::formatx;

        static NAME_COUNT: AtomicU32 = AtomicU32::new(0);

        let name = formatx!("<not-ipc-{}>", NAME_COUNT.fetch_add(1, Ordering::Relaxed));

        let (init_sender, init_recv) = flume::bounded(1);

        name_map::get().lock().unwrap().insert(name.clone(), init_sender);

        AppInit { name, init: init_recv }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Tries to connect to the view-process and receive the actual channels.
    pub fn connect(self) -> AnyResult<(RequestSender, ResponseReceiver, EventReceiver)> {
        let (req_sender, chan_sender) = self.init.recv_timeout(Duration::from_secs(5)).map_err(|e| match e {
            flume::RecvTimeoutError::Timeout => "timeout, did not connect in 5s",
            flume::RecvTimeoutError::Disconnected => panic!("disconnected"),
        })?;
        let (rsp_sender, rsp_recv) = flume::unbounded();
        let (evt_sender, evt_recv) = flume::unbounded();
        chan_sender.send((rsp_sender, evt_sender))?;
        Ok((
            RequestSender(Mutex::new(req_sender)),
            ResponseReceiver(Mutex::new(rsp_recv)),
            EventReceiver(Mutex::new(evt_recv)),
        ))
    }
}

/// Start the view-process server and waits for `(request, response, event)`.
#[cfg(not(ipc))]
pub fn connect_view_process(server_name: Txt) -> IpcResult<ViewChannels> {
    let app_init_sender = name_map::get().lock().unwrap().remove(&server_name).unwrap();

    let (req_sender, req_recv) = channel();
    let (chan_sender, chan_recv) = channel();

    app_init_sender.send((req_sender, chan_sender)).map_err(handle_send_error)?;
    let (rsp_sender, evt_sender) = chan_recv.recv().map_err(handle_recv_error)?;

    Ok(ViewChannels {
        request_receiver: RequestReceiver(Mutex::new(req_recv)),
        response_sender: ResponseSender(Mutex::new(rsp_sender)),
        event_sender: EventSender(Mutex::new(evt_sender)),
    })
}

/// Channels that must be used for implementing a view-process.
pub struct ViewChannels {
    /// View implementers must receive requests from this channel, call [`Api::respond`] and then
    /// return the response using the `response_sender`.
    ///
    /// [`Api::respond`]: crate::Api::respond
    pub request_receiver: RequestReceiver,

    /// View implementers must synchronously send one response per request received in `request_receiver`.
    pub response_sender: ResponseSender,

    /// View implements must send events using this channel. Events can be asynchronous.
    pub event_sender: EventSender,
}

pub(crate) struct RequestSender(Mutex<IpcSender<Request>>);
impl RequestSender {
    pub fn send(&mut self, req: Request) -> IpcResult<()> {
        self.0.get_mut().send(req).map_err(handle_send_error)
    }
}

/// Requests channel end-point.
///
/// View-process implementers must receive [`Request`], call [`Api::respond`] and then use a [`ResponseSender`]
/// to send back the response.
///
/// [`Api::respond`]: crate::Api::respond
pub struct RequestReceiver(Mutex<IpcReceiver<Request>>); // Mutex for Sync
impl RequestReceiver {
    /// Receive one [`Request`].
    pub fn recv(&mut self) -> IpcResult<Request> {
        self.0.get_mut().recv().map_err(handle_recv_error)
    }
}

/// Responses channel entry-point.
///
/// View-process implementers must send [`Response`] returned by [`Api::respond`] using this sender.
///
/// Requests are received using [`RequestReceiver`] a response must be send for each request, synchronously.
///
/// [`Api::respond`]: crate::Api::respond
pub struct ResponseSender(Mutex<IpcSender<Response>>); // Mutex for Sync
impl ResponseSender {
    /// Send a response.
    ///
    /// # Panics
    ///
    /// If the `rsp` is not [`must_be_send`].
    ///
    /// [`must_be_send`]: Response::must_be_send
    pub fn send(&mut self, rsp: Response) -> IpcResult<()> {
        assert!(rsp.must_be_send());
        self.0.get_mut().send(rsp).map_err(handle_send_error)
    }
}
pub(crate) struct ResponseReceiver(Mutex<IpcReceiver<Response>>);
impl ResponseReceiver {
    pub fn recv(&mut self) -> IpcResult<Response> {
        self.0.get_mut().recv().map_err(handle_recv_error)
    }
}

/// Event channel entry-point.
///
/// View-process implementers must send [`Event`] messages using this sender. The events
/// can be asynchronous, not related to the [`Api::respond`] calls.
///
/// [`Api::respond`]: crate::Api::respond
pub struct EventSender(Mutex<IpcSender<Event>>);
impl EventSender {
    /// Send an event notification.
    pub fn send(&mut self, ev: Event) -> IpcResult<()> {
        self.0.get_mut().send(ev).map_err(handle_send_error)
    }
}
pub(crate) struct EventReceiver(Mutex<IpcReceiver<Event>>);
impl EventReceiver {
    pub fn recv(&mut self) -> IpcResult<Event> {
        self.0.get_mut().recv().map_err(handle_recv_error)
    }

    #[cfg(ipc)]
    pub fn recv_timeout(&mut self, duration: Duration) -> IpcResult<Option<Event>> {
        match self.0.get_mut().try_recv_timeout(duration) {
            Ok(ev) => Ok(Some(ev)),
            Err(e) => match e {
                ipc_channel::ipc::TryRecvError::IpcError(ipc_error) => Err(handle_recv_error(ipc_error)),
                ipc_channel::ipc::TryRecvError::Empty => Ok(None),
            },
        }
    }

    #[cfg(not(ipc))]
    pub fn recv_timeout(&mut self, duration: Duration) -> IpcResult<Option<Event>> {
        match self.0.get_mut().recv_timeout(duration) {
            Ok(ev) => Ok(Some(ev)),
            Err(e) => match e {
                flume::RecvTimeoutError::Timeout => Ok(None),
                flume::RecvTimeoutError::Disconnected => Err(ViewChannelError::Disconnected),
            },
        }
    }
}

#[cfg(ipc)]
fn handle_recv_error(e: ipc_channel::ipc::IpcError) -> ViewChannelError {
    match e {
        ipc_channel::ipc::IpcError::Disconnected => ViewChannelError::Disconnected,
        e => {
            tracing::error!("IO or bincode error: {e:?}");
            ViewChannelError::Disconnected
        }
    }
}
#[cfg(not(ipc))]
fn handle_recv_error(e: flume::RecvError) -> ViewChannelError {
    match e {
        flume::RecvError::Disconnected => ViewChannelError::Disconnected,
    }
}

#[cfg(ipc)]
#[expect(clippy::boxed_local)]
fn handle_send_error(e: ipc_channel::Error) -> ViewChannelError {
    match *e {
        ipc_channel::ErrorKind::Io(e) => {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return ViewChannelError::Disconnected;
            }
            #[cfg(windows)]
            if e.raw_os_error() == Some(-2147024664) {
                // 0x800700E8 - "The pipe is being closed."
                return ViewChannelError::Disconnected;
            }
            #[cfg(target_os = "macos")]
            if e.kind() == std::io::ErrorKind::NotFound && format!("{e:?}") == "Custom { kind: NotFound, error: SendInvalidDest }" {
                // this error happens in the same test that on Windows is 0x800700E8 and on Ubuntu is BrokenPipe
                return ViewChannelError::Disconnected;
            }
            panic!("unexpected IO error: {e:?}")
        }
        e => panic!("serialization error: {e:?}"),
    }
}

#[cfg(not(ipc))]
fn handle_send_error<T>(_: flume::SendError<T>) -> ViewChannelError {
    ViewChannelError::Disconnected
}

#[cfg(ipc)]
fn handle_io_error(e: std::io::Error) -> ViewChannelError {
    match e.kind() {
        std::io::ErrorKind::BrokenPipe => ViewChannelError::Disconnected,
        e => panic!("unexpected IO error: {e:?}"),
    }
}
