use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::{env, fmt, thread};

use ipmpsc::{Receiver, Sender, SharedRingBuffer};
use webrender::api::units::LayoutSize;
use webrender::api::{BuiltDisplayListDescriptor, PipelineId};

use crate::{message::*, CHANNEL_VAR, MODE_VAR, VERSION};

/// View Process controller, used in the App Process.
pub struct App {
    process: Child,
    request_sender: Sender,
    response_receiver: Receiver,
    headless: bool,
    windows: Vec<Window>,
    devices: Vec<Device>,
}

impl App {
    /// Start the view process as an instance of the [`current_exe`].
    ///
    /// The `on_event` closure is called in another thread every time the app receives an event.
    ///
    /// [`current_exe`]: std::env::current_exe
    pub fn start<F>(request: StartRequest, on_event: F) -> Self
    where
        F: FnMut(Ev) + Send + 'static,
    {
        Self::start_with(std::env::current_exe().unwrap(), request, on_event)
    }

    /// Start with a custom view process.
    ///
    /// The `on_event` closure is called in another thread every time the app receives an event.
    pub fn start_with<F>(view_process_exe: PathBuf, request: StartRequest, mut on_event: F) -> Self
    where
        F: FnMut(Ev) + Send + 'static,
    {
        let channel_dir = loop {
            let temp_dir = env::temp_dir().join(uuid::Uuid::new_v4().to_simple().to_string());
            match std::fs::create_dir(&temp_dir) {
                Ok(_) => break temp_dir,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("failed to create channel directory: {}", e),
            }
        };

        let response_receiver = Receiver::new(
            SharedRingBuffer::create(channel_dir.join("response").display().to_string().as_str(), MAX_RESPONSE_SIZE)
                .expect("response channel creation failed"),
        );
        let event_receiver = Receiver::new(
            SharedRingBuffer::create(channel_dir.join("event").display().to_string().as_str(), MAX_RESPONSE_SIZE)
                .expect("event channel creation failed"),
        );
        let request_sender = Sender::new(
            SharedRingBuffer::create(channel_dir.join("request").display().to_string().as_str(), MAX_REQUEST_SIZE)
                .expect("request channel creation failed"),
        );

        // create process and spawn it
        let process = Command::new(view_process_exe)
            .env(CHANNEL_VAR, channel_dir)
            .env(MODE_VAR, if request.headless { "headless" } else { "headed" })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("renderer view process failed to spawn");

        let app = App {
            process,
            request_sender,
            response_receiver,
            headless: request.headless,
            windows: vec![],
            devices: vec![],
        };

        match app.request(Request::ProtocolVersion) {
            Response::ProtocolVersion(v) => {
                if v != VERSION {
                    panic!(
                        "`zero-ui-wr {}` is not supported by this controller, ensure /
                    the `zero-ui-wr` crate is the same version in all involved executables",
                        v
                    );
                }
            }
            _ => panic!("view process did not start correctly"),
        }

        match app.request(Request::Start(request)) {
            Response::Started => {
                thread::spawn(move || {
                    while let Ok(ev) = event_receiver.recv() {
                        on_event(ev);
                    }
                });

                app
            }
            _ => panic!("view process did not start correctly"),
        }
    }

    fn request(&self, request: Request) -> Response {
        self.request_sender.send(&request).unwrap();
        self.response_receiver.recv().unwrap()
    }

    /// If is running in headless mode.
    #[inline]
    pub fn headless(&self) -> bool {
        self.headless
    }

    /// Open a window.
    pub fn open_window(&mut self, request: OpenWindowRequest) -> u32 {
        let mut window = Window {
            id: 0,
            title: request.title.clone(),
            pos: request.pos,
            size: request.size,
            visible: request.visible,
            frame: request.frame.clone(),
        };
        match self.request(Request::OpenWindow(request)) {
            Response::WindowOpened(id) => {
                window.id = id;
                self.windows.push(window);
                id
            }
            _ => panic!("view process did not respond correctly"),
        }
    }

    /// Set the window title.
    pub fn set_title(&mut self, window: WinId, title: String) -> Result<(), WindowNotFound> {
        match self.windows.iter_mut().position(|w| w.id == window) {
            Some(i) => match self.request(Request::SetWindowTitle(window, title.clone())) {
                Response::WindowTitleChanged(id) if id == window => {
                    self.windows[i].title = title;
                    Ok(())
                }
                Response::WindowNotFound(id) if id == window => {
                    self.windows.remove(i);
                    Err(WindowNotFound(id))
                }
                _ => panic!("view process did not respond correctly"),
            },
            None => Err(WindowNotFound(window)),
        }
    }

