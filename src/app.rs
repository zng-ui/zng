use crate::core::{NewWindow, NextUpdate, Ui, WebRenderEvent, Window};
use rayon::ThreadPoolBuilder;
use std::sync::Arc;

use fnv::FnvHashMap;

use glutin::event::Event;
use glutin::event_loop::{ControlFlow, EventLoop};
use webrender::api::{ColorF, LayoutSize};

/// Runs the application with arguments for creating the first window.
///
/// This function does not return, the process exits when the last window is closed.
///
/// # Arguments
/// `clear_color`: First window background color.
/// `inner_size`: First window size.
/// `content`: First window content factory.
pub fn run<C: Ui + 'static>(
    clear_color: ColorF,
    inner_size: LayoutSize,
    content: impl Fn(&mut NextUpdate) -> C + 'static,
) -> ! {
    let event_loop = EventLoop::with_user_event();
    let mut windows = FnvHashMap::default();
    let ui_threads = Arc::new(
        ThreadPoolBuilder::new()
            .thread_name(|idx| format!("UI#{}", idx))
            .build()
            .unwrap(),
    );

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
                        let win = Window::new(new_win, &event_loop, event_loop_proxy.clone(), Arc::clone(&ui_threads));

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
                var.reset_touched();
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
