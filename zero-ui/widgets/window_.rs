use crate::core::focus::*;
use crate::core::gesture::*;
use crate::core::window::{HeadlessScreen, RedrawArgs, StartPosition, Window};
use crate::prelude::new_widget::*;

/// A window container.
///
/// The instance type is [`Window`], witch can be given to the [`Windows`](crate::core::window::Windows) service
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
    use super::*;

    inherit!(container);

    properties! {
        /// Window title.
        properties::title;

        /// Window icon.
        ///
        /// See [`WindowIcon`] for details.
        properties::icon;

        /// Window position when it opens.
        #[allowed_in_when = false]
        start_position(impl Into<StartPosition>) = StartPosition::Default;

        /// Window position (left, top).
        ///
        ///  If set to a variable it is kept in sync.
        ///
        /// Set to [`f32::NAN`](f32::NAN) to not give an initial position.
        properties::position = {
            // use shared var in debug to allow inspecting the value.
            #[cfg(debug_assertions)]
            let r = crate::core::var::var(crate::core::units::Point::new(f32::NAN, f32::NAN));

            #[cfg(not(debug_assertions))]
            let r = (f32::NAN, f32::NAN);

            r
        };

        /// Window size.
        ///
        /// If set to a variable it is kept in sync.
        ///
        /// Does not include the OS window border.
        properties::size = {
            // use shared var in debug to allow inspecting the value.
            #[cfg(debug_assertions)]
            let r = crate::core::var::var(crate::core::units::Size::new(f32::NAN, f32::NAN));

            #[cfg(not(debug_assertions))]
            let r = (f32::NAN, f32::NAN);

            r
        };

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

        /// Windows remember the last focused widget and return focus when activated again.
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
        headless_screen(impl Into<HeadlessScreen>) = HeadlessScreen::default();

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

        /// Event just before the window frame is redraw.
        #[allowed_in_when = false]
        on_pre_redraw(impl FnMut(&mut RedrawArgs) + 'static) = |_| {};

        /// Event just after the window frame is redraw.
        #[allowed_in_when = false]
        on_redraw(impl FnMut(&mut RedrawArgs) + 'static) = |_| {};

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

        fn set_windowin_var<T: VarValue + PartialEq>(
            child: impl UiNode,
            local_var: impl IntoVar<T>,
            win_var: impl Fn(&WindowVars) -> &RcVar<T> + 'static,
        ) -> impl UiNode {
            struct SetWindowVar<C, P, W> {
                child: C,
                local_var: P,
                win_var: W,
            }

            #[impl_ui_node(child)]
            impl<C: UiNode, T: VarValue + PartialEq, P: Var<T>, W: Fn(&WindowVars) -> &RcVar<T> + 'static> SetWindowVar<C, P, W> {
                fn win_var<'a>(&self, window_state: &'a StateMap) -> &'a RcVar<T> {
                    let vars = window_state.get::<WindowVars>().expect("no `WindowVars` in `window_state`");
                    (self.win_var)(vars)
                }

                fn set(&mut self, ctx: &mut WidgetContext) {
                    let win_var = self.win_var(ctx.window_state);
                    let local_val = self.local_var.get(ctx.vars);
                    if win_var.get(ctx.vars) != local_val {
                        win_var.set(ctx.vars, local_val.clone());
                    }
                }

                #[UiNode]
                fn init(&mut self, ctx: &mut WidgetContext) {
                    // local_var => win_var
                    self.set(ctx);
                    self.child.init(ctx);
                }

                #[UiNode]
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if self.local_var.is_new(ctx.vars) {
                        // local_var ==> win_var
                        self.set(ctx);
                    } else if !self.local_var.is_read_only(ctx.vars) {
                        let win_var = self.win_var(ctx.window_state);
                        if let Some(win_val) = win_var.get_new(ctx.vars) {
                            // local_var <== win_var
                            if win_val != self.local_var.get(ctx.vars) {
                                let _ = self.local_var.set(ctx.vars, win_val.clone());
                            }
                        }
                    }
                    self.child.update(ctx);
                }
            }

            SetWindowVar {
                child,
                local_var: local_var.into_var(),
                win_var,
            }
        }

        macro_rules! declare {
            ($(
                $ident:ident: $Type:ty,
            )+) => {
                $(paste::paste! {
                    #[doc = "Sets the [`WindowVars::" $ident "`]."]
                    ///
                    #[doc = "Sets `" $ident "` back if the window var updates with a new value."]
                    #[property(context)]
                    pub fn $ident(child: impl UiNode, $ident: impl IntoVar<$Type>) -> impl UiNode {
                        set_windowin_var(child, $ident, |v| v.$ident())
                    }
                })+
            }
        }
        declare! {
            chrome: WindowChrome,
            icon: WindowIcon,
            title: Text,

            position: Point,

            size: Size,
            min_size: Size,
            max_size: Size,
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
    }
}

#[cfg(not(debug_assertions))]
fn print_frame_inspector() -> impl FnMut(&mut WidgetContext, &ShortcutArgs) {
    |_, _| {}
}

#[cfg(debug_assertions)]
fn print_frame_inspector() -> impl FnMut(&mut WidgetContext, &ShortcutArgs) {
    use crate::core::debug::{write_frame, WriteFrameState};

    let mut state = WriteFrameState::none();
    move |ctx, args| {
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
    }
}
