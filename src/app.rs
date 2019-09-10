use crate::ui::{NewWindow, NextUpdate, Ui};
use crate::window::{WebRenderEvent, Window};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::Arc;

use std::collections::HashMap;

use glutin::event::Event;
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowId;
use webrender::api::{ColorF, LayoutSize};

pub struct App {
    event_loop: EventLoop<WebRenderEvent>,
    windows: HashMap<WindowId, Window>,
    ui_threads: Arc<ThreadPool>,
}

impl App {
    pub fn new() -> App {
        App {
            event_loop: EventLoop::with_user_event(),
            windows: HashMap::new(),
            ui_threads: Arc::new(
                ThreadPoolBuilder::new()
                    .thread_name(|idx| format!("UI#{}", idx))
                    .build()
                    .unwrap(),
            ),
        }
    }

    pub fn run<TContent: Ui + 'static>(
        self,
        clear_color: ColorF,
        inner_size: LayoutSize,
        content: impl Fn(&mut NextUpdate) -> TContent + 'static,
    ) -> ! {
        let App {
            event_loop,
            mut windows,
            ui_threads,
        } = self;

        let main_window = NewWindow {
            content: Box::new(move |c| content(c).into_box()),
            clear_color,
            inner_size,
        };

        let event_loop_proxy = event_loop.create_proxy();
        let win = Window::new(
            main_window,
            &event_loop,
            event_loop_proxy.clone(),
            Arc::clone(&ui_threads),
        );

        windows.insert(win.id(), win);

        let mut in_event_sequence = false;
        let mut has_update = true;

        event_loop.run(move |event, event_loop, control_flow| {
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
                        let wins = win.new_window_requests();
                        for new_win in wins {
                            let win =
                                Window::new(new_win, &event_loop, event_loop_proxy.clone(), Arc::clone(&ui_threads));

                            windows.insert(win.id(), win);
                        }
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
                let mut value_changes = vec![];

                for win in windows.values_mut() {
                    value_changes.append(&mut win.value_changes());
                }

                for var in value_changes.iter_mut() {
                    var.commit();
                }

                for win in windows.values_mut() {
                    if win.close {
                        to_remove.push(win.id());
                        continue;
                    }

                    if win.redraw {
                        win.redraw_and_swap_buffers();
                    }

                    win.update(!value_changes.is_empty());
                }

                for mut var in value_changes {
                    var.reset_changed();
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
