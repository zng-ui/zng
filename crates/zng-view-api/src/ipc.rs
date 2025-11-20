//! IPC types.

use std::time::Duration;

use crate::{AnyResult, Event, Request, Response};

use parking_lot::Mutex;
use zng_task::channel::{self, ChannelError, IpcReceiver, IpcSender};
use zng_txt::Txt;

type AppInitMsg = (
    channel::IpcReceiver<Request>,
    channel::IpcSender<Response>,
    channel::IpcSender<Event>,
);

/// Call `new`, then spawn the view-process using the `name` then call `connect`.
pub(crate) struct AppInit {
    init_sender: channel::NamedIpcSender<AppInitMsg>,
}
impl AppInit {
    pub fn new() -> Self {
        AppInit {
            init_sender: channel::NamedIpcSender::new().expect("failed to create init channel"),
        }
    }

    /// Unique name for the view-process to find this channel.
    pub fn name(&self) -> &str {
        self.init_sender.name()
    }

    /// Tries to connect to the view-process and receive the actual channels.
    pub fn connect(self) -> AnyResult<(RequestSender, ResponseReceiver, EventReceiver)> {
        let mut init_sender = self
            .init_sender
            .connect_deadline_blocking(std::time::Duration::from_secs(crate::view_timeout()))?;

        let (req_sender, req_recv) = channel::ipc_unbounded()?;
        let (rsp_sender, rsp_recv) = channel::ipc_unbounded()?;
        let (evt_sender, evt_recv) = channel::ipc_unbounded()?;
        init_sender.send_blocking((req_recv, rsp_sender, evt_sender))?;
        Ok((
            RequestSender(Mutex::new(req_sender)),
            ResponseReceiver(Mutex::new(rsp_recv)),
            EventReceiver(Mutex::new(evt_recv)),
        ))
    }
}

/// Start the view-process server and waits for `(request, response, event)`.
pub fn connect_view_process(ipc_sender_name: Txt) -> Result<ViewChannels, channel::ChannelError> {
    let _s = tracing::trace_span!("connect_view_process").entered();

    let mut init_recv = channel::IpcReceiver::<AppInitMsg>::connect(ipc_sender_name)?;

    let (req_recv, rsp_sender, evt_sender) = init_recv.recv_deadline_blocking(std::time::Duration::from_secs(crate::view_timeout()))?;

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

type IpcResult<T> = Result<T, ChannelError>;

pub(crate) struct RequestSender(Mutex<IpcSender<Request>>);
impl RequestSender {
    pub fn send(&mut self, req: Request) -> IpcResult<()> {
        let r = self.0.get_mut().send_blocking(req);
        if let Err(e) = &r {
            tracing::error!("request sender error, {e}");
        }
        r
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
        let r = self.0.get_mut().recv_blocking();
        if let Err(e) = &r {
            tracing::error!("request receiver error, {e}");
        }
        r
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
        let r = self.0.get_mut().send_blocking(rsp);
        if let Err(e) = &r {
            tracing::error!("response sender error, {e}");
        }
        r
    }
}
pub(crate) struct ResponseReceiver(Mutex<IpcReceiver<Response>>);
impl ResponseReceiver {
    pub fn recv(&mut self) -> IpcResult<Response> {
        let r = self.0.get_mut().recv_blocking();
        if let Err(e) = &r {
            tracing::error!("response receiver error, {e}");
        }
        r
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
        let r = self.0.get_mut().send_blocking(ev);
        if let Err(e) = &r {
            tracing::error!("event sender error, {e}");
        }
        r
    }
}
pub(crate) struct EventReceiver(Mutex<IpcReceiver<Event>>);
impl EventReceiver {
    pub fn recv(&mut self) -> IpcResult<Event> {
        let r = self.0.get_mut().recv_blocking();
        if let Err(e) = &r {
            tracing::error!("event receiver error, {e}");
        }
        r
    }

    pub fn recv_timeout(&mut self, duration: Duration) -> IpcResult<Event> {
        let r = self.0.get_mut().recv_deadline_blocking(duration);
        if let Err(e) = &r {
            match e {
                ChannelError::Timeout => {}
                e => tracing::error!("event receiver error, {e}"),
            }
        }
        r
    }
}
