use crate::core::focus::*;
use crate::core::gesture::*;
use crate::core::window::{HeadlessScreen, RedrawArgs, StartPosition, Window};
use crate::prelude::new_widget::*;
use crate::properties::events::window::*;

/// A window container.
///
/// The instance type is [`Window`], that can be given to the [`Windows`](crate::core::window::Windows) service
/// to open a system window that is kept in sync with the window properties set in the widget.
///
/// # Example
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(|_| {
///     window! {
///         title = "Window 1";
///         content = text("Window 1");
///     }
/// })
/// ```
/// See [`run_window`](crate::core::window::AppRunWindow::run_window) for more details.
#[widget($crate::widgets::window)]
pub mod window {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::core::command::CommandHandle;
    use crate::core::window::{WindowFocusChangedEvent, WindowId, WindowMode, WindowOpenEvent, WindowsExt};
    use crate::properties::commands::CloseWindowCommand;

    use super::*;

    inherit!(container);

    properties! {
        /// Window title.
        properties::title;

        /// Window icon.
        ///
        /// See [`WindowIcon`] for details.
        properties::icon;

        /// Window chrome, the non-client area of the window.
        ///
        /// See [`WindowChrome`] for details.
        properties::chrome;

        /// Window position when it opens.
        #[allowed_in_when = false]
        start_position(impl IntoValue<StartPosition>) = StartPosition::Default;

        /// Window position (*x*, *y*).
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial position.
        ///
        /// You can also set [`x`](#wp-x) and [`y`](#wp-y) as independent properties.
        properties::position = {
            // use shared var in debug to allow inspecting the value.
            #[cfg(debug_assertions)]
            let r = crate::core::var::var(crate::core::units::Point::new(f32::NAN, f32::NAN));

            #[cfg(not(debug_assertions))]
            let r = (f32::NAN, f32::NAN);

            r
        };

        /// Window position *x*.
        ///
        /// This property value is the same as the [`position.x`](#wp-position) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial position.
        properties::x;

        /// Window position *y*.
        ///
        /// This property value is the same as the [`position.y`](#wp-position) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial position.
        properties::y;

        /// Window size (*width*, *height*).
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial size.
        ///
        /// Does not include the OS window border.
        ///
        /// You can also set the [`width`](#wp-width) and [`height`](#wp-height) as independent properties.
        properties::size = {
            // use shared var in debug to allow inspecting the value.
            #[cfg(debug_assertions)]
            let r = crate::core::var::var(crate::core::units::Size::new(f32::NAN, f32::NAN));

            #[cfg(not(debug_assertions))]
            let r = (f32::NAN, f32::NAN);

            r
        };

        /// Window size *width*.
        ///
        /// This property value is the same as the [`size.width`](#wp-size) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial position.
        properties::width;

        /// Window size *height*.
        ///
        /// This property value is the same as the [`size.height`](#wp-size) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial position.
        properties::height;

        /// Window minimum size.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial value.
        ///
        /// You can also set the [`min_width`](#wp-min_width) and [`min_height`](#wp-min_height) as independent properties.
        properties::min_size;

        /// Window minimum width.
        ///
        /// This property value is the same as the [`min_size.width`](#wp-min_size) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial value.
        properties::min_width;

        /// Window minimum height.
        ///
        /// This property value is the same as the [`min_size.height`](#wp-min_size) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial value.
        properties::min_height;

        /// Window maximum size.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial value.
        ///
        /// You can also set the [`max_width`](#wp-max_width) and [`max_height`](#wp-max_height) as independent properties.
        properties::max_size;

        /// Window maximum width.
        ///
        /// This property value is the same as the [`max_size.width`](#wp-max_size) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial value.
        properties::max_width;

        /// Window maximum height.
        ///
        /// This property value is the same as the [`max_size.height`](#wp-max_size) value.
        ///
        /// If set to a variable it is kept in sync, set to [`f32::NAN`] to not give an initial value.
        properties::max_height;

        /// Window auto size to content.
        ///
        /// If enabled overwrites the other sizes with the content size.
        properties::auto_size;

        /// Window background color.
        background_color = rgb(0.1, 0.1, 0.1);

        /// Unique identifier of the window root widget.
        #[allowed_in_when = false]
        root_id(WidgetId) = WidgetId::new_unique();

        /// Windows are focus scopes by default.
        focus_scope = true;

        /// Windows cycle TAB navigation by default.
        tab_nav = TabNav::Cycle;

        /// Windows cycle arrow navigation by default.
        directional_nav = DirectionalNav::Cycle;

        /// Windows remember the last focused widget and return focus when the window is focused.
        focus_scope_behavior = FocusScopeOnFocus::LastFocused;

        /// Test inspector.
        on_shortcut as on_shortcut_inspect = print_frame_inspector();

        /// If the user can resize the window.
        ///
        /// Note that the window can still change size, this only disables
        /// the OS window frame controls that change size.
        properties::resizable;

        /// If the window is visible.
        ///
        /// When set to `false` the window and its *taskbar* icon are not visible, that is different
        /// from a minimized window where the icon is still visible.
        properties::visible;

        /// Whether the window should always stay on top of other windows.
        ///
        /// Note this only applies to other windows that are not also "always-on-top".
        ///
        /// The default value is `false`.
        properties::always_on_top;

        /// If the window is visible in the task-bar.
        ///
        /// The default value is `true`.
        properties::taskbar_visible;

        /// Extra configuration for the window when run in [headless mode](crate::core::window::WindowMode::is_headless).
        ///
        /// When a window runs in headed mode some values are inferred by window context, such as the scale factor that
        /// is taken from the monitor. In headless mode these values can be configured manually.
        #[allowed_in_when = false]
        headless_screen(impl IntoValue<HeadlessScreen>) = HeadlessScreen::default();

        /// Lock-in kiosk mode.
        ///
        /// In kiosk mode the only window states allowed are full-screen or full-screen exclusive, and
        /// all subsequent windows opened are child of the kiosk window.
        ///
        /// Note that this does not configure the operating system window manager,
        /// you still need to setup a kiosk environment, it does not block `ALT+TAB`. This just stops the
        /// app itself from accidentally exiting kiosk mode.
        #[allowed_in_when = false]
        kiosk(bool) = false;

        /// Event just after the window opens.
        ///
        /// This event notifies once per window, after the window content is inited and the first frame is rendered.
        ///
        /// This property is the [`on_pre_window_open`](fn@on_pre_window_open) so window handlers see it first.
        on_pre_window_open as on_open;

        /// Event just before the window frame is redraw.
        #[allowed_in_when = false]
        on_pre_redraw(impl FnMut(&mut RedrawArgs) + 'static) = |_| {};

        /// Event just after the window frame is redraw.
        #[allowed_in_when = false]
        on_redraw(impl FnMut(&mut RedrawArgs) + 'static) = |_| {};

        /// On window close requested.
        ///
        /// This event notifies every time the user or the app tries to close the window, you can call
        /// [`cancel`](WindowCloseRequestedArgs::cancel) to stop the window from being closed.
        on_window_close_requested as on_close_requested;

        remove {
            // replaced with `root_id` to more clearly indicate that it is not the window ID.
            id;
            // replaced with `visible` because Visibility::Hidden is not a thing for windows.
            visibility
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn new(
        child: impl UiNode,
        root_id: WidgetId,
        start_position: impl Into<StartPosition>,
        kiosk: bool,
        headless_screen: impl Into<HeadlessScreen>,
        on_pre_redraw: impl FnMut(&mut RedrawArgs) + 'static,
        on_redraw: impl FnMut(&mut RedrawArgs) + 'static,
    ) -> Window {
        struct OnCloseWindowNode<C: UiNode> {
            child: C,
            handle: CommandHandle,
            #[cfg(windows)]
            allow_alt_f4: Rc<Cell<bool>>,
        }
        impl<C: UiNode> OnCloseWindowNode<C> {
            // if our window is focused.
            fn window_is_focused(&self, ctx: &mut WidgetContext) -> bool {
                let window_id = ctx.path.window_id();
                ctx.services
                    .focus()
                    .focused()
                    .map(|p| p.window_id() == window_id)
                    .unwrap_or_default()
            }

            // in Windows, when using a real window, block the system's ALT+F4 when that shortcut
            // is not present in the command.
            #[cfg(windows)]
            fn setup_alt_f4_block(&self, ctx: &mut WidgetContext, opened_window: WindowId) {
                use zero_ui_core::{
                    app::raw_events::{RawKeyInputArgs, RawKeyInputEvent},
                    app::DeviceId,
                    keyboard::{Key, KeyState},
                };

                let window_id = ctx.path.window_id();
                if opened_window != window_id {
                    return;
                }

                let sender = ctx.events.sender(RawKeyInputEvent);
                let sender_device_id = DeviceId::new_unique();

                let window = ctx.services.windows().window(window_id).unwrap();
                if let WindowMode::Headed = window.mode() {
                    let allow_alt_f4 = self.allow_alt_f4.clone();
                    window.set_raw_windows_event_handler(move |_, msg, wparam, _| {
                        if msg == winapi::um::winuser::WM_SYSKEYDOWN && wparam as i32 == winapi::um::winuser::VK_F4 && !allow_alt_f4.get() {
                            let _ = sender.send(RawKeyInputArgs::now(
                                window_id,
                                sender_device_id,
                                wparam as u32,
                                KeyState::Pressed,
                                Some(Key::F4),
                            ));
                            return Some(0);
                        }
                        None
                    });
                }
            }
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for OnCloseWindowNode<C> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let enabled = self.window_is_focused(ctx);
                self.handle = CloseWindowCommand.new_handle(ctx, enabled);
                self.allow_alt_f4
                    .set(CloseWindowCommand.shortcut().get(ctx).0.contains(&shortcut![ALT + F4]));
                self.child.init(ctx)
            }
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.child.deinit(ctx);
                #[cfg(windows)]
                self.allow_alt_f4.set(true);
            }
            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some(args) = CloseWindowCommand.update(args) {
                    // command requested, run it only if we are focused.
                    if self.window_is_focused(ctx) {
                        let _ = ctx.services.windows().close(ctx.path.window_id());
                    }
                    self.child.event(ctx, args)
                } else if let Some(args) = WindowFocusChangedEvent.update(args) {
                    // toggle enabled if our window activated/deactivated.
                    if args.window_id == ctx.path.window_id() {
                        self.handle.set_enabled(args.focused);
                    }
                    self.child.event(ctx, args);
                } else if let Some(args) = WindowOpenEvent.update(args) {
                    #[cfg(windows)]
                    self.setup_alt_f4_block(ctx, args.window_id);

                    self.child.event(ctx, args);
                } else {
                    self.child.event(ctx, args)
                }
            }
            fn update(&mut self, ctx: &mut WidgetContext) {
                // update the ALT+F4 block flag in Windows.
                #[cfg(windows)]
                if let Some(s) = CloseWindowCommand.shortcut().get_new(ctx) {
                    self.allow_alt_f4.set(s.0.contains(&shortcut![ALT + F4]));
                }

                self.child.update(ctx);
            }
        }
        let child = OnCloseWindowNode {
            child,
            handle: CommandHandle::dummy(),
            allow_alt_f4: Rc::default(),
        };
        Window::new(
            root_id,
            start_position,
            kiosk,
            headless_screen,
            Box::new(on_pre_redraw),
            Box::new(on_redraw),
            child,
        )
    }

    /// Window stand-alone properties.
    ///
    /// These properties are already included in the [`window!`](mod@crate::widgets::window) definition,
    /// but you can also use then stand-alone. They configure the window from any widget inside the window.
    pub mod properties {
        use crate::core::window::{AutoSize, WindowChrome, WindowIcon, WindowId, WindowVars};
        use crate::prelude::new_property::*;

        fn bind_window_var<T: VarValue + PartialEq>(
            child: impl UiNode,
            user_var: impl IntoVar<T>,
            select: impl Fn(&WindowVars) -> &RcVar<T> + 'static,
        ) -> impl UiNode {
            struct BindWindowVarNode<C, V, S> {
                child: C,
                user_var: V,
                select: S,
                binding: Option<VarBindingHandle>,
            }

            #[impl_ui_node(child)]
            impl<T, C, V, S> UiNode for BindWindowVarNode<C, V, S>
            where
                T: VarValue + PartialEq,
                C: UiNode,
                V: Var<T>,
                S: Fn(&WindowVars) -> &RcVar<T> + 'static,
            {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let window_var = (self.select)(ctx.window_state.req::<WindowVars>());
                    if self.user_var.can_update() {
                        self.binding = Some(self.user_var.bind_bidi(ctx.vars, window_var));
                    }
                    window_var.set_ne(ctx.vars, self.user_var.get_clone(ctx.vars));
                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.binding = None;
                    self.child.deinit(ctx);
                }
            }
            BindWindowVarNode {
                child,
                user_var: user_var.into_var(),
                select,
                binding: None,
            }
        }

        #[property(context)]
        pub fn size2(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
            bind_window_var(child, size, |w| w.size())
        }

        // Properties that have a scalar value type, just compare and set.
        macro_rules! set_properties {
            ($(
                $ident:ident: $Type:ty,
            )+) => {
                $(paste::paste! {
                    #[doc = "Sets the [`" $ident "`](WindowVars::" $ident ") window var."]
                    ///
                    #[doc = "Sets `" $ident "` back if the window var updates with a new value."]
                    #[property(context)]
                    pub fn $ident(child: impl UiNode, $ident: impl IntoVar<$Type>) -> impl UiNode {
                        struct [<Window $ident:camel Node>] <C, V> {
                            child: C,
                            $ident: V,
                        }
                        #[impl_ui_node(child)]
                        impl<C: UiNode, V: Var<$Type>> [<Window $ident:camel Node>] <C, V> {
                            fn set(&mut self, ctx: &mut WidgetContext) {
                                let $ident = self.$ident.get_clone(ctx);
                                ctx.window_state
                                    .get::<WindowVars>()
                                    .expect("no `WindowVars` in `window_state`")
                                    .$ident()
                                    .set_ne(ctx.vars, $ident);
                            }

                            #[UiNode]
                            fn init(&mut self, ctx: &mut WidgetContext) {
                                self.set(ctx);
                                self.child.init(ctx);
                            }

                            #[UiNode]
                            fn update(&mut self, ctx: &mut WidgetContext) {
                                if self.$ident.is_new(ctx) {
                                    self.set(ctx);
                                } else if let Some($ident) = ctx
                                    .window_state
                                    .get::<WindowVars>()
                                    .expect("no `WindowVars` in `window_state`")
                                    .$ident()
                                    .get_new(ctx) {
                                    let _ = self.$ident.set_ne(ctx.vars, $ident.clone());
                                }
                                self.child.update(ctx);
                            }
                        }
                        [<Window $ident:camel Node>] {
                            child,
                            $ident: $ident.into_var()
                        }
                    }
                })+
            }
        }
        set_properties! {
            chrome: WindowChrome,
            icon: WindowIcon,
            title: Text,

            auto_size: AutoSize,

            resizable: bool,
            movable: bool,

            always_on_top: bool,

            visible: bool,
            taskbar_visible: bool,

            parent: Option<WindowId>,
            modal: bool,

            transparent: bool,
        }

        // Properties with types composed of two Length members, only pass on finite members.
        macro_rules! set_or_modify_properties {
            ($(
                $ident:ident ( $member_a:ident, $member_b:ident ) : $Type:ty,
            )+) => {$(paste::paste! {
                #[doc = "Sets the [`" $ident "`](WindowVars::" $ident ") window var."]
                ///
                #[doc = "Sets `" $ident "` back if the window var updates with a new value."]
                #[property(context)]
                pub fn $ident(child: impl UiNode, $ident: impl IntoVar<$Type>) -> impl UiNode {
                    struct [<Window $ident:camel Node>] <C, V> {
                        child: C,
                        $ident: V,
                    }
                    #[impl_ui_node(child)]
                    impl<C: UiNode, V: Var<$Type>> [<Window $ident:camel Node>] <C, V> {
                        fn set(&mut self, ctx: &mut WidgetContext) {
                            let $ident = self.$ident.copy(ctx);
                            let [<$ident var>] = ctx.window_state.get::<WindowVars>().expect("no `WindowVars` in `window_state`").$ident();

                            if $ident.$member_a.is_finite() {
                                if $ident.$member_b.is_finite() {
                                    [<$ident var>].set_ne(ctx.vars, $ident);
                                } else if $ident.$member_a != [<$ident var>].get(ctx).$member_a {
                                    [<$ident var>].modify(ctx.vars, move |v|v.$member_a = $ident.$member_a);
                                }
                            } else if $ident.$member_b.is_finite() && $ident.$member_b != [<$ident var>].get(ctx).$member_b {
                                [<$ident var>].modify(ctx.vars, move |v|v.$member_b = $ident.$member_b);
                            }
                        }

                        #[UiNode]
                        fn init(&mut self, ctx: &mut WidgetContext) {
                            self.set(ctx);
                            self.child.init(ctx);
                        }

                        #[UiNode]
                        fn update(&mut self, ctx: &mut WidgetContext) {
                            if self.$ident.is_new(ctx) {
                                self.set(ctx);
                            } else if let Some($ident) = ctx
                                .window_state
                                .get::<WindowVars>()
                                .expect("no `WindowVars` in `window_state`")
                                .$ident()
                                .get_new(ctx)
                            {
                                let _ = self.$ident.set_ne(ctx.vars, $ident.clone());
                            }
                            self.child.update(ctx);
                        }
                    }
                    [<Window $ident:camel Node>] {
                        child,
                        $ident: $ident.into_var()
                    }
                }
            })+}
        }
        set_or_modify_properties! {
            position(x, y): Point,
            size(width, height): Size,
            min_size(width, height): Size,
            max_size(width, height): Size,
        }

        // Properties that set only a member of a window var.
        macro_rules! modify_properties {
            ($(
                $ident:ident = $var:ident . $member:ident,
            )+) => {$(paste::paste! {
                #[doc = "Sets the `" $member "` member of the [`" $var "`](WindowVars::" $var ") window var."]
                ///
                #[doc = "Sets `" $ident "` back if the window var updates with a new value."]
                #[property(context)]
                pub fn $ident(child: impl UiNode, $ident: impl IntoVar<Length>) -> impl UiNode {
                    struct [<Window $ident:camel Node>] <C, V> {
                        child: C,
                        $ident: V,
                    }
                    #[impl_ui_node(child)]
                    impl<C: UiNode, V: Var<Length>> [<Window $ident:camel Node>]<C, V> {
                        fn set(&mut self, ctx: &mut WidgetContext) {
                            let $ident = *self.$ident.get(ctx);
                            if $ident.is_finite() {
                                let $var = ctx
                                    .window_state
                                    .get::<WindowVars>()
                                    .expect("no `WindowVars` in `window_state`")
                                    .$var();
                                if $ident != $var.get(ctx).$member {
                                    $var.modify(ctx.vars, move |s| s.$member = $ident);
                                }
                            }
                        }

                        #[UiNode]
                        fn init(&mut self, ctx: &mut WidgetContext) {
                            self.set(ctx);
                            self.child.init(ctx);
                        }

                        #[UiNode]
                        fn update(&mut self, ctx: &mut WidgetContext) {
                            if self.$ident.is_new(ctx) {
                                self.set(ctx);
                            } else if let Some($var) = ctx
                                .window_state
                                .get::<WindowVars>()
                                .expect("no `WindowVars` in `window_state`")
                                .$var()
                                .get_new(ctx)
                            {
                                let _ = self.$ident.set_ne(ctx.vars, $var.$member);
                            }
                            self.child.update(ctx);
                        }
                    }
                    [<Window $ident:camel Node>] {
                        child,
                        $ident: $ident.into_var()
                    }
                }
            })+}
        }
        modify_properties! {
            width = size.width,
            height = size.height,

            min_width = min_size.width,
            min_height = min_size.height,
            max_width = max_size.width,
            max_height = max_size.height,

            x = position.x,
            y = position.y,
        }
    }
}

#[cfg(not(debug_assertions))]
fn print_frame_inspector() -> impl WidgetHandler<ShortcutArgs> {
    hn!(|_, _| {})
}

#[cfg(debug_assertions)]
fn print_frame_inspector() -> impl WidgetHandler<ShortcutArgs> {
    use crate::core::debug::{write_frame, WriteFrameState};

    let mut state = WriteFrameState::none();
    hn!(|ctx, args: &ShortcutArgs| {
        if args.shortcut == shortcut!(CTRL | SHIFT + I) {
            args.stop_propagation();

            let frame = ctx
                .services
                .req::<crate::core::window::Windows>()
                .window(ctx.path.window_id())
                .unwrap()
                .frame_info();

            write_frame(frame, &state, &mut std::io::stderr());

            state = WriteFrameState::new(&frame);
        }
    })
}
