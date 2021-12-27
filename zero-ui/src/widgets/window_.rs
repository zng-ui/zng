use crate::core::focus::*;
use crate::core::window::{HeadlessMonitor, RenderMode, StartPosition, Window};
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

        /// Window state.
        ///
        /// If set to a writeable variable it is updated back if the user changes the window state.
        ///
        /// See [`WindowState`] for details.
        properties::state;

        /// Window position (*x*, *y*).
        ///
        /// The position is computed in relation to the [`monitor`](#wp-monitor) value and is re-applied every
        /// time this property or monitor updates.
        ///
        /// Setting [`Length::Default`] to either *x* or *y* causes the system initial position to be used in both dimensions.
        /// This variable is not updated back if the user moves the window, you can use the [`actual_position`](#wp-actual_position)
        /// to get the computed position.
        ///
        /// You can also set [`x`](#wp-x) and [`y`](#wp-y) as independent properties.
        properties::position;

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

        /// Window clear color.
        ///
        /// Color used to *clear* the previous frame pixels before rendering a new frame.
        /// It is visible if window content does not completely fill the content area, this
        /// can happen if you do not set a background or the background is semi-transparent, also
        /// can happen during very fast resizes.
        properties::clear_color = rgb(0.1, 0.1, 0.1);

        /// Unique identifier of the window root widget.
        #[allowed_in_when = false]
        root_id(impl IntoValue<WidgetId>) = WidgetId::new_unique();

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

        /// Frame image capture mode.
        ///
        /// This property is specially useful headless windows that are used to render.
        properties::frame_capture_mode;

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

        /// If semi-transparent content is "see-through", mixin with the OS pixels "behind" the window.
        ///
        /// This is `true` by default, as it avoids the screen flashing black for windows opening in maximized or fullscreen modes
        /// in the Microsoft Windows OS.
        ///
        /// Note that to make use of this feature you must unset the [`clear_color`] and [`background_color`] or set then to
        /// a semi-transparent color. The composition is a simple alpha blend, effects like blur do not apply to
        /// the pixels "behind" the window.
        ///
        /// [`clear_color`]: #wp-clear_color
        /// [`background_color`]: #wp-background_color
        #[allowed_in_when = false]
        allow_transparency(bool) = true;

        /// Render performance mode overwrite for this window, if set to `None` the [`Windows::default_render_mode`] is used.
        ///
        /// # Examples
        ///
        /// Prefer `Integrated` renderer backend for just this window:
        ///
        /// ```no_run
        /// use zero_ui::core::window::RenderMode;
        /// use zero_ui::prelude::*;
        ///
        /// fn example(ctx: &mut WindowContext) -> Window {
        ///     let selected_mode = ctx.window_state.req(WindowVarsKey).render_mode();
        ///     window! {
        ///         title = "Render Mode";
        ///         render_mode = RenderMode::Integrated;
        ///         content = text(selected_mode.map(|m| formatx!("Preference: Integrated\nActual: {:?}", m)));
        ///     }
        /// }
        /// ```
        ///
        /// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
        /// see [`RenderMode`] for more details about each mode and fallbacks.
        #[allowed_in_when = false]
        render_mode(impl IntoValue<Option<RenderMode>>) = None;

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

        /// On window deinited.
        ///
        /// This event notifies once after the window content is deinited because it is closing.
        crate::properties::events::widget::on_deinit as on_close;

        /// On window position changed.
        ///
        /// This event notifies every time the user or app changes the window position. You can also track the window
        /// position using the [`actual_position`] variable.
        ///
        /// This property is the [`on_pre_window_moved`] so window handlers see it first.
        ///
        /// [`actual_position`]: WindowVars::actual_position
        /// [`on_pre_window_moved`]: fn@on_pre_window_moved
        on_pre_window_moved as on_moved;

        /// On window size changed.
        ///
        /// This event notifies every time the user or app changes the window content area size. You can also track
        /// the window size using the [`actual_size`] variable.
        ///
        /// This property is the [`on_pre_window_resized`] so window handlers see it first.
        ///
        /// [`actual_size`]: WindowVars::actual_size
        /// [`on_pre_window_resized`]: fn@on_pre_window_resized
        on_pre_window_resized as on_resized;

        /// On window state changed.
        ///
        /// This event notifies every time the user or app changes the window state. You can also track the window
        /// state by setting [`state`] to a read-write variable.
        ///
        /// This property is the [`on_pre_window_state_changed`] so window handlers see it first.
        ///
        /// [`state`]: #wp-state
        /// [`on_pre_window_state_changed`]: fn@on_pre_window_state_changed
        on_pre_window_state_changed as on_state_changed;

        /// On window maximized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@on_pre_window_maximized
        on_pre_window_maximized as on_maximized;

        /// On window exited the maximized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from maximized.
        ///
        /// This property is the [`on_pre_window_unmaximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@on_pre_window_maximized
        on_pre_window_unmaximized as on_unmaximized;

        /// On window minimized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_minimized`]: fn@on_pre_window_minimized
        on_pre_window_minimized as on_minimized;

        /// On window exited the minimized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from minimized.
        ///
        /// This property is the [`on_pre_window_unminimized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unminimized`]: fn@on_pre_window_unminimized
        on_pre_window_unminimized as on_unminimized;

        /// On window state changed to [`Normal`].
        ///
        /// This event notifies every time the user or app changes the window state to [`Normal`].
        ///
        /// This property is the [`on_pre_window_restored`] so window handlers see it first.
        ///
        /// [`Normal`]: WindowState::Normal
        /// [`on_pre_window_restored`]: fn@on_pre_window_restored
        on_pre_window_restored as on_restored;

        /// On window enter one of the fullscreen states.
        ///
        /// This event notifies every time the user or app changes the window state to [`Fullscreen`] or [`Exclusive`].
        ///
        /// This property is the [`on_pre_window_fullscreen`] so window handlers see it first.
        ///
        /// [`Fullscreen`]: WindowState::Fullscreen
        /// [`Exclusive`]: WindowState::Exclusive
        /// [`on_pre_window_fullscreen`]: fn@on_pre_window_fullscreen
        on_pre_window_fullscreen as on_fullscreen;

        /// On window is no longer fullscreen.
        ///
        /// This event notifies every time the user or app changed the window state to one that is not fullscreen.
        ///
        /// This property is the [`on_pre_window_exited_fullscreen`] so window handlers see it first.
        ///
        /// [`on_pre_window_exited_fullscreen`]: fn@on_pre_window_exited_fullscreen
        on_pre_window_exited_fullscreen as on_exited_fullscreen;

        /// On window frame rendered.
        ///
        /// If [`frame_image_capture`](#wp-frame_image_capture) is set
        on_pre_frame_image_ready as on_frame_image_ready;

        remove {
            // replaced with `root_id` to more clearly indicate that it is not the window ID.
            id;
            // replaced with `visible` because Visibility::Hidden is not a thing for windows.
            visibility
        }
    }

    fn new_event(child: impl UiNode, #[cfg(debug_assertions)] can_inspect: impl IntoVar<bool>) -> impl UiNode {
        let child = commands::window_control_node(child);
        #[cfg(debug_assertions)]
        let child = commands::inspect_node(child, can_inspect);
        child
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn new(
        child: impl UiNode,
        root_id: impl IntoValue<WidgetId>,
        start_position: impl IntoValue<StartPosition>,
        kiosk: bool,
        allow_transparency: bool,
        render_mode: impl IntoValue<Option<RenderMode>>,
        headless_monitor: impl IntoValue<HeadlessMonitor>,
    ) -> Window {
        Window::new(
            root_id,
            start_position,
            kiosk,
            allow_transparency,
            render_mode,
            headless_monitor,
            child,
        )
    }

    /// Window stand-alone properties.
    ///
    /// These properties are already included in the [`window!`](mod@crate::widgets::window) definition,
    /// but you can also use then stand-alone. They configure the window from any widget inside the window.
    pub mod properties {
        use std::marker::PhantomData;

        use crate::core::window::{
            AutoSize, FrameCaptureMode, MonitorQuery, WindowChrome, WindowIcon, WindowId, WindowState, WindowVars, WindowVarsKey,
        };
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
                    #[doc = "Binds the [`"$ident "`](WindowVars::"$ident ") window var with the property value."]
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

            state: WindowState,

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

            frame_capture_mode: FrameCaptureMode,
        }

        macro_rules! map_properties {
            ($(
                $ident:ident . $member:ident = $name:ident : $Type:ty,
            )+) => {$(paste::paste! {
                #[doc = "Binds the `"$member "` of the [`"$ident "`](WindowVars::"$ident ") window var with the property value."]
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

        /// Sets the frame clear color.
        #[property(context, default(colors::WHITE))]
        pub fn clear_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            struct ClearColorNode<U, C> {
                child: U,
                clear_color: C,
            }
            #[impl_ui_node(child)]
            impl<U: UiNode, C: Var<Rgba>> UiNode for ClearColorNode<U, C> {
                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    subscriptions.var(ctx, &self.clear_color);
                    self.child.subscriptions(ctx, subscriptions);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if self.clear_color.is_new(ctx) {
                        ctx.updates.render_update();
                    }
                    self.child.update(ctx);
                }
                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    frame.set_clear_color(self.clear_color.copy(ctx).into());
                    self.child.render(ctx, frame);
                }
                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    update.set_clear_color(self.clear_color.copy(ctx).into());
                    self.child.render_update(ctx, update);
                }
            }
            ClearColorNode {
                child,
                clear_color: color.into_var(),
            }
        }

        // TODO read-only properties.
    }

    /// Commands that control the window.
    pub mod commands {
        use zero_ui::core::{
            command::*,
            context::{InfoContext, WidgetContext},
            event::EventUpdateArgs,
            focus::FocusExt,
            gesture::*,
            var::*,
            widget_info::WidgetSubscriptions,
            window::{WindowFocusChangedEvent, WindowsExt},
            *,
        };
        use zero_ui_core::window::WindowState;

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

            /// Represents the window **minimize** action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Minimize Window"                                     |
            /// | [`info`]     | "Minimize the current window."                        |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            pub MinimizeCommand
                .init_name("Minimize Window")
                .init_info("Minimize the current window.");

            /// Represents the window **maximize** action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Maximize Window"                                     |
            /// | [`info`]     | "Maximize the current window."                        |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            pub MaximizeCommand
                .init_name("Maximize Window")
                .init_info("Maximize the current window.");

            /// Represents the window **toggle fullscreen** action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Full-Screen"                                         |
            /// | [`info`]     | "Toggle full-screen mode on the current window."      |
            /// | [`shortcut`] | `CMD+SHIFT+F` on MacOS, `F11` on other systems.       |
            ///
            /// # Behavior
            ///
            /// This command is about the *windowed* fullscreen state ([`WindowState::Fullscreen`]),
            /// use the [`ExclusiveFullscreenCommand`] to toggle *exclusive* video mode fullscreen.
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            /// [`shortcut`]: CommandShortcutExt
            pub FullscreenCommand
                .init_name("Full-Screen")
                .init_info("Toggle full-screen mode on the current window.")
                .init_shortcut({
                    if cfg!(target_os = "macos") {
                        shortcut!(CTRL|SHIFT+F)
                    } else {
                        shortcut!(F11)
                    }
                });

            /// Represents the window **toggle fullscreen** action.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                 |
            /// |--------------|-------------------------------------------------------|
            /// | [`name`]     | "Minimize Window"                                     |
            /// | [`info`]     | "Minimize the current window."                        |
            ///
            /// # Behavior
            ///
            /// This command is about the *exclusive* fullscreen state ([`WindowSTate::Exclusive`]),
            /// use the [`FullscreenCommand`] to toggle *windowed* fullscreen.
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            pub ExclusiveFullscreenCommand
                .init_name("Exclusive Full-Screen")
                .init_info("Toggle exclusive full-screen mode on the current window.");

            /// Represents the window **restore** action.
            ///
            /// Restores the window to its previous not-minimized state or normal state.
            ///
            /// # Metadata
            ///
            /// This command initializes with the following metadata:
            ///
            /// | metadata     | value                                                                      |
            /// |--------------|----------------------------------------------------------------------------|
            /// | [`name`]     | "Restore Window"                                                           |
            /// | [`info`]     | "Restores the window to its previous not-minimized state or normal state." |
            ///
            /// [`name`]: CommandNameExt
            /// [`info`]: CommandInfoExt
            pub RestoreCommand
                .init_name("Restore Window")
                .init_info("Restores the window to its previous not-minimized state or normal state.");

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

        pub(super) fn window_control_node(child: impl UiNode) -> impl UiNode {
            struct WindowControlNode<C: UiNode> {
                child: C,
                maximize_handle: CommandHandle,
                minimize_handle: CommandHandle,
                restore_handle: CommandHandle,
                close_handle: CommandHandle,
                fullscreen_handle: CommandHandle,
                exclusive_handle: CommandHandle,

                allow_alt_f4_binding: VarBindingHandle,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for WindowControlNode<C> {
                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    subscriptions
                        .event(WindowFocusChangedEvent)
                        .event(MaximizeCommand)
                        .event(MinimizeCommand)
                        .event(FullscreenCommand)
                        .event(ExclusiveFullscreenCommand)
                        .event(RestoreCommand)
                        .event(CloseCommand);

                    self.child.subscriptions(ctx, subscriptions);
                }

                fn init(&mut self, ctx: &mut WidgetContext) {
                    let window_id = ctx.path.window_id();
                    let enabled = ctx.services.focus().is_window_focused(window_id).copy(ctx.vars);

                    // state
                    self.maximize_handle = MaximizeCommand.new_handle(ctx, enabled);
                    self.minimize_handle = MinimizeCommand.new_handle(ctx, enabled);
                    self.fullscreen_handle = FullscreenCommand.new_handle(ctx, enabled);
                    self.exclusive_handle = ExclusiveFullscreenCommand.new_handle(ctx, enabled);
                    self.restore_handle = RestoreCommand.new_handle(ctx, enabled);

                    // close
                    self.close_handle = CloseCommand.new_handle(ctx, enabled);

                    if cfg!(windows) {
                        // hijacks allow_alt_f4 for the close command, if we don't do this
                        // the view-process can block the key press and send a close event
                        // without the CloseCommand event ever firing.
                        let allow_alt_f4 = ctx.services.windows().vars(window_id).unwrap().allow_alt_f4();
                        self.allow_alt_f4_binding = CloseCommand
                            .shortcut()
                            .bind_map(ctx.vars, allow_alt_f4, |_, s| s.contains(shortcut![ALT + F4]));
                    }

                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.allow_alt_f4_binding = VarBindingHandle::dummy();
                    self.child.deinit(ctx);
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    fn set_state(ctx: &mut WidgetContext, state: WindowState) {
                        let window_id = ctx.path.window_id();
                        if ctx.services.focus().is_window_focused(window_id).copy(ctx.vars) {
                            let _ = ctx.services.windows().vars(window_id).unwrap().state().set_ne(ctx.vars, state);
                        }
                    }

                    if let Some(args) = WindowFocusChangedEvent.update(args) {
                        // toggle enabled if our window activated/deactivated.
                        if args.window_id == ctx.path.window_id() {
                            // TODO should handle enabled just be a `var`?
                            self.maximize_handle.set_enabled(args.focused);
                            self.maximize_handle.set_enabled(args.focused);
                            self.restore_handle.set_enabled(args.focused);
                            self.close_handle.set_enabled(args.focused);
                        }
                        self.child.event(ctx, args);
                    } else if let Some(args) = MaximizeCommand.update(args) {
                        set_state(ctx, WindowState::Maximized);
                        self.child.event(ctx, args);
                    } else if let Some(args) = MinimizeCommand.update(args) {
                        set_state(ctx, WindowState::Minimized);
                        self.child.event(ctx, args);
                    } else if let Some(args) = RestoreCommand.update(args) {
                        let window_id = ctx.path.window_id();
                        if ctx.services.focus().is_window_focused(window_id).copy(ctx.vars) {
                            let vars = ctx.services.windows().vars(window_id).unwrap();
                            vars.state().set_ne(ctx.vars, vars.restore_state().copy(ctx.vars));
                        }
                        self.child.event(ctx, args);
                    } else if let Some(args) = CloseCommand.update(args) {
                        // close requested, run it only if we are focused.
                        let window_id = ctx.path.window_id();
                        if ctx.services.focus().is_window_focused(window_id).copy(ctx.vars) {
                            let _ = ctx.services.windows().close(window_id);
                        }
                        self.child.event(ctx, args)
                    } else if let Some(args) = FullscreenCommand.update(args) {
                        // fullscreen or restore.
                        let window_id = ctx.path.window_id();
                        if ctx.services.focus().is_window_focused(window_id).copy(ctx.vars) {
                            let vars = ctx.services.windows().vars(window_id).unwrap();
                            if vars.state().copy(ctx.vars).is_fullscreen() {
                                vars.state().set(ctx.vars, vars.restore_state().copy(ctx.vars));
                            } else {
                                vars.state().set(ctx.vars, WindowState::Fullscreen);
                            }
                        }
                        self.child.event(ctx, args)
                    } else if let Some(args) = ExclusiveFullscreenCommand.update(args) {
                        // exclusive fullscreen or restore.
                        let window_id = ctx.path.window_id();
                        if ctx.services.focus().is_window_focused(window_id).copy(ctx.vars) {
                            let vars = ctx.services.windows().vars(window_id).unwrap();
                            if vars.state().copy(ctx.vars).is_fullscreen() {
                                vars.state().set(ctx.vars, vars.restore_state().copy(ctx.vars));
                            } else {
                                vars.state().set(ctx.vars, WindowState::Exclusive);
                            }
                        }
                        self.child.event(ctx, args)
                    } else {
                        self.child.event(ctx, args)
                    }
                }
            }
            WindowControlNode {
                child,
                maximize_handle: CommandHandle::dummy(),
                minimize_handle: CommandHandle::dummy(),
                restore_handle: CommandHandle::dummy(),
                fullscreen_handle: CommandHandle::dummy(),
                exclusive_handle: CommandHandle::dummy(),
                close_handle: CommandHandle::dummy(),
                allow_alt_f4_binding: VarBindingHandle::dummy(),
            }
        }

        #[cfg(debug_assertions)]
        pub(super) fn inspect_node(child: impl UiNode, can_inspect: impl var::IntoVar<bool>) -> impl UiNode {
            use crate::core::inspector::{write_tree, WriteTreeState};

            let mut state = WriteTreeState::none();

            let can_inspect = can_inspect.into_var();

            on_command(
                child,
                |ctx| InspectCommand.scoped(ctx.path.window_id()),
                move |ctx| {
                    let is_win_focused = ctx.services.focus().is_window_focused(ctx.path.window_id());
                    expr_var! { *#{can_inspect.clone()} && *#{is_win_focused} }
                },
                hn!(|ctx, args: &CommandArgs| {
                    args.stop_propagation();

                    let frame = ctx.services.windows().widget_tree(ctx.path.window_id()).unwrap();

                    let mut buffer = vec![];
                    write_tree(frame, &state, &mut buffer);

                    state = WriteTreeState::new(frame);

                    task::spawn_wait(move || {
                        use std::io::*;
                        stdout()
                            .write_all(&buffer)
                            .unwrap_or_else(|e| tracing::error!("error printing frame {}", e));
                    });
                }),
            )
        }
    }
}
