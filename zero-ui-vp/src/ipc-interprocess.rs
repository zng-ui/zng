use crate::{Ev, Request, Response};

use interprocess::local_socket::*;

pub use std::io::Result;
use std::{io::ErrorKind, thread, time::Duration};

/// Generate a unique server name.
pub(crate) fn new_server_name() -> String {
    let unique = uuid::Uuid::new_v4().to_simple();
    if cfg!(windows) {
        format!("\\\\.\\pipe\\ui-vp-{}", unique)
    } else {
        format!("/tmp/ui-vp-{}", unique)
    }
}

/// Start the view-process server and waits for `(request, response, event)`.
pub(crate) fn connect_view_process(server_name: &str) -> (RequestReceiver, ResponseSender, EvSender) {
    let req_listener = LocalSocketListener::bind(format!("{}-req", server_name)).expect("failed to bind request channel");
    let rsp_listener = LocalSocketListener::bind(format!("{}-rsp", server_name)).expect("failed to bind response channel");
    let evt_listener = LocalSocketListener::bind(format!("{}-evt", server_name)).expect("failed to bind event channel");

    let req = RequestReceiver(req_listener.accept().expect("failed to connect request channel"));
    let rsp = ResponseSender(rsp_listener.accept().expect("failed to connect response channel"));
    let evt = EvSender(evt_listener.accept().expect("failed to connect event channel"));

    (req, rsp, evt)
}

/// Connect to the view-process server 3 times, `(request, response, event)`.
pub(crate) fn connect_app_process(server_name: &str) -> (RequestSender, ResponseReceiver, EvReceiver) {
    fn try_connect(name: String) -> LocalSocketStream {
        let delay = Duration::from_millis(100);
        let mut retries = 10;

        // view-process can be initializing, so we retry for a bit.
        loop {
            match LocalSocketStream::connect(name.as_str()) {
                Ok(s) => return s,
                Err(e) if e.kind() == ErrorKind::NotFound && retries > 0 => {
                    retries -= 1;
                    thread::sleep(delay);
                }
                Err(e) => panic!("failed to connect to the `{}` channel, {:?}", name, e),
            }
        }
    }

    let req = RequestSender(try_connect(format!("{}-req", server_name)));
    let rsp = ResponseReceiver(try_connect(format!("{}-rsp", server_name)));
    let evt = EvReceiver(try_connect(format!("{}-evt", server_name)));

    (req, rsp, evt)
}

pub(crate) struct RequestSender(LocalSocketStream);
impl RequestSender {
    pub fn send(&mut self, req: &Request) -> Result<()> {
        bincode::serialize_into(&mut self.0, req).map_err(handle_error)
    }
}
pub(crate) struct RequestReceiver(LocalSocketStream);
impl RequestReceiver {
    pub fn recv(&mut self) -> Result<Request> {
        bincode::deserialize_from(&mut self.0).map_err(handle_error)
    }
}

pub(crate) struct ResponseSender(LocalSocketStream);
impl ResponseSender {
    pub fn send(&mut self, rsp: &Response) -> Result<()> {
        bincode::serialize_into(&mut self.0, rsp).map_err(handle_error)
    }
}
pub(crate) struct ResponseReceiver(LocalSocketStream);
impl ResponseReceiver {
    pub fn recv(&mut self) -> Result<Response> {
        bincode::deserialize_from(&mut self.0).map_err(handle_error)
    }
}

pub(crate) struct EvSender(LocalSocketStream);
impl EvSender {
    pub fn send(&mut self, ev: &Ev) -> Result<()> {
        bincode::serialize_into(&mut self.0, ev).map_err(handle_error)
    }
}
pub(crate) struct EvReceiver(LocalSocketStream);
impl EvReceiver {
    pub fn recv(&mut self) -> Result<Ev> {
        bincode::deserialize_from(&mut self.0).map_err(handle_error)
    }
}

#[allow(clippy::boxed_local)]
fn handle_error(e: bincode::Error) -> std::io::Error {
    match *e {
        bincode::ErrorKind::Io(e) => e,
        e => panic!("serialization error {:?}", e),
    }
}
