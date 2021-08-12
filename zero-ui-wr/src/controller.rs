use std::env;
use std::process::{Child, Command, Stdio};

use ipmpsc::{Receiver, Sender, SharedRingBuffer};

use crate::{message::*, CHANNEL_VAR};

pub struct App {
    process: Child,
    sender: Sender,
    receiver: Receiver,
    windows: Vec<Window>,
    devices: Vec<Device>,
}

impl App {
    pub fn new(device_events: bool) -> Self {
        let channel_dir = loop {
            let temp_dir = env::temp_dir().join(uuid::Uuid::new_v4().to_simple().to_string());
            match std::fs::create_dir(&temp_dir) {
                Ok(_) => break temp_dir,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("failed to create channel directory: {}", e),
            }
        };

        let receiver = Receiver::new(
            SharedRingBuffer::create(channel_dir.join("response").display().to_string().as_str(), MAX_RESPONSE_SIZE)
                .expect("response channel creation failed"),
        );
        let sender = Sender::new(
            SharedRingBuffer::create(channel_dir.join("request").display().to_string().as_str(), MAX_REQUEST_SIZE)
                .expect("request channel creation failed"),
        );

        // create process and spawn it
        let process = Command::new(std::env::current_exe().unwrap())
            .env(CHANNEL_VAR, channel_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("renderer view process failed to spawn");

        let mut app = App {
            process,
            sender,
            receiver,
            windows: vec![],
            devices: vec![],
        };

        match app.request(Request::Start(StartRequest { device_events })) {
            Response::Started => app,
            _ => panic!("render view process did not start correctly"),
        }
    }

    fn request(&self, request: Request) -> Response {
        self.sender.send(&request).unwrap();
        self.receiver.recv().unwrap()
    }
}

struct Window {}

struct Device {}
