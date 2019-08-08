use crate::window::Window;
use glutin::*;
use std::collections::BTreeMap;
use std::thread;
use std::time::{Duration, Instant};

pub struct App {
    events_loop: EventsLoop,
    windows: BTreeMap<WindowId, Window>,
}

impl App {
    pub fn new() -> App {
        App {
            events_loop: EventsLoop::new(),
            windows: BTreeMap::new(),
        }
    }

    pub fn window(
        mut self,
        title: impl ToString,
        background_color: webrender::api::ColorF,
    ) -> Self {
        let win = Window::new(title.to_string(), background_color, &self.events_loop);
        self.windows.insert(win.id(), win);
        self
    }

    pub fn run(mut self) {
        while !self.windows.is_empty() {
            let time_start = Instant::now();

            let windows = &mut self.windows;

            self.events_loop.poll_events(|event| match event {
                Event::WindowEvent { window_id, event } => {
                    if let Some(win) = windows.get_mut(&window_id) {
                        win.event(event);

                        if win.exit {
                            let win = windows.remove(&window_id).unwrap();
                            win.deinit();
                        } else {
                            win.render();
                            win.render(); // TODO
                        }
                    }
                }
                _ => {}
            });

            let diff = time_start.elapsed();
            const FRAME_TIME: Duration = Duration::from_millis(16);
            if diff < FRAME_TIME {
                thread::sleep(FRAME_TIME - diff);
            }
        }
    }
}
