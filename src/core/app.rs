use crate::core::event::EventEmitter;
use crate::core::event::EventListener;
use crate::core::{
    context::*, event::CancelableEventArgs, focus::FocusManager, font::FontManager, gesture::GestureEvents, keyboard::KeyboardEvents,
    mouse::MouseEvents, types::*, window::WindowManager,
};
use glutin::event::Event as GEvent;
use glutin::event_loop::{ControlFlow, EventLoop};
use std::any::{type_name, TypeId};
use std::mem;

/// An [App] extension.
pub trait AppExtension: 'static {
    /// Type id of this extension.
    #[inline]
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// If this extension is the `app_extension_id` or dispatches to it.
    #[inline]
    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.id() == app_extension_id
    }

    /// Initializes this extension.
    #[inline]
    fn init(&mut self, _ctx: &mut AppInitContext) {}

    /// Called when the OS sends an event to a device.
    #[inline]
    fn on_device_event(&mut self, _device_id: DeviceId, _event: &DeviceEvent, _ctx: &mut AppContext) {}

    /// Called when the OS sends an event to a window.
    #[inline]
    fn on_window_event(&mut self, _window_id: WindowId, _event: &WindowEvent, _ctx: &mut AppContext) {}

    /// Called when a new frame is ready to be presented.
    #[inline]
    fn on_new_frame_ready(&mut self, _window_id: WindowId, _ctx: &mut AppContext) {}

    /// Called every update after the Ui update.
    #[inline]
    fn update(&mut self, _update: UpdateRequest, _ctx: &mut AppContext) {}

    /// Called after every sequence of updates if display update was requested.
    #[inline]
    fn update_display(&mut self, _update: UpdateDisplayRequest, _ctx: &mut AppContext) {}

    /// Called when the OS sends a request for re-drawing the last frame.
    #[inline]
    fn on_redraw_requested(&mut self, _window_id: WindowId, _ctx: &mut AppContext) {}

    /// Called when a shutdown was requested.
    #[inline]
    fn on_shutdown_requested(&mut self, _args: &ShutdownRequestedArgs, _ctx: &mut AppContext) {}

    /// Called when the application is shutting down.
    ///
    /// Update requests generated during this call are ignored.
    #[inline]
    fn deinit(&mut self, _ctx: &mut AppContext) {}
}

cancelable_event_args! {
    /// Arguments for `on_shutdown_requested`.
    pub struct ShutdownRequestedArgs {
        ..
        /// Always true.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}

impl AppExtension for () {}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    #[inline]
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.0.init(ctx);
        self.1.init(ctx);
    }

    #[inline]
    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.0.is_or_contain(app_extension_id) || self.1.is_or_contain(app_extension_id) || self.id() == app_extension_id
    }

    #[inline]
    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.0.on_device_event(device_id, event, ctx);
        self.1.on_device_event(device_id, event, ctx);
    }

    #[inline]
    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.0.on_window_event(window_id, event, ctx);
        self.1.on_window_event(window_id, event, ctx);
    }

    #[inline]
    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_new_frame_ready(window_id, ctx);
        self.1.on_new_frame_ready(window_id, ctx);
    }

    #[inline]
    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.0.update(update, ctx);
        self.1.update(update, ctx);
    }

    #[inline]
    fn update_display(&mut self, update: UpdateDisplayRequest, ctx: &mut AppContext) {
        self.0.update_display(update, ctx);
        self.1.update_display(update, ctx);
    }

    #[inline]
    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_redraw_requested(window_id, ctx);
        self.1.on_redraw_requested(window_id, ctx);
    }

    #[inline]
    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        self.0.on_shutdown_requested(args, ctx);
        self.1.on_shutdown_requested(args, ctx);
    }

    #[inline]
    fn deinit(&mut self, ctx: &mut AppContext) {
        self.0.deinit(ctx);
        self.1.deinit(ctx);
    }
}

/// Defines and runs an application.
pub struct App;

impl App {
    /// Application without any extension.
    #[inline]
    pub fn empty() -> AppExtended<()> {
        AppExtended { extensions: () }
    }

    /// Application with default extensions.
    ///
    /// # Extensions
    ///
    /// Extensions included.
    ///
    /// * [MouseEvents]
    /// * [KeyboardEvents]
    /// * [GestureEvents]
    /// * [WindowManager]
    /// * [FontManager]
    /// * [FocusManager]
    #[inline]
    pub fn default() -> AppExtended<impl AppExtension> {
        App::empty()
            .extend(MouseEvents::default())
            .extend(KeyboardEvents::default())
            .extend(GestureEvents::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
    }
}

/// Application with extensions.
pub struct AppExtended<E: AppExtension> {
    extensions: E,
}

pub struct ShutDownCancelled;

/// Service for managing the application process.
pub struct AppProcess {
    shutdown_requests: Vec<EventEmitter<ShutDownCancelled>>,
    update_notifier: UpdateNotifier,
}
impl AppService for AppProcess {}
impl AppProcess {
    pub fn new(update_notifier: UpdateNotifier) -> Self {
        AppProcess {
            shutdown_requests: Vec::new(),
            update_notifier,
        }
    }

