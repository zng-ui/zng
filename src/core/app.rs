use crate::core::{context::*, events::*, focus::FocusManager, font::FontManager, types::*, window::WindowManager};

use glutin::event::Event as GEvent;
use glutin::event_loop::{ControlFlow, EventLoop};
use std::any::{type_name, TypeId};

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
    fn update_display(&mut self, _update: UpdateDisplayRequest) {}

    /// Called when the OS sends a request for re-drawing the last frame.
    #[inline]
    fn on_redraw_requested(&mut self, _window_id: WindowId, _ctx: &mut AppContext) {}
}

impl AppExtension for () {}

impl<A: AppExtension, B: AppExtension> AppExtension for (A, B) {
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.0.init(ctx);
        self.1.init(ctx);
    }

    fn is_or_contain(&self, app_extension_id: TypeId) -> bool {
        self.0.is_or_contain(app_extension_id) || self.1.is_or_contain(app_extension_id) || self.id() == app_extension_id
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: &DeviceEvent, ctx: &mut AppContext) {
        self.0.on_device_event(device_id, event, ctx);
        self.1.on_device_event(device_id, event, ctx);
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        self.0.on_window_event(window_id, event, ctx);
        self.1.on_window_event(window_id, event, ctx);
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_new_frame_ready(window_id, ctx);
        self.1.on_new_frame_ready(window_id, ctx);
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.0.update(update, ctx);
        self.1.update(update, ctx);
    }

    fn update_display(&mut self, update: UpdateDisplayRequest) {
        self.0.update_display(update);
        self.1.update_display(update);
    }

    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        self.0.on_redraw_requested(window_id, ctx);
        self.1.on_redraw_requested(window_id, ctx);
    }
}

/// Defines and runs an application.
pub struct App;

impl App {
    /// Application without any extension.
    pub fn empty() -> AppExtended<()> {
        AppExtended { extensions: () }
    }

    /// Application with default extensions.
    pub fn default() -> AppExtended<impl AppExtension> {
        App::empty()
            .extend(MouseEvents::default())
            .extend(KeyboardEvents::default())
            .extend(WindowManager::default())
            .extend(FontManager::default())
            .extend(FocusManager::default())
    }
}

/// Application with extensions.
pub struct AppExtended<E: AppExtension> {
    extensions: E,
}

impl<E: AppExtension> AppExtended<E> {
    /// Gets if the application is already extended with the extension type.
    pub fn extended_with<F: AppExtension>(&self) -> bool {
        self.extensions.is_or_contain(TypeId::of::<F>())
    }

    /// Includes an application extension.
    ///
    /// # Panics
    /// * `"app already extended with `{}`"` when the app is already [extended_with](AppExtended::extended_with) the
    /// extension type.
    pub fn extend<F: AppExtension>(self, extension: F) -> AppExtended<impl AppExtension> {
        if self.extended_with::<F>() {
            panic!("app already extended with `{}`", type_name::<F>())
        }
        AppExtended {
            extensions: (self.extensions, extension),
        }
    }

    /// Runs the application.
    pub fn run(self, start: impl FnOnce(&mut AppContext)) -> ! {
        let event_loop = EventLoop::with_user_event();

        let mut extensions = self.extensions;

        let mut owned_ctx = OwnedAppContext::instance(event_loop.create_proxy());

        extensions.init(&mut owned_ctx.borrow_init());

        let mut in_sequence = false;
        let mut sequence_update = UpdateDisplayRequest::None;

        start(&mut owned_ctx.borrow(&event_loop));

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            let mut event_update = UpdateRequest::default();
            match event {
                GEvent::NewEvents(_) => {
                    in_sequence = true;
                }

                GEvent::WindowEvent { window_id, event } => {
                    extensions.on_window_event(window_id, &event, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(AppEvent::NewFrameReady(window_id)) => {
                    extensions.on_new_frame_ready(window_id, &mut owned_ctx.borrow(event_loop));
                }
                GEvent::UserEvent(AppEvent::Update) => {
                    event_update = owned_ctx.take_request();
                }
                GEvent::DeviceEvent { device_id, event } => {
                    extensions.on_device_event(device_id, &event, &mut owned_ctx.borrow(event_loop));
                }

                GEvent::MainEventsCleared => {
                    in_sequence = false;
                }

                GEvent::RedrawRequested(window_id) => extensions.on_redraw_requested(window_id, &mut owned_ctx.borrow(event_loop)),

                _ => {}
            }

            loop {
                let (mut update, display) = owned_ctx.apply_updates();
                update |= event_update;
                sequence_update |= display;

                if update.update || update.update_hp {
                    extensions.update(update, &mut owned_ctx.borrow(event_loop));
                } else {
                    break;
                }
            }

            if !in_sequence && sequence_update.is_some() {
                extensions.update_display(sequence_update);
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
