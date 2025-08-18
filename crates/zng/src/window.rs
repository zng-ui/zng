#![cfg(feature = "window")]

//! Window service, widget, events, commands and other types.
//!
//! The [`Window!`](struct@Window) widget instantiates a window root, the windows service uses the window root as the
//! root widget of new window.
//!
//! The example below declares a window that toggles if it can close.
//!
//! ```
//! # fn main() {}
//! use zng::prelude::*;
//!
//! fn app() {
//!     APP.defaults().run_window(async { window() });
//! }
//!
//! fn window() -> window::WindowRoot {
//!     let allow_close = var(true);
//!     Window! {
//!         on_close_requested = hn!(allow_close, |args: &window::WindowCloseRequestedArgs| {
//!             if !allow_close.get() {
//!                 args.propagation().stop();
//!             }
//!         });
//!
//!         title = "Can I Close?";
//!         child_align = layout::Align::CENTER;
//!         child = Toggle! {
//!             child = Text!(allow_close.map(|a| formatx!("allow close = {a:?}")));
//!             checked = allow_close;
//!         };
//!     }
//! }
//! ```
//!
//! The [`WINDOWS`] service can be used to open, manage and close windows. The example below
//! opens a parent and child window.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn app() {
//!     APP.defaults().run(async {
//!         let r = WINDOWS.open(async { main_window() });
//!         println!("opened {}", r.wait_rsp().await);
//!     });
//! }
//!
//! fn main_window() -> window::WindowRoot {
//!     Window! {
//!         title = "Main Window";
//!         child_align = layout::Align::CENTER;
//!         child = {
//!             let enabled = var(true);
//!             Button! {
//!                 child = Text!("Open/Close Child");
//!                 on_click = async_hn!(enabled, |_| {
//!                     enabled.set(false);
//!
//!                     if WINDOWS.is_open("child-id") {
//!                         if let Ok(r) = WINDOWS.close("child-id") {
//!                             r.wait_done().await;
//!                         }
//!                     } else {
//!                         let parent = WINDOW.id();
//!                         WINDOWS.open_id("child-id", async move { child_window(parent) }).wait_done().await;
//!                     }
//!
//!                     enabled.set(true);
//!                 });
//!                 widget::enabled;
//!             }
//!         };
//!     }
//! }
//!
//! fn child_window(parent: WindowId) -> window::WindowRoot {
//!     Window! {
//!         parent;
//!         title = "Child Window";
//!         size = (200, 100);
//!         child = Button! {
//!             child = Text!("Close");
//!             on_click = hn!(|_| {
//!                 let _ = WINDOW.close();
//!             });
//!         };
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! # Full API
//!
//! See [`zng_ext_window`], [`zng_app::window`] and [`zng_wgt_window`] for the full window API.

pub use zng_app::window::{MonitorId, WINDOW, WindowId, WindowMode};

pub use zng_ext_window::{
    AppRunWindowExt, AutoSize, CloseWindowResult, FRAME_IMAGE_READY_EVENT, FocusIndicator, FrameCaptureMode, FrameImageReadyArgs,
    HeadlessAppWindowExt, HeadlessMonitor, IME_EVENT, ImeArgs, MONITORS, MONITORS_CHANGED_EVENT, MonitorInfo, MonitorQuery,
    MonitorsChangedArgs, ParallelWin, RenderMode, StartPosition, VideoMode, WINDOW_CHANGED_EVENT, WINDOW_CLOSE_EVENT,
    WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_Ext, WINDOW_LOAD_EVENT, WINDOW_OPEN_EVENT, WINDOWS, WidgetInfoBuilderImeArea, WidgetInfoImeArea,
    WindowButton, WindowChangedArgs, WindowCloseArgs, WindowCloseRequestedArgs, WindowIcon, WindowLoadingHandle, WindowOpenArgs,
    WindowRoot, WindowRootExtenderArgs, WindowState, WindowStateAllowed, WindowVars,
};

/// Window commands.
pub mod cmd {
    pub use zng_ext_window::cmd::*;

    #[cfg(feature = "inspector")]
    pub use zng_wgt_inspector::INSPECT_CMD;
}

pub use zng_wgt_window::{BlockWindowLoad, Window};

pub use zng_wgt_window::events::{
    on_frame_image_ready, on_ime, on_pre_frame_image_ready, on_pre_ime, on_pre_window_changed, on_pre_window_close_requested,
    on_pre_window_exited_fullscreen, on_pre_window_fullscreen, on_pre_window_load, on_pre_window_maximized, on_pre_window_minimized,
    on_pre_window_moved, on_pre_window_open, on_pre_window_resized, on_pre_window_restored, on_pre_window_state_changed,
    on_pre_window_unmaximized, on_pre_window_unminimized, on_window_changed, on_window_close_requested, on_window_exited_fullscreen,
    on_window_fullscreen, on_window_load, on_window_maximized, on_window_minimized, on_window_moved, on_window_open, on_window_resized,
    on_window_restored, on_window_state_changed, on_window_unmaximized, on_window_unminimized,
};

