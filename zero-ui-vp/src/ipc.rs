use std::{fmt, io};

use crate::{util, Ev, Request, Response};

use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};

pub type Result<T> = std::result::Result<T, Disconnected>;

#[derive(Debug)]
pub struct Disconnected;
impl fmt::Display for Disconnected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ipc channel disconnected")
    }
}
impl std::error::Error for Disconnected {}

/// Call `new`, then spawn the view-process using the `name` then call `connect`.
pub struct AppInit {
    // (
    //    RequestSender,
    //    Workaround-sender-for-response-channel,
    //    EventReceiver,
    // )
    #[allow(clippy::type_complexity)]
    server: IpcOneShotServer<(IpcSender<Request>, IpcSender<IpcSender<Response>>, IpcReceiver<Ev>)>,
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
    pub(crate) fn connect(self) -> util::AnyResult<(RequestSender, ResponseReceiver, EvReceiver)> {
        let (_, (req_sender, rsp_chan_sender, evt_recv)) = self.server.accept()?;
        let (rsp_sender, rsp_recv) = ipc_channel::ipc::channel()?;
        rsp_chan_sender.send(rsp_sender)?;
        Ok((RequestSender(req_sender), ResponseReceiver(rsp_recv), EvReceiver(evt_recv)))
    }
}

/// Start the view-process server and waits for `(request, response, event)`.
pub(crate) fn connect_view_process(server_name: String) -> (RequestReceiver, ResponseSender, EvSender) {
    let app_init_sender = IpcSender::connect(server_name).expect("failed to connect to init channel");

    let (req_sender, req_recv) = ipc_channel::ipc::channel().expect("failed to create request channel");
    // Large messages can only be received in a receiver created in the same process that is receiving (on Windows)
    // so we create a channel to transfer the response sender, because it is the app process that will receive responses.
    // See issue: https://github.com/servo/ipc-channel/issues/277
    let (rsp_chan_sender, rsp_chan_recv) = ipc_channel::ipc::channel().expect("failed to create response channel");
    let (evt_sender, evt_recv) = ipc_channel::ipc::channel().expect("failed to create event channel");

    app_init_sender
        .send((req_sender, rsp_chan_sender, evt_recv))
        .expect("failed to send channels");
    let rsp_sender = rsp_chan_recv.recv().expect("failed to create response channel");

    (RequestReceiver(req_recv), ResponseSender(rsp_sender), EvSender(evt_sender))
}

pub(crate) struct RequestSender(IpcSender<Request>);
impl RequestSender {
    pub fn send(&mut self, req: Request) -> Result<()> {
        self.0.send(req).map_err(handle_send_error)
    }
}
pub(crate) struct RequestReceiver(IpcReceiver<Request>);
impl RequestReceiver {
    pub fn recv(&mut self) -> Result<Request> {
        self.0.recv().map_err(handle_recv_error)
    }
}

pub(crate) struct ResponseSender(IpcSender<Response>);
impl ResponseSender {
    pub fn send(&mut self, rsp: Response) -> Result<()> {
        self.0.send(rsp).map_err(handle_send_error)
    }
}
pub(crate) struct ResponseReceiver(IpcReceiver<Response>);
impl ResponseReceiver {
    pub fn recv(&mut self) -> Result<Response> {
        self.0.recv().map_err(handle_recv_error)
    }
}

pub(crate) struct EvSender(IpcSender<Ev>);
impl EvSender {
    pub fn send(&mut self, ev: Ev) -> Result<()> {
        self.0.send(ev).map_err(handle_send_error)
    }
}
impl Drop for ResponseSender {
    fn drop(&mut self) {
        util::write_trace!("EvSender::drop");
    }
}
pub(crate) struct EvReceiver(IpcReceiver<Ev>);
impl EvReceiver {
    pub fn recv(&mut self) -> Result<Ev> {
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

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn disconnect_recv() {
        let app = AppInit::new();

        let name = app.name().to_owned();

        let view = thread::spawn(move || {
            let (_request_recv, _response_sender, _event_sender) = connect_view_process(name);
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
            let (_request_recv, _response_sender, _event_sender) = connect_view_process(name);
        });

        let (mut request_sender, _response_recv, _event_recv) = app.connect().unwrap();

        view.join().unwrap();

        let _ = request_sender.send(Request::version {}).unwrap_err();
    }
}
