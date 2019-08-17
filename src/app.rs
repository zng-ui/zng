use crate::ui::Ui;
use crate::window::{WebRenderEvent, Window};

use std::collections::HashMap;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowId;
use webrender::api::LayoutSize;

pub struct App {
    events_loop: EventLoop<WebRenderEvent>,
    windows: HashMap<WindowId, Window>,
}

impl App {
    pub fn new() -> App {
        App {
            events_loop: EventLoop::with_user_event(),
            windows: HashMap::new(),
        }
    }

    pub fn window(
        mut self,
        title: impl ToString,
        background_color: webrender::api::ColorF,
        content: impl Ui + 'static,
    ) -> Self {
        let win = Window::new(
            title.to_string(),
            background_color,
            LayoutSize::new(800., 600.),
            content.into_box(),
            &self.events_loop,
        );
        self.windows.insert(win.id(), win);
        self
    }

    pub fn run(self) -> ! {
        let App {
            events_loop,
            mut windows,
        } = self;

        events_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { window_id, event } => {
                    if let Some(win) = windows.get_mut(&window_id) {
                        win.event(event);
                    }
                }
                Event::UserEvent(WebRenderEvent::NewFrameReady(window_id)) => {
                    if let Some(win) = windows.get_mut(&window_id) {
                        win.event(WindowEvent::RedrawRequested);
                    }
                }
                Event::EventsCleared => {
                    let to_remove: Vec<_> = windows.values().filter(|w| w.close).map(|w| w.id()).collect();
                    for window_id in to_remove {
                        let win = windows.remove(&window_id).unwrap();
                        win.deinit();
                    }

                    if windows.is_empty() {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    for win in windows.values_mut() {
                        if win.update_layout {
                            win.layout();
                        }
                        if win.render_frame {
                            win.send_render_frame();
                        }
                        if win.redraw {
                            win.redraw_and_swap_buffers();
                        }
                    }
                }
                Event::LoopDestroyed => {
                    for (_, win) in windows.drain() {
                        win.deinit();
                    }
                }

                _ => {}
            }
        })
    }
}