    /// Set the window position.
    pub fn set_position(&mut self, window: WinId, pos: (i32, i32)) -> Result<(), WindowNotFound> {
        match self.windows.iter_mut().position(|w| w.id == window) {
            Some(i) => match self.request(Request::SetWindowPosition(window, pos)) {
                Response::WindowMoved(id, pos) if id == window => {
                    self.windows[i].pos = pos;
                    Ok(())
                }
                Response::WindowNotFound(id) if id == window => {
                    self.windows.remove(i);
                    Err(WindowNotFound(id))
                }
                _ => panic!("view process did not respond correctly"),
            },
            None => Err(WindowNotFound(window)),
        }
    }

    /// Set the window size.
    pub fn set_size(&mut self, window: WinId, size: (u32, u32)) -> Result<(), WindowNotFound> {
        match self.windows.iter_mut().position(|w| w.id == window) {
            Some(i) => match self.request(Request::SetWindowSize(window, size)) {
                Response::WindowResized(id, size) if id == window => {
                    self.windows[i].size = size;
                    Ok(())
                }
                Response::WindowNotFound(id) if id == window => {
                    self.windows.remove(i);
                    Err(WindowNotFound(id))
                }
                _ => panic!("view process did not respond correctly"),
            },
            None => Err(WindowNotFound(window)),
        }
    }

    /// Gets the window size.
    pub fn size(&self, window: WinId) -> Result<(u32, u32), WindowNotFound> {
        match self.windows.iter().find(|w| w.id == window) {
            Some(w) => Ok(w.size),
            None => Err(WindowNotFound(window)),
        }
    }

    /// Gets the window scale factor.
    pub fn scale_factor(&self, window: WinId) -> Result<f32, WindowNotFound> {
        match self.windows.iter().find(|w| w.id == window) {
            Some(w) => Ok(todo!()),
            None => Err(WindowNotFound(window)),
        }
    }

    /// Set the window visibility.
    pub fn set_visible(&mut self, window: WinId, visible: bool) -> Result<(), WindowNotFound> {
        match self.windows.iter_mut().position(|w| w.id == window) {
            Some(i) => match self.request(Request::SetWindowVisible(window, visible)) {
                Response::WindowVisibilityChanged(id, visible) if id == window => {
                    self.windows[i].visible = visible;
                    Ok(())
                }
                Response::WindowNotFound(id) if id == window => {
                    self.windows.remove(i);
                    Err(WindowNotFound(id))
                }
                _ => panic!("view process did not respond correctly"),
            },
            None => Err(WindowNotFound(window)),
        }
    }

    /// Reads the `rect` from the current frame pixels.
    ///
    /// This is a *direct call* to `glReadPixels`, `x` and `y` start
    /// at the bottom-left corner of the rectangle and each *stride*
    /// is a row from bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels(&mut self, window: WinId, rect: [u32; 4]) -> Result<Vec<u8>, WindowNotFound> {
        match self.windows.iter_mut().position(|w| w.id == window) {
            Some(i) => match self.request(Request::ReadPixels(window, rect)) {
                Response::FramePixels(id, pixels) if id == window => Ok(pixels),
                Response::WindowNotFound(id) if id == window => {
                    self.windows.remove(i);
                    Err(WindowNotFound(id))
                }
                _ => panic!("view process did not respond correctly"),
            },
            None => Err(WindowNotFound(window)),
        }
    }

    /// Close the window.
    pub fn close_window(&mut self, window: WinId) -> Result<(), WindowNotFound> {
        match self.windows.iter().position(|w| w.id == window) {
            Some(i) => {
                self.windows.remove(i);
            }
            None => return Err(WindowNotFound(window)),
        }

        match self.request(Request::CloseWindow(window)) {
            Response::WindowClosed(id) if id == window => Ok(()),
            Response::WindowNotFound(id) if id == window => Err(WindowNotFound(id)),
            _ => panic!("view process did not respond correctly"),
        }
    }

    /// Read the system text anti-aliasing config.
    pub fn text_aa(&self) -> TextAntiAliasing {
        match self.request(Request::TextAa) {
            Response::TextAa(aa) => aa,
            _ => panic!("view process did not respond correctly"),
        }
    }

    /// Gracefully shutdown the view process, returns when the process is closed.
    pub fn shutdown(mut self) {
        self.request_sender.send(&Request::Shutdown).unwrap();
        self.process.wait().unwrap();
    }
}
impl Drop for App {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

struct Window {
    id: WinId,
    title: String,
    pos: (i32, i32),
    size: (u32, u32),
    visible: bool,
    frame: (PipelineId, LayoutSize, (Vec<u8>, BuiltDisplayListDescriptor)),
}

struct Device {
    id: DevId,
}

/// Error when a window ID is not opened in an [`App`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct WindowNotFound(pub WinId);
impl fmt::Display for WindowNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window `{}` not found", self.0)
    }
}
impl std::error::Error for WindowNotFound {}
