use crate::ui::{Ui, InitContext};
use crate::window::{WebRenderEvent, Window};

use std::collections::HashMap;

use glutin::event::Event;
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowId;
use webrender::api::LayoutSize;

pub struct App {
    event_loop: EventLoop<WebRenderEvent>,
    windows: HashMap<WindowId, Window>,
}

impl App {
    pub fn new() -> App {
        App {
            event_loop: EventLoop::with_user_event(),
            windows: HashMap::new(),
        }
    }

    pub fn window<Tcontent: Ui + 'static>(
        mut self,
        title: impl ToString,
        background_color: webrender::api::ColorF,
        content: impl Fn (&InitContext) -> Tcontent,
    ) -> Self {
        let win = Window::new(
            title.to_string(),
            background_color,
            LayoutSize::new(800., 600.),
            |c| content(c).into_box(),
            &self.event_loop,
            self.event_loop.create_proxy(),
        );
        self.windows.insert(win.id(), win);
        self
    }

    pub fn run(self) -> ! {
        let App {
            event_loop,
            mut windows,
        } = self;

        // will use to create window inside run callback.
        let _event_loop_proxy = event_loop.create_proxy();

        let mut in_event_sequence = false;
        let mut has_update = true;

        event_loop.run(move |event, _event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(_) => {
                    in_event_sequence = true;
                }
                Event::EventsCleared => {
                    in_event_sequence = false;
                }

                Event::WindowEvent { window_id, event } => {
                    if let Some(win) = windows.get_mut(&window_id) {
                        has_update |= win.event(event);
                    }
                }
                Event::UserEvent(WebRenderEvent::NewFrameReady(window_id)) => {
                    if let Some(win) = windows.get_mut(&window_id) {
                        // this can cause a RedrawRequested after EventsCleared but before NewEvents
                        win.request_redraw();
                    }
                }

                Event::LoopDestroyed => {
                    for (_, win) in windows.drain() {
                        win.deinit();
                    }
                }
                _ => {}
            }

            if !in_event_sequence && has_update {
                has_update = false;

                let mut to_remove = vec![];

                for win in windows.values_mut() {
                    if win.close {
                        to_remove.push(win.id());
                        continue;
                    }
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

                for window_id in to_remove {
                    let win = windows.remove(&window_id).unwrap();
                    win.deinit();
                }

                if windows.is_empty() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
        })
    }
}
