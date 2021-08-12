use serde::*;
use webrender::api::{units::LayoutSize, BuiltDisplayListDescriptor, PipelineId};

#[derive(Serialize, Deserialize)]
pub enum Request {
    Start(StartRequest),
    OpenWindow(WindowRequest),
    CloseWindow(u32),
    Shutdown,
}

#[derive(Serialize, Deserialize)]
pub struct StartRequest {
    pub device_events: bool,
}

#[derive(Serialize, Deserialize)]
pub struct WindowRequest {
    pub title: String,
    pub pos: (u32, u32),
    pub size: (u32, u32),
    pub frame: (PipelineId, LayoutSize, (Vec<u8>, BuiltDisplayListDescriptor)),
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Started,
    WindowOpened(u32),
    WindowResized(u32, (u32, u32)),
    WindowMoved(u32, (i32, i32)),
    WindowCloseRequested(u32),
    WindowClosed(u32),
    WindowNotFound(u32),
}

pub const MAX_RESPONSE_SIZE: u32 = 1024u32.pow(2) * 20;
pub const MAX_REQUEST_SIZE: u32 = 1024u32.pow(2) * 20;
