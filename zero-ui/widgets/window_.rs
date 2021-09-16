use crate::core::focus::*;
use crate::core::window::{HeadlessMonitor, StartPosition, Window};
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
        /// Set to `Px::MAX` to not give a position. This variable is not updated back
        /// if the user moves the window, you can use the [`actual_position`](#wp-actual_position) to get the
        /// computed position.
        ///
        /// The position is computed in relation to the [`monitor`](#wp-monitor) value and is re-applied every
        /// time this property or monitor updates.
        ///
        /// You can also set [`x`](#wp-x) and [`y`](#wp-y) as independent properties.
        properties::position;

        /// Monitor used for calculating the [`start_position`], [`position`] and [`size`] of the window.
        ///
        /// When the window is dragged to a different monitor this property does not update, you can use the
        /// [`actual_monitor`] property to get the current monitor.
        ///
        /// You can change this property after the window has opened to move the window to a different monitor,
        /// see [`WindowVars::monitor`] for more details about this function.
        ///
        /// Is the [`MonitorQuery::Primary`] by default.
        ///
        /// [`start_position`]: #wp-start_position
        /// [`position`]: #wp-position
        /// [`size`]: #wp-size
        /// [`WindowVars::monitor`]: crate::core::window::WindowVars::monitor
        properties::monitor;

        /// Window position *x*.
        ///
        /// This property value is the same as the [`position.x`](#wp-position) value.
        properties::x;

        /// Window position *y*.
        ///
        /// This property value is the same as the [`position.y`](#wp-position) value.
        properties::y;

        /// Window size (*width*, *height*).
        ///
        /// Does not include the OS window border.
        ///
        /// You can also set the [`width`](#wp-width) and [`height`](#wp-height) as independent properties.
        properties::size;

        /// Window size *width*.
        ///
        /// This property value is the same as the [`size.width`](#wp-size) value.
        properties::width;

        /// Window size *height*.
        ///
        /// This property value is the same as the [`size.height`](#wp-size) value.
        properties::height;

        /// Window minimum size.
        ///
        /// You can also set the [`min_width`](#wp-min_width) and [`min_height`](#wp-min_height) as independent properties.
        properties::min_size;

        /// Window minimum width.
        ///
        /// This property value is the same as the [`min_size.width`](#wp-min_size) value.
        properties::min_width;

        /// Window minimum height.
        ///
        /// This property value is the same as the [`min_size.height`](#wp-min_size) value.
        properties::min_height;

        /// Window maximum size.
        ///
        /// You can also set the [`max_width`](#wp-max_width) and [`max_height`](#wp-max_height) as independent properties.
        properties::max_size;

        /// Window maximum width.
        ///
        /// This property value is the same as the [`max_size.width`](#wp-max_size) value.
        properties::max_width;

        /// Window maximum height.
        ///
        /// This property value is the same as the [`max_size.height`](#wp-max_size) value.
        properties::max_height;

        /// Window auto-size to content.
        ///
        /// When enabled overwrites [`size`](#wp-size), but is still coerced by [`min_size`](#wp-min_size)
        /// and [`max_size`](#wp-max_size). Auto-size is disabled if the user [manually resizes](#wp-resizable).
        ///
        /// The default value is [`AutoSize::DISABLED`].
        ///
        /// [`AutoSize::DISABLED`]: crate::prelude::AutoSize::DISABLED
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

        /// If the Debug Inspector can be opened for this window.
        ///
        /// The default value is `true`.
        #[cfg(debug_assertions)]
        can_inspect(impl IntoVar<bool>) = true;

        /// Extra configuration for the window when run in [headless mode](crate::core::window::WindowMode::is_headless).
        ///
        /// When a window runs in headed mode some values are inferred by window context, such as the scale factor that
        /// is taken from the monitor. In headless mode these values can be configured manually.
        #[allowed_in_when = false]
        headless_monitor(impl IntoValue<HeadlessMonitor>) = HeadlessMonitor::default();

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
        /// This event notifies once per window, after the window content is inited and the first frame was send to the renderer.
        /// Note that the first frame metadata is available in [`Windows::frame_info`], but it probably has not finished rendering.
        ///
        /// This property is the [`on_pre_window_open`](fn@on_pre_window_open) so window handlers see it first.
        on_pre_window_open as on_open;

        /// On window close requested.
        ///
        /// This event notifies every time the user or the app tries to close the window, you can call
        /// [`cancel`](WindowCloseRequestedArgs::cancel) to stop the window from being closed.
        on_window_close_requested as on_close_requested;

        /// Event just after a frame finishes rendering.
        ///
        /// You can call [`Windows::frame_pixels`] to get a copy of the pixels.
        ///
        /// This property is the [`on_pre_frame_pixels_ready`](fn@on_pre_frame_pixels_ready) so window handlers see it first.
        on_pre_frame_pixels_ready as on_pixels_ready;

        remove {
            // replaced with `root_id` to more clearly indicate that it is not the window ID.
            id;
            // replaced with `visible` because Visibility::Hidden is not a thing for windows.
            visibility
        }
    }

    fn new_event(child: impl UiNode, #[cfg(debug_assertions)] can_inspect: impl IntoVar<bool>) -> impl UiNode {
        let child = commands::close_node(child);
        #[cfg(debug_assertions)]
        let child = commands::inspect_node(child, can_inspect);
        child
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn new(
        child: impl UiNode,
        root_id: WidgetId,
        start_position: impl Into<StartPosition>,
        kiosk: bool,
        headless_monitor: impl Into<HeadlessMonitor>,
    ) -> Window {
        Window::new(root_id, start_position, kiosk, headless_monitor, child)
    }

    /// Window stand-alone properties.
    ///
    /// These properties are already included in the [`window!`](mod@crate::widgets::window) definition,
    /// but you can also use then stand-alone. They configure the window from any widget inside the window.
    pub mod properties {
        use std::marker::PhantomData;

        use crate::core::window::{AutoSize, MonitorQuery, WindowChrome, WindowIcon, WindowId, WindowVars, WindowVarsKey};
        use crate::prelude::new_property::*;

        fn bind_window_var<T, V>(child: impl UiNode, user_var: impl IntoVar<T>, select: impl Fn(&WindowVars) -> V + 'static) -> impl UiNode
        where
            T: VarValue + PartialEq,
            V: Var<T>,
        {
            struct BindWindowVarNode<C, V, S, T> {
                _p: PhantomData<T>,
                child: C,
                user_var: V,
                select: S,
                binding: Option<VarBindingHandle>,
            }

            #[impl_ui_node(child)]
            impl<T, C, V, SV, S> UiNode for BindWindowVarNode<C, V, S, T>
            where
                T: VarValue + PartialEq,
                C: UiNode,
                V: Var<T>,
                SV: Var<T>,
                S: Fn(&WindowVars) -> SV + 'static,
            {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let window_var = (self.select)(ctx.window_state.req(WindowVarsKey));
                    if self.user_var.can_update() {
                        self.binding = Some(self.user_var.bind_bidi(ctx.vars, &window_var));
                    }
                    window_var.set_ne(ctx.vars, self.user_var.get_clone(ctx.vars)).unwrap();
                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.binding = None;
                    self.child.deinit(ctx);
                }
            }
            BindWindowVarNode {
                _p: PhantomData,
                child,
                user_var: user_var.into_var(),
                select,
                binding: None,
            }
        }

        // Properties that set the full value.
        macro_rules! set_properties {
            ($(
                $ident:ident: $Type:ty,
            )+) => {
                $(paste::paste! {
                    #[doc = "Binds the [`"$ident"`](WindowVars::"$ident") window var with the property value."]
                    ///
                    /// The binding is bidirectional and the window variable is assigned on init.
                    #[property(context)]
                    pub fn $ident(child: impl UiNode, $ident: impl IntoVar<$Type>) -> impl UiNode {
                        bind_window_var(child, $ident, |w|w.$ident().clone())
                    }
                })+
            }
        }
        set_properties! {
            position: Point,
            monitor: MonitorQuery,

            size: Size,
            min_size: Size,
            max_size: Size,

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

        macro_rules! map_properties {
            ($(
                $ident:ident . $member:ident = $name:ident : $Type:ty,
            )+) => {$(paste::paste! {
                #[doc = "Binds the `"$member"` of the [`"$ident"`](WindowVars::"$ident") window var with the property value."]
                ///
                /// The binding is bidirectional and the window variable is assigned on init.
                #[property(context)]
                pub fn $name(child: impl UiNode, $name: impl IntoVar<$Type>) -> impl UiNode {
                    bind_window_var(child, $name, |w|w.$ident().map_ref_bidi(|v| &v.$member, |v|&mut v.$member))
                }
            })+}
        }
        map_properties! {
            position.x = x: Length,
            position.y = y: Length,
            size.width = width: Length,
            size.height = height: Length,
            min_size.width = min_width: Length,
            min_size.height = min_height: Length,
            max_size.width = max_width: Length,
            max_size.height = max_height: Length,
        }

        // TODO read-only properties.
    }

    /// Commands that control the window.
    pub mod commands {
        use zero_ui::core::{
            command::*,
            context::WidgetContext,
            event::EventUpdateArgs,
            focus::FocusExt,
            gesture::*,
            window::{WindowFocusChangedEvent, WindowsExt},
            *,
        };
        use zero_ui_core::var::RcVar;

        command! {
            /// Represents the window **close** action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Close Window"                                        |
            /// | [`info`]     | "Close the current window."                           |
            /// | [`shortcut`] | `ALT+F4`, `CTRL+W`                                    |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            pub CloseCommand
                .init_name("Close Window")
                .init_info("Close the current window.")
                .init_shortcut([shortcut!(ALT+F4), shortcut!(CTRL+W)]);

            /// Represent the window **inspect** action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Debug Inspector"                                     |
            /// | [`info`]     | "Inspect the current window."                         |
            /// | [`shortcut`] | `CTRL+SHIFT+I`, `F12`                                 |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            pub InspectCommand
                .init_name("Debug Inspector")
                .init_info("Inspect the current window.")
                .init_shortcut([shortcut!(CTRL|SHIFT+I), shortcut!(F12)]);
        }

        pub(super) fn close_node(child: impl UiNode) -> impl UiNode {
            struct OnCloseNode<C: UiNode> {
                child: C,
                handle: CommandHandle,
                allow_alt_f4: RcVar<bool>,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for OnCloseNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let window_id = ctx.path.window_id();
                    self.allow_alt_f4 = ctx.services.windows().vars(window_id).unwrap().allow_alt_f4().clone();

                    let enabled = ctx.services.focus().is_window_focused(window_id);
                    self.handle = CloseCommand.new_handle(ctx, enabled);

                    // Highjacks ALT+F4 in Windows.
                    self.allow_alt_f4
                        .set_ne(ctx.vars, CloseCommand.shortcut().get(ctx.vars).contains(shortcut![ALT + F4]));

                    self.child.init(ctx)
                }
                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    if let Some(args) = CloseCommand.update(args) {
                        // command requested, run it only if we are focused.
                        let window_id = ctx.path.window_id();
                        if ctx.services.focus().is_window_focused(window_id) {
                            let _ = ctx.services.windows().close(window_id);
                        }
                        self.child.event(ctx, args)
                    } else if let Some(args) = WindowFocusChangedEvent.update(args) {
                        // toggle enabled if our window activated/deactivated.
                        if args.window_id == ctx.path.window_id() {
                            self.handle.set_enabled(args.focused);
                        }
                        self.child.event(ctx, args);
                    } else {
                        self.child.event(ctx, args)
                    }
                }
                fn update(&mut self, ctx: &mut WidgetContext) {
                    // update the ALT+F4 block flag in Windows.
                    #[cfg(windows)]
                    if let Some(s) = CloseCommand.shortcut().get_new(ctx.vars) {
                        self.allow_alt_f4.set_ne(ctx.vars, s.contains(shortcut![ALT + F4]));
                    }

                    self.child.update(ctx);
                }
            }

            OnCloseNode {
                child,
                handle: CommandHandle::dummy(),
                allow_alt_f4: var::var(false), // dummy
            }
        }

        #[cfg(debug_assertions)]
        pub(super) fn inspect_node(child: impl UiNode, can_inspect: impl var::IntoVar<bool>) -> impl UiNode {
            use crate::core::{
                debug::{write_frame, WriteFrameState},
                var::Var,
            };

            let mut state = WriteFrameState::none();

            let can_inspect = can_inspect.into_var();

            on_command(
                child,
                |ctx| InspectCommand.scoped(ctx.path.window_id()),
                move |ctx| can_inspect.copy(ctx) && ctx.services.focus().is_window_focused(ctx.path.window_id()),
                hn!(|ctx, args: &CommandArgs| {
                    args.stop_propagation();

                    let frame = ctx.services.windows().frame_info(ctx.path.window_id()).unwrap();

                    let mut buffer = vec![];
                    write_frame(frame, &state, &mut buffer);

                    state = WriteFrameState::new(frame);

                    task::spawn_wait(move || {
                        use std::io::*;
                        stdout()
                            .write_all(&buffer)
                            .unwrap_or_else(|e| log::error!("error printing frame {}", e));
                    });
                }),
            )
        }
    }
}
