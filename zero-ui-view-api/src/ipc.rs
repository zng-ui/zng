use std::{fmt, io, thread, time::Duration};

use crate::{AnyResult, Event, Request, Response};

use ipc_channel::ipc::IpcOneShotServer;

pub(crate) type IpcResult<T> = std::result::Result<T, Disconnected>;

/// Channel disconnected error.
#[derive(Debug)]
pub struct Disconnected;
impl fmt::Display for Disconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ipc channel disconnected")
    }
}
impl std::error::Error for Disconnected {}

pub use ipc_channel::ipc::{bytes_channel, IpcBytesReceiver, IpcBytesSender, IpcSharedMemory};
use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};

/// Call `new`, then spawn the view-process using the `name` then call `connect`.
pub(crate) struct AppInit {
    // (
    //    RequestSender,
    //    Workaround-sender-for-response-channel,
    //    EventReceiver,
    // )
    #[allow(clippy::type_complexity)]
    server: IpcOneShotServer<(IpcSender<Request>, IpcSender<(IpcSender<Response>, IpcSender<Event>)>)>,
    name: String,
}
impl AppInit {
    pub fn new() -> Self {
        let (server, name) = IpcOneShotServer::new().expect("failed to create init channel");
        AppInit { server, name }
    }

    /// Unique name for the view-process to find this channel.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Tries to connect to the view-process and receive the actual channels.
    pub fn connect(self) -> AnyResult<(RequestSender, ResponseReceiver, EventReceiver)> {
        let (init_sender, init_recv) = flume::bounded(1);
        let handle = thread::spawn(move || {
            let r = self.server.accept();
            let _ = init_sender.send(r);
        });

        let (_, (req_sender, chan_sender)) = init_recv.recv_timeout(Duration::from_secs(5)).map_err(|e| match e {
            flume::RecvTimeoutError::Timeout => "timeout, did not connect in 5 seconds",
            flume::RecvTimeoutError::Disconnected => {
                std::panic::resume_unwind(handle.join().unwrap_err());
            }
        })??;
        let (rsp_sender, rsp_recv) = channel()?;
        let (evt_sender, evt_recv) = channel()?;
        chan_sender.send((rsp_sender, evt_sender))?;
        Ok((RequestSender(req_sender), ResponseReceiver(rsp_recv), EventReceiver(evt_recv)))
    }
}

/// Start the view-process server and waits for `(request, response, event)`.
pub fn connect_view_process(server_name: String) -> IpcResult<ViewChannels> {
    let app_init_sender = IpcSender::connect(server_name).expect("failed to connect to init channel");

    let (req_sender, req_recv) = channel().map_err(handle_io_error)?;
    // Large messages can only be received in a receiver created in the same process that is receiving (on Windows)
    // so we create a channel to transfer the response and event senders.
    // See issue: https://github.com/servo/ipc-channel/issues/277
    let (chan_sender, chan_recv) = channel().map_err(handle_io_error)?;

    app_init_sender.send((req_sender, chan_sender)).map_err(handle_send_error)?;
    let (rsp_sender, evt_sender) = chan_recv.recv().map_err(handle_recv_error)?;

    Ok(ViewChannels {
        request_receiver: RequestReceiver(req_recv),
        response_sender: ResponseSender(rsp_sender),
        event_sender: EventSender(evt_sender),
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

pub(crate) struct RequestSender(IpcSender<Request>);
impl RequestSender {
    pub fn send(&mut self, req: Request) -> IpcResult<()> {
        self.0.send(req).map_err(handle_send_error)
    }
}

/// Requests channel end-point.
///
/// View-process implementers must receive [`Request`], call [`Api::respond`] and then use a [`ResponseSender`]
/// to send back the response.
///
/// [`Api::respond`]: crate::Api::respond
pub struct RequestReceiver(IpcReceiver<Request>);
impl RequestReceiver {
    /// Receive one [`Request`].
    pub fn recv(&mut self) -> IpcResult<Request> {
        self.0.recv().map_err(handle_recv_error)
    }
}

/// Responses channel entry-point.
///
/// View-process implementers must send [`Response`] returned by [`Api::respond`] using this sender.
///
/// Requests are received using [`RequestReceiver`] a response must be send for each request, synchronously.
///
/// [`Api::respond`]: crate::Api::respond
pub struct ResponseSender(IpcSender<Response>);
impl ResponseSender {
    /// Send a response.
    pub fn send(&mut self, rsp: Response) -> IpcResult<()> {
        self.0.send(rsp).map_err(handle_send_error)
    }
}
pub(crate) struct ResponseReceiver(IpcReceiver<Response>);
impl ResponseReceiver {
    pub fn recv(&mut self) -> IpcResult<Response> {
        self.0.recv().map_err(handle_recv_error)
    }
}

/// Event channel entry-point.
///
/// View-process implementers must send [`Event`] messages using this sender. The events
/// can be asynchronous, not related to the [`Api::respond`] calls.
///
/// [`Api::respond`]: crate::Api::respond
pub struct EventSender(IpcSender<Event>);
impl EventSender {
    /// Send an event notification.
    pub fn send(&mut self, ev: Event) -> IpcResult<()> {
        self.0.send(ev).map_err(handle_send_error)
    }
}
pub(crate) struct EventReceiver(IpcReceiver<Event>);
impl EventReceiver {
    pub fn recv(&mut self) -> IpcResult<Event> {
        self.0.recv().map_err(handle_recv_error)
    }
}

fn handle_recv_error(e: ipc_channel::ipc::IpcError) -> Disconnected {
    match e {
        ipc_channel::ipc::IpcError::Disconnected => Disconnected,
        e => panic!("IO or bincode error: {:?}", e),
    }
}

#[allow(clippy::boxed_local)]
fn handle_send_error(e: ipc_channel::Error) -> Disconnected {
    match *e {
        ipc_channel::ErrorKind::Io(e) if e.kind() == io::ErrorKind::BrokenPipe => Disconnected,
        ipc_channel::ErrorKind::Io(e) => panic!("unexpected IO error: {:?}", e),
        e => panic!("serialization error: {:?}", e),
    }
}

fn handle_io_error(e: io::Error) -> Disconnected {
    match e.kind() {
        io::ErrorKind::BrokenPipe => Disconnected,
        e => panic!("unexpected IO error: {:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;
    use crate::RequestData;

    #[test]
    fn disconnect_recv() {
        let app = AppInit::new();

        let name = app.name().to_owned();

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

        let name = app.name().to_owned();

        let view = thread::spawn(move || {
            let _channels = connect_view_process(name);
        });

        let (mut request_sender, _response_recv, _event_recv) = app.connect().unwrap();

        view.join().unwrap();

        let _ = request_sender.send(Request(RequestData::api_version {})).unwrap_err();
    }
}