/// Debug inspection helpers.
///
/// The properties in this module can be set on a window or widget to visualize layout and render internals.
///
/// The [`INSPECTOR`] service can be used to configure the inspector window, add custom watchers.
/// Note that you can use the [`cmd::INSPECT_CMD`] command to open the Inspector.
///
/// # Examples
///
/// The example below registers two custom live updating watchers.
///
/// ```
/// # use zng::prelude::*;
/// #
/// # let _scope = APP.minimal();
/// window::inspector::INSPECTOR.register_watcher(|wgt, builder| {
///     // watch custom info metadata
///     use zng::markdown::WidgetInfoExt as _;
///     let watcher = wgt.info().map(|i| formatx!("{:?}", i.anchor()));
///     builder.insert("markdown.anchor", watcher);
///
///     // watch value that can change every layout/render without info rebuild
///     let watcher = wgt.render_watcher(|i| formatx!("{:?}", i.bounds_info().inline().is_some()));
///     builder.insert("is_inlined", watcher);
/// });
/// ```
///
/// The closure is called on widget selection change (in the inspector screen), the values are presented in the
/// `/* INFO */` section of the properties panel.
///
/// # Full API
///
/// See [`zng_wgt_inspector`] for the full API.
///
/// [`cmd::INSPECT_CMD`]: crate::window::cmd::INSPECT_CMD
/// [`INSPECTOR`]: crate::window::inspector::INSPECTOR
#[cfg(feature = "inspector")]
pub mod inspector {
    pub use zng_wgt_inspector::debug::{InspectMode, show_bounds, show_center_points, show_directional_query, show_hit_test, show_rows};

    pub use zng_wgt_inspector::{INSPECTOR, InspectedInfo, InspectedTree, InspectedWidget, InspectorWatcherBuilder};
}

/// Default handler registered in mobile platforms.
///
/// This is registered on app init for platforms that only support one window, it intercepts headed window open requests after the
/// first and opens them as a nested modal layer on the main window.
///
/// See [`WINDOWS::register_open_nested_handler`] for more details.
pub fn default_mobile_nested_open_handler(args: &mut zng_ext_window::OpenNestedHandlerArgs) {
    use crate::prelude::*;

    if !matches!(args.ctx().mode(), WindowMode::Headed) {
        return;
    }

    let open: Vec<_> = WINDOWS
        .widget_trees()
        .into_iter()
        .filter(|w| WINDOWS.mode(w.window_id()) == Ok(window::WindowMode::Headed) && WINDOWS.nest_parent(w.window_id()).is_none())
        .take(2)
        .collect();

    if open.len() == 1 {
        let id = args.ctx().id();
        let vars = args.vars();
        #[cfg(feature = "image")]
        let icon = vars.icon();
        let title = vars.title();
        let node = task::parking_lot::Mutex::new(Some(args.nest()));

        let host_win_id = open[0].window_id();
        let host_wgt_id = WidgetId::new_unique();
        layer::LAYERS_INSERT_CMD.scoped(host_win_id).notify_param((
            layer::LayerIndex::TOP_MOST,
            wgt_fn!(|_: ()| {
                let frame = Container! {
                    layout::margin = 10;
                    layout::align = Align::CENTER;
                    widget::modal = true;
                    #[cfg(feature = "color_filter")]
                    color::filter::drop_shadow = {
                        offset: 4,
                        blur_radius: 6,
                        color: colors::BLACK.with_alpha(50.pct()),
                    };
                    widget::background_color = light_dark(rgb(0.95, 0.95, 0.95), rgb(0.05, 0.05, 0.05));
                    widget::corner_radius = 4;
                    layout::padding = 5;
                    child_top = {
                        node: Container! {
                            #[cfg(feature = "image")]
                            child_start =
                                Image! {
                                    layout::size = 24;
                                    source = icon.map(|i| match i {
                                        WindowIcon::Image(s) => s.clone(),
                                        WindowIcon::Default => ImageSource::flood(layout::PxSize::zero(), rgba(0, 0, 0, 0), None),
                                    });
                                },
                                4,
                            ;
                            child = Text! {
                                txt = title.clone();
                                txt_align = Align::CENTER;
                                font_weight = FontWeight::BOLD;
                            };
                            #[cfg(feature = "button")]
                            child_end =
                                Button! {
                                    style_fn = zng::button::LightStyle!();
                                    child = ICONS.get_or("close", || Text!("x"));
                                    on_click = hn!(|args: &gesture::ClickArgs| {
                                        args.propagation().stop();
                                        let _ = WINDOWS.close(id);
                                    });
                                },
                                4,
                            ;
                        },
                        spacing: 5,
                    };
                    child = node.lock().take().into_node().into_widget();
                };
                Container! {
                    id = host_wgt_id;
                    child = frame;
                    widget::background_color = colors::BLACK.with_alpha(20.pct());
                    layout::padding = WINDOWS.vars(host_win_id).unwrap().safe_padding().map_into();
                }
            }),
        ));

        window::WINDOW_CLOSE_EVENT
            .on_pre_event(app_hn!(|args: &window::WindowCloseArgs, ev: &dyn zng::handler::AppWeakHandle| {
                if args.windows.contains(&id) {
                    ev.unsubscribe();
                    layer::LAYERS_REMOVE_CMD.scoped(host_win_id).notify_param(host_wgt_id);
                }
            }))
            .perm();
    }
}
