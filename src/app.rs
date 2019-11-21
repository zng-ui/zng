use crate::core::FocusKey;
use crate::core::{NewWindow, NextUpdate, Ui, WebRenderEvent, Window};
use rayon::ThreadPoolBuilder;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use fnv::FnvHashMap;

#[cfg(feature = "app_profiler")]
use crate::core::profiler::{register_thread_with_profiler, ProfileScope, write_profile};
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
    #[cfg(feature = "app_profiler")]
    register_thread_with_profiler();

    #[cfg(feature = "app_profiler")]
    let mut app_scope = ProfileScope::new("app".to_owned());
    let event_loop = EventLoop::with_user_event();
    let mut windows = FnvHashMap::default();
    let ui_threads = Arc::new(
        ThreadPoolBuilder::new()
            .thread_name(|idx| format!("UI#{}", idx))
            .start_handler(move |idx| {
                #[cfg(feature = "app_profiler")]
                register_thread_with_profiler();
            })
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
    let focused = Focused::default();

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
                #[cfg(feature = "app_profiler")]
                {
                    let output = "./profile.json";
                    drop(std::mem::replace(&mut app_scope, ProfileScope::new(String::new())));
                    write_profile(output);
                }
            }
            _ => {}
        }

        if !in_event_sequence {
            while has_update {
                has_update = false;

                // windows creation/destruction updates
                let mut to_remove = vec![];
                let mut new_windows = vec![];
                for win in windows.values_mut() {
                    new_windows.append(&mut win.new_window_requests());

                    if win.close {
                        to_remove.push(win.id());
                    }
                }
                for new_win in new_windows {
                    let win = Window::new(new_win, &event_loop, event_loop_proxy.clone(), Arc::clone(&ui_threads));
                    windows.insert(win.id(), win);
                }
                for window_id in to_remove {
                    let win = windows.remove(&window_id).unwrap();
                    win.deinit();
                }

                // if we have no window left.
                if windows.is_empty() {
                    // exit application.
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                // value updates & window content updates

                // value updates affect all windows, collect all changed vars
                let mut value_changes = vec![];
                for win in windows.values_mut() {
                    value_changes.append(&mut win.value_changes());
                }
                // commit changes and set touched = true
                for var in value_changes.iter_mut() {
                    var.commit();
                }

                // do all window content updates.
                for win in windows.values_mut() {
                    // if a window has a frame ready to show
                    if win.redraw {
                        // show new frame
                        win.redraw_and_swap_buffers();
                    }

                    // do content update, it can cause another update
                    has_update |= win.update(!value_changes.is_empty(), Rc::clone(&focused));
                }

                // value updates done, reset touched flag.
                for mut var in value_changes {
                    var.reset_touched();
                }
            }
        }
    })
}

pub(crate) type Focused = Rc<Cell<Option<FocusKey>>>;
