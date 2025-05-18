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
/// # `not(feature="ipc")`
///
/// If the default `"ipc"` feature is disabled this is only a `Vec<u8>`.
#[derive(Clone, Serialize, Deserialize)]
pub struct IpcBytes {
    // `IpcSharedMemory` cannot have zero length, we use `None` in this case.
    #[cfg(ipc)]
    bytes: Option<ipc_channel::ipc::IpcSharedMemory>,
    // `IpcSharedMemory` only clones a pointer.
    #[cfg(not(ipc))]
    #[serde(with = "arc_bytes")]
    bytes: std::sync::Arc<Vec<u8>>,
}
/// Pointer equal.
impl PartialEq for IpcBytes {
    #[cfg(not(ipc))]
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.bytes, &other.bytes)
    }

    #[cfg(ipc)]
    fn eq(&self, other: &Self) -> bool {
        match (&self.bytes, &other.bytes) {
            (None, None) => true,
            (Some(a), Some(b)) => a.as_ptr() == b.as_ptr(),
            _ => false,
        }
    }
}
impl IpcBytes {
    /// Copy the `bytes` to a new shared memory allocation.
    pub fn from_slice(bytes: &[u8]) -> Self {
        IpcBytes {
            #[cfg(ipc)]
            bytes: {
                if bytes.is_empty() {
                    None
                } else {
                    Some(ipc_channel::ipc::IpcSharedMemory::from_bytes(bytes))
                }
            },
            #[cfg(not(ipc))]
            bytes: std::sync::Arc::new(bytes.to_vec()),
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
        IpcBytes {
            bytes: std::sync::Arc::new(bytes),
        }
    }

    /// Copy the shared bytes to a new vec.
    ///
    /// If the `"ipc"` feature is not enabled and `self` is the only reference this operation is zero-cost.
    pub fn to_vec(self) -> Vec<u8> {
        #[cfg(ipc)]
        {
            self.bytes.map(|s| s.to_vec()).unwrap_or_default()
        }
        #[cfg(not(ipc))]
        {
            match std::sync::Arc::try_unwrap(self.bytes) {
                Ok(d) => d,
                Err(a) => a.as_ref().to_vec(),
            }
        }
    }

    /// Returns the underlying shared memory reference, if the bytes are not zero-length.
    #[cfg(ipc)]
    pub fn ipc_shared_memory(&self) -> Option<ipc_channel::ipc::IpcSharedMemory> {
        self.bytes.clone()
    }

    /// Returns the underlying shared reference.
    #[cfg(not(ipc))]
    pub fn arc(&self) -> std::sync::Arc<Vec<u8>> {
        self.bytes.clone()
    }
}
impl Deref for IpcBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        #[cfg(ipc)]
        return if let Some(bytes) = &self.bytes { bytes } else { &[] };

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
        let (init_sender, init_recv) = flume::bounded(1);
        let handle = std::thread::spawn(move || {
            let r = self.server.accept();
            let _ = init_sender.send(r);
        });

        let (_, (req_sender, chan_sender)) = init_recv.recv_timeout(Duration::from_secs(10)).map_err(|e| match e {
            flume::RecvTimeoutError::Timeout => "timeout, did not connect in 10 seconds",
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
            flume::RecvTimeoutError::Timeout => "timeout, did not connect in 5 seconds",
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

#[cfg(all(test, ipc))]
mod tests {
    use std::thread;

    use zng_txt::ToTxt;

    use super::*;
    use crate::RequestData;

    #[test]
    fn disconnect_recv() {
        let app = AppInit::new();

        let name = app.name().to_txt();

        let view = thread::spawn(move || {
            let _channels = connect_view_process(name);
        });

        let (_request_sender, mut response_recv, _event_recv) = app.connect().unwrap();

        view.join().unwrap();

        let _ = response_recv.recv().unwrap_err();
    }

    #[test]
    fn disconnect_send() {
        let app = AppInit::new();

        let name = app.name().to_txt();

        let view = thread::spawn(move || {
            let _channels = connect_view_process(name);
        });

        let (mut request_sender, _response_recv, _event_recv) = app.connect().unwrap();

        view.join().unwrap();

        let _ = request_sender
            .send(Request(RequestData::close {
                id: crate::window::WindowId::INVALID,
            }))
            .unwrap_err();
    }
}