    /// Register a request for process shutdown in the next update.
    ///
    /// Returns an event listener that is updated once with the unit value [`ShutDownCancelled`](ShutDownCancelled)
    /// if the shutdown operation is cancelled.
    pub fn shutdown(&mut self) -> EventListener<ShutDownCancelled> {
        let emitter = EventEmitter::new(false);
        self.shutdown_requests.push(emitter.clone());
        self.update_notifier.push_update();
        emitter.into_listener()
    }

    fn take_requests(&mut self) -> Vec<EventEmitter<ShutDownCancelled>> {
        mem::take(&mut self.shutdown_requests)
    }
}
///Returns if should shutdown
fn shutdown(shutdown_requests: Vec<EventEmitter<ShutDownCancelled>>, ctx: &mut AppContext, ext: &mut impl AppExtension) -> bool {
    if shutdown_requests.is_empty() {
        return false;
    }
    let args = ShutdownRequestedArgs::now();
    ext.on_shutdown_requested(&args, ctx);
    if args.cancel_requested() {
        for c in shutdown_requests {
            ctx.updates.push_notify(c, ShutDownCancelled)
        }
    }
    !args.cancel_requested()
}

impl<E: AppExtension> AppExtended<E> {
    /// Gets if the application is already extended with the extension type.
    #[inline]
    pub fn extended_with<F: AppExtension>(&self) -> bool {
        self.extensions.is_or_contain(TypeId::of::<F>())
    }

    /// Includes an application extension.
    ///
    /// # Panics
    /// * `"app already extended with `{}`"` when the app is already [`extended_with`](AppExtended::extended_with) the
    /// extension type.
    #[inline]
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<impl AppExtension> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }
        AppExtended {
            extensions: (self.extensions, extension),
        }
    }

    /// Runs the application event loop calling `start` once at the beginning.
    #[inline]
    pub fn run(self, start: impl FnOnce(&mut AppContext)) -> ! {
        #[cfg(feature = "app_profiler")]
        crate::core::profiler::register_thread_with_profiler();

        profile_scope!("app::run");

        let event_loop = EventLoop::with_user_event();

        let mut extensions = self.extensions;

        let mut owned_ctx = OwnedAppContext::instance(event_loop.create_proxy());

        let mut init_ctx = owned_ctx.borrow_init();
        init_ctx.services.register(AppProcess::new(init_ctx.updates.notifier().clone()));
        extensions.init(&mut init_ctx);

        let mut in_sequence = false;
        let mut sequence_update = UpdateDisplayRequest::None;

        start(&mut owned_ctx.borrow(&event_loop));

        event_loop.run(move |event, event_loop, control_flow| {
            profile_scope!("app::event");

            *control_flow = ControlFlow::Wait;

            let mut event_update = UpdateRequest::default();
            match event {
                GEvent::NewEvents(_) => {
                    in_sequence = true;
                }

                GEvent::WindowEvent { window_id, event } => {
                    profile_scope!("app::on_window_event");
                    extensions.on_window_event(window_id, &event, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(AppEvent::NewFrameReady(window_id)) => {
                    profile_scope!("app::on_new_frame_ready");
                    extensions.on_new_frame_ready(window_id, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(AppEvent::Update) => {
                    event_update = owned_ctx.take_request();
                }
                GEvent::DeviceEvent { device_id, event } => {
                    profile_scope!("app::on_device_event");
                    extensions.on_device_event(device_id, &event, &mut owned_ctx.borrow(event_loop));
                }

                GEvent::MainEventsCleared => {
                    in_sequence = false;
                }

                GEvent::RedrawRequested(window_id) => {
                    profile_scope!("app::on_redraw_requested");
                    extensions.on_redraw_requested(window_id, &mut owned_ctx.borrow(event_loop))
                }

                #[cfg(feature = "app_profiler")]
                GEvent::LoopDestroyed => {
                    crate::core::profiler::write_profile("app_profile.json", true);
                }

                _ => {}
            }

            let mut limit = 100_000;
            loop {
                let (mut update, display) = owned_ctx.apply_updates();
                update |= mem::take(&mut event_update);
                sequence_update |= display;

                if update.update || update.update_hp {
                    profile_scope!("app::update");
                    let mut ctx = owned_ctx.borrow(event_loop);
                    let shutdown_requests = ctx.services.req::<AppProcess>().take_requests();
                    if shutdown(shutdown_requests, &mut ctx, &mut extensions) {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    extensions.update(update, &mut ctx);
                } else {
                    break;
                }

                limit -= 1;
                if limit == 0 {
                    panic!("immediate update loop reached limit of `100_000` repeats")
                }
            }

            if !in_sequence && sequence_update.is_some() {
                profile_scope!("app::update_display");
                extensions.update_display(sequence_update, &mut owned_ctx.borrow(event_loop));
                sequence_update = UpdateDisplayRequest::None;
            }
        })
    }
}

#[derive(Debug)]
pub enum AppEvent {
    NewFrameReady(WindowId),
    Update,
}
