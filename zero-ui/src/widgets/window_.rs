use crate::core::focus::*;
use crate::core::window::{HeadlessMonitor, RenderMode, StartPosition, Window};
use crate::prelude::new_widget::*;
use crate::properties::events::window::*;

/// A window container.
///
/// The instance type is [`Window`], that can be given to the [`Windows`](crate::core::window::Windows) service
/// to open a system window that is kept in sync with the window properties set in the widget.
///
/// # Examples
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
        ///
        /// [`WindowIcon`]: crate::core::window::WindowIcon
        properties::icon;

        /// Window chrome, the non-client area of the window.
        ///
        /// See [`WindowChrome`] for details.
        ///
        /// [`WindowChrome`]: crate::core::window::WindowChrome
        properties::chrome;

        /// Window position when it opens.
        #[allowed_in_when = false]
        start_position(impl IntoValue<StartPosition>) = StartPosition::Default;

        /// Window state.
        ///
        /// If set to a writeable variable it is updated back if the user changes the window state.
        ///
        /// See [`WindowState`] for details.
        ///
        /// [`WindowState`]: crate::core::window::WindowState
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

        /// The point in the window content that does not move when the window is resized by [`auto_size`].
        ///
        /// When the window size increases it *grows* to the right-bottom, the top-left corner does not move because
        /// the origin of window position is at the top-left and the position did not change, this variables overwrites this origin
        /// for [`auto_size`] resizes, the window position is adjusted so that it is the *center* of the resize.
        ///
        /// Note this only applies to auto-resizes, the initial auto-size when the window opens is positioned
        /// according to the [`start_position`] value.
        ///
        /// The default value is [`Point::top_left`].
        ///
        /// [`auto_size`]: #wp-auto_size
        /// [`start_position`]: #wp-start_position
        properties::auto_size_origin;

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
        /// [`MonitorQuery::Primary`]: crate::core::window::MonitorQuery::Primary
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
        ///         content = text(selected_mode.map(|m| formatx!("Preference: Integrated\nActual: {m:?}")));
        ///     }
        /// }
        /// ```
        ///
        /// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
        /// see [`RenderMode`] for more details about each mode and fallbacks.
        ///
        /// [`Windows::default_render_mode`]: crate::core::window::Windows::default_render_mode
        #[allowed_in_when = false]
        render_mode(impl IntoValue<Option<RenderMode>>) = None;

        /// Event just after the window opens.
        ///
        /// This event notifies once per window, after the window content is inited and the first frame was send to the renderer.
        /// Note that the first frame metadata is available in [`Windows::widget_tree`], but it probably has not finished rendering.
        ///
        /// This property is the [`on_pre_window_open`](fn@on_pre_window_open) so window handlers see it first.
        ///
        /// [`Windows::widget_tree`]: crate::core::window::Windows::widget_tree
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
        /// [`on_pre_window_unmaximized`]: fn@on_pre_window_unmaximized
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

        nodes::layers(child)
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
        Window::new_root(
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
            auto_size_origin: Point,

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

    /// Commands that control the scoped window.
    ///
    /// The window widget implements all these commands scoped to the window ID.
    pub mod commands {
        use zero_ui::core::{
            command::*,
            context::{InfoContext, WidgetContext},
            event::EventUpdateArgs,
            gesture::*,
            var::*,
            widget_info::WidgetSubscriptions,
            window::{WindowVarsKey, WindowsExt},
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
                .init_name("Close")
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
                .init_name("Minimize")
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
                .init_name("Maximize")
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
            /// | [`shortcut`] | `CMD|SHIFT+F` on MacOS, `F11` on other systems.       |
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
                .init_name("Restore")
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
            /// | [`shortcut`] | `CTRL|SHIFT+I`, `F12`                                 |
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
            struct WindowControlNode<C> {
                child: C,

                maximize_handle: CommandHandle,
                minimize_handle: CommandHandle,
                restore_handle: CommandHandle,

                fullscreen_handle: CommandHandle,
                exclusive_handle: CommandHandle,

                close_handle: CommandHandle,

                state_var: Option<RcVar<WindowState>>,

                allow_alt_f4_binding: VarBindingHandle,
            }
            impl<C> WindowControlNode<C> {
                fn update_state(&mut self, state: WindowState) {
                    self.restore_handle.set_enabled(state != WindowState::Normal);
                    self.maximize_handle.set_enabled(state != WindowState::Maximized);
                    self.minimize_handle.set_enabled(state != WindowState::Minimized);
                }
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for WindowControlNode<C> {
                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    let scope = ctx.path.window_id();

                    subscriptions
                        .event(MaximizeCommand.scoped(scope))
                        .event(MinimizeCommand.scoped(scope))
                        .event(FullscreenCommand.scoped(scope))
                        .event(ExclusiveFullscreenCommand.scoped(scope))
                        .event(RestoreCommand.scoped(scope))
                        .event(CloseCommand.scoped(scope))
                        .var(ctx, self.state_var.as_ref().unwrap());

                    self.child.subscriptions(ctx, subscriptions);
                }

                fn init(&mut self, ctx: &mut WidgetContext) {
                    let window_id = ctx.path.window_id();

                    // state
                    self.maximize_handle = MaximizeCommand.scoped(window_id).new_handle(ctx, false);
                    self.minimize_handle = MinimizeCommand.scoped(window_id).new_handle(ctx, false);
                    self.fullscreen_handle = FullscreenCommand.scoped(window_id).new_handle(ctx, true);
                    self.exclusive_handle = ExclusiveFullscreenCommand.scoped(window_id).new_handle(ctx, true);
                    self.restore_handle = RestoreCommand.scoped(window_id).new_handle(ctx, false);
                    let state_var = ctx.window_state.req(WindowVarsKey).state().clone();
                    self.update_state(state_var.copy(ctx));
                    self.state_var = Some(state_var);

                    // close
                    self.close_handle = CloseCommand.scoped(window_id).new_handle(ctx, true);

                    if cfg!(windows) {
                        // hijacks allow_alt_f4 for the close command, if we don't do this
                        // the view-process can block the key press and send a close event
                        // without the CloseCommand event ever firing.
                        let allow_alt_f4 = ctx.services.windows().vars(window_id).unwrap().allow_alt_f4();
                        self.allow_alt_f4_binding = CloseCommand
                            .scoped(window_id)
                            .shortcut()
                            .bind_map(ctx.vars, allow_alt_f4, |_, s| s.contains(shortcut![ALT + F4]));
                    }

                    self.child.init(ctx);
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.maximize_handle = CommandHandle::dummy();
                    self.minimize_handle = CommandHandle::dummy();
                    self.restore_handle = CommandHandle::dummy();

                    self.fullscreen_handle = CommandHandle::dummy();
                    self.exclusive_handle = CommandHandle::dummy();

                    self.close_handle = CommandHandle::dummy();
                    self.state_var = None;

                    self.allow_alt_f4_binding = VarBindingHandle::dummy();
                    self.child.deinit(ctx);
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    let scope = ctx.path.window_id();
                    let state_var = self.state_var.as_ref().unwrap();
                    let restore_state = || ctx.window_state.req(WindowVarsKey).restore_state().copy(ctx.vars);

                    if let Some(args) = MaximizeCommand.scoped(scope).update(args) {
                        if self.maximize_handle.is_enabled() {
                            state_var.set_ne(ctx, WindowState::Maximized);
                        }

                        self.child.event(ctx, args);
                        return;
                    }

                    if let Some(args) = MinimizeCommand.scoped(scope).update(args) {
                        if self.minimize_handle.is_enabled() {
                            state_var.set_ne(ctx, WindowState::Minimized);
                        }

                        self.child.event(ctx, args);
                        return;
                    }

                    if let Some(args) = RestoreCommand.scoped(scope).update(args) {
                        if self.restore_handle.is_enabled() {
                            state_var.set_ne(ctx, restore_state());
                        }

                        self.child.event(ctx, args);
                        return;
                    }

                    if let Some(args) = CloseCommand.scoped(scope).update(args) {
                        if self.close_handle.is_enabled() {
                            let _ = ctx.services.windows().close(scope);
                        }

                        self.child.event(ctx, args);
                        return;
                    }

                    if let Some(args) = FullscreenCommand.scoped(scope).update(args) {
                        if self.fullscreen_handle.is_enabled() {
                            if let WindowState::Fullscreen = state_var.copy(ctx) {
                                state_var.set(ctx, restore_state());
                            } else {
                                state_var.set(ctx, WindowState::Fullscreen);
                            }
                        }

                        self.child.event(ctx, args);
                        return;
                    }

                    if let Some(args) = ExclusiveFullscreenCommand.scoped(scope).update(args) {
                        if self.exclusive_handle.is_enabled() {
                            if let WindowState::Exclusive = state_var.copy(ctx) {
                                state_var.set(ctx, restore_state());
                            } else {
                                state_var.set(ctx, WindowState::Exclusive);
                            }
                        }

                        self.child.event(ctx, args);
                        return;
                    }

                    self.child.event(ctx, args);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(state) = self.state_var.as_ref().unwrap().copy_new(ctx) {
                        self.update_state(state);
                    }

                    self.child.update(ctx);
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

                state_var: None,

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
                move |_| can_inspect.clone(),
                hn!(|ctx, args: &CommandArgs| {
                    args.stop_propagation();

                    let mut buffer = vec![];
                    write_tree(ctx.info_tree, &state, &mut buffer);

                    state = WriteTreeState::new(ctx.info_tree);

                    task::spawn_wait(move || {
                        use std::io::*;
                        stdout()
                            .write_all(&buffer)
                            .unwrap_or_else(|e| tracing::error!("error printing frame {e}"));
                    });
                }),
            )
        }
    }

    #[doc(inline)]
    pub use nodes::{AnchorMode, AnchorSize, AnchorTransform, LayerIndex, WindowLayers};

    /// UI nodes used for building a window widget.
    pub mod nodes {
        use crate::prelude::new_property::*;

        /// Windows layers.
        ///
        /// The window layers is z-order stacking panel that fills the window content area, widgets can be inserted
        /// with a *z-index* that is the [`LayerIndex`]. The inserted widgets parent is the window root widget and
        /// it is affected by the context properties set on the window only.
        ///
        /// # Layout & Render
        ///
        /// Layered widgets are measured and arranged using the same constrains as the window root widget, the desired
        /// size is discarded, only the root widget desired size can affect the window size. Layered widgets are all layout
        /// and rendered after the window content and from the bottom layer up to the top-most, this means that the [`WidgetLayoutInfo`]
        /// and [`WidgetRenderInfo`] of normal widgets are always up-to-date when the layered widget is arranged and rendered, so if you
        /// implement custom layouts that align the layered widget with a normal widget using the info values it will always be in sync with
        /// a single layout pass, see [`insert_anchored`] for more details.
        ///
        /// [`WindowContext`]: crate::core::context::WindowContext
        /// [`insert_anchored`]: Self::insert_anchored
        pub struct WindowLayers {
            items: SortedWidgetVecRef,
        }
        impl WindowLayers {
            /// Insert the `widget` in the layer identified by a [`LayerIndex`].
            ///
            /// If the `layer` variable updates the widget is moved to the new layer, if multiple widgets
            /// are inserted in the same layer the later inserts are on top of the previous.
            pub fn insert(ctx: &mut WidgetContext, layer: impl IntoVar<LayerIndex>, widget: impl Widget) {
                struct LayeredWidget<L, W> {
                    layer: L,
                    widget: W,
                }
                #[impl_ui_node(
                    delegate = &self.widget,
                    delegate_mut = &mut self.widget,
                )]
                impl<L: Var<LayerIndex>, W: Widget> UiNode for LayeredWidget<L, W> {
                    fn init(&mut self, ctx: &mut WidgetContext) {
                        self.widget.state_mut().set(LayerIndexKey, self.layer.copy(ctx.vars));
                        self.widget.init(ctx);
                    }

                    fn update(&mut self, ctx: &mut WidgetContext) {
                        if let Some(index) = self.layer.copy_new(ctx) {
                            self.widget.state_mut().set(LayerIndexKey, index);
                            ctx.window_state.req(WindowLayersKey).items.sort(ctx.updates, ctx.path.widget_id());
                        }
                        self.widget.update(ctx);
                    }
                }
                impl<L: Var<LayerIndex>, W: Widget> Widget for LayeredWidget<L, W> {
                    fn id(&self) -> WidgetId {
                        self.widget.id()
                    }

                    fn state(&self) -> &StateMap {
                        self.widget.state()
                    }

                    fn state_mut(&mut self) -> &mut StateMap {
                        self.widget.state_mut()
                    }

                    fn outer_info(&self) -> &WidgetLayoutInfo {
                        self.widget.outer_info()
                    }

                    fn inner_info(&self) -> &WidgetLayoutInfo {
                        self.widget.inner_info()
                    }

                    fn border_info(&self) -> &WidgetBorderInfo {
                        self.widget.border_info()
                    }

                    fn render_info(&self) -> &WidgetRenderInfo {
                        self.widget.render_info()
                    }
                }

                ctx.window_state.req(WindowLayersKey).items.insert(
                    ctx.updates,
                    LayeredWidget {
                        layer: layer.into_var(),
                        widget,
                    },
                );
            }

            /// Insert the `widget` in the layer and *anchor* it to the offset/transform of another widget.
            ///
            /// The `anchor` is the ID of another widget, the inserted `widget` will be offset/transform so that it aligns
            /// with the `anchor` widget top-left. The `mode` is a value of [`AnchorMode`] that defines if the `widget` will
            /// receive the full transform or just the offset.
            ///
            /// If the `anchor` widget is not found the `widget` is not rendered (visibility `Collapsed`).
            pub fn insert_anchored(
                ctx: &mut WidgetContext,
                layer: impl IntoVar<LayerIndex>,
                anchor: impl IntoVar<WidgetId>,
                mode: impl IntoVar<AnchorMode>,

                widget: impl Widget,
            ) {
                struct AnchoredWidget<A, M, W> {
                    anchor: A,
                    mode: M,
                    widget: W,

                    anchor_info: Option<(WidgetLayoutInfo, WidgetLayoutInfo, WidgetBorderInfo, WidgetRenderInfo)>,

                    desired_size: PxSize,
                    interaction: bool,

                    spatial_id: SpatialFrameId,
                    transform_key: FrameBindingKey<RenderTransform>,
                    transform: RenderTransform,
                }
                #[impl_ui_node(
                    delegate = &self.widget,
                    delegate_mut = &mut self.widget,
                )]
                impl<A, M, W> UiNode for AnchoredWidget<A, M, W>
                where
                    A: Var<WidgetId>,
                    M: Var<AnchorMode>,
                    W: Widget,
                {
                    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                        subscriptions.event(WidgetInfoChangedEvent);

                        self.widget.subscriptions(ctx, subscriptions)
                    }

                    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                        if self.interaction {
                            let anchor = self.anchor.copy(ctx);
                            let widget = ctx.path.widget_id();
                            info.push_interaction_filter(move |args| {
                                if args.info.self_and_ancestors().any(|w| w.widget_id() == widget) {
                                    args.info.tree().find(anchor).map(|a| a.allow_interaction()).unwrap_or(false)
                                } else {
                                    true
                                }
                            });
                        }
                        self.widget.info(ctx, info)
                    }

                    fn init(&mut self, ctx: &mut WidgetContext) {
                        if let Some(w) = ctx.info_tree.find(self.anchor.copy(ctx.vars)) {
                            self.anchor_info = Some((w.inner_info(), w.outer_info(), w.border_info(), w.render_info()));
                        }

                        self.interaction = self.mode.get(ctx).interaction;

                        self.widget.init(ctx);
                    }

                    fn deinit(&mut self, ctx: &mut WidgetContext) {
                        self.anchor_info = None;
                        self.widget.deinit(ctx);
                    }

                    fn event<Args: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &Args) {
                        if let Some(args) = WidgetInfoChangedEvent.update(args) {
                            if args.window_id == ctx.path.window_id() {
                                self.anchor_info = ctx
                                    .info_tree
                                    .find(self.anchor.copy(ctx.vars))
                                    .map(|w| (w.inner_info(), w.outer_info(), w.border_info(), w.render_info()));
                            }
                            self.widget.event(ctx, args);
                        } else {
                            self.widget.event(ctx, args);
                        }
                    }

                    fn update(&mut self, ctx: &mut WidgetContext) {
                        if let Some(anchor) = self.anchor.copy_new(ctx) {
                            self.anchor_info = ctx
                                .info_tree
                                .find(anchor)
                                .map(|w| (w.inner_info(), w.outer_info(), w.border_info(), w.render_info()));
                            if self.mode.get(ctx).interaction {
                                ctx.updates.info();
                            }
                            ctx.updates.layout_and_render();
                        }
                        if let Some(mode) = self.mode.get_new(ctx) {
                            if mode.interaction != self.interaction {
                                self.interaction = mode.interaction;
                                ctx.updates.info();
                            }
                            ctx.updates.layout_and_render();
                        }
                        self.widget.update(ctx);
                    }

                    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                        if let Some((inner, outer, border, _)) = &self.anchor_info {
                            let mode = self.mode.get(ctx.vars);

                            if !mode.visibility || inner.size() != PxSize::zero() {
                                let available_size = match mode.size {
                                    AnchorSize::Infinite => AvailableSize::inf(),
                                    AnchorSize::Window => available_size,
                                    AnchorSize::InnerSize => AvailableSize::finite(inner.size()),
                                    AnchorSize::InnerBorder => AvailableSize::finite(border.inner_border_size(inner)),
                                    AnchorSize::OuterSize => AvailableSize::finite(outer.size()),
                                };
                                let desired_size = self.widget.measure(ctx, available_size);
                                if mode.size == AnchorSize::Infinite {
                                    self.desired_size = desired_size;
                                }
                                return desired_size;
                            }
                        }

                        PxSize::zero()
                    }

                    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                        if let Some((inner, outer, border, _)) = &self.anchor_info {
                            let mode = self.mode.get(ctx.vars);

                            if !mode.visibility || inner.size() != PxSize::zero() {
                                // if we don't link visibility or anchor is not collapsed.

                                let final_size = match mode.size {
                                    AnchorSize::Infinite => self.desired_size,
                                    AnchorSize::Window => final_size,
                                    AnchorSize::InnerSize => inner.size(),
                                    AnchorSize::InnerBorder => border.inner_border_size(inner),
                                    AnchorSize::OuterSize => outer.size(),
                                };
                                self.transform = match &mode.transform {
                                    AnchorTransform::None => RenderTransform::identity(),
                                    AnchorTransform::InnerOffset(p) => {
                                        let p = p.to_layout(ctx, AvailableSize::finite(inner.size()), PxPoint::zero());
                                        let offset = inner.point_in_window(p);
                                        RenderTransform::translation_px(offset.to_vector())
                                    }
                                    AnchorTransform::InnerBorderOffset(p) => {
                                        let p = p.to_layout(ctx, AvailableSize::finite(inner.size()), PxPoint::zero());
                                        let offset = border.inner_point_in_window(inner, p);
                                        RenderTransform::translation_px(offset.to_vector())
                                    }
                                    AnchorTransform::OuterOffset(p) => {
                                        let p = p.to_layout(ctx, AvailableSize::finite(outer.size()), PxPoint::zero());
                                        let offset = outer.point_in_window(p);
                                        RenderTransform::translation_px(offset.to_vector())
                                    }
                                    AnchorTransform::InnerTransform => inner.transform(),
                                    AnchorTransform::InnerBorderTransform => border.inner_transform(inner),
                                    AnchorTransform::OuterTransform => outer.transform(),
                                };

                                if mode.corner_radius {
                                    let mut cr = border.corner_radius();
                                    if let AnchorSize::InnerBorder = mode.size {
                                        cr = cr.deflate(border.offsets());
                                    }
                                    widget_layout.with_base_corner_radius(cr, |wl| {
                                        wl.with_custom_transform(&self.transform, |wl| {
                                            self.widget.arrange(ctx, wl, final_size);
                                        });
                                    })
                                } else {
                                    widget_layout.with_custom_transform(&self.transform, |wl| {
                                        self.widget.arrange(ctx, wl, final_size);
                                    });
                                }

                                return;
                            }
                        }

                        widget_layout.collapse(ctx.info_tree);
                    }

                    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                        if let Some((_, _, _, render_info)) = &self.anchor_info {
                            if !self.mode.get(ctx).visibility || render_info.rendered() {
                                frame.push_reference_frame(self.spatial_id, self.transform_key.bind(self.transform), false, |frame| {
                                    self.widget.render(ctx, frame);
                                });
                                return;
                            }
                        }

                        frame.skip_render(ctx.info_tree);
                    }

                    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                        if let Some((_, _, _, render_info)) = &self.anchor_info {
                            if !self.mode.get(ctx).visibility || render_info.rendered() {
                                update.update_transform(self.transform_key.update(self.transform));
                                self.widget.render_update(ctx, update);
                            }
                        }
                    }
                }
                impl<A: Var<WidgetId>, M: Var<AnchorMode>, W: Widget> Widget for AnchoredWidget<A, M, W> {
                    fn id(&self) -> WidgetId {
                        self.widget.id()
                    }

                    fn state(&self) -> &StateMap {
                        self.widget.state()
                    }

                    fn state_mut(&mut self) -> &mut StateMap {
                        self.widget.state_mut()
                    }

                    fn outer_info(&self) -> &WidgetLayoutInfo {
                        self.widget.outer_info()
                    }

                    fn inner_info(&self) -> &WidgetLayoutInfo {
                        self.widget.inner_info()
                    }

                    fn border_info(&self) -> &WidgetBorderInfo {
                        self.widget.border_info()
                    }

                    fn render_info(&self) -> &WidgetRenderInfo {
                        self.widget.render_info()
                    }
                }

                Self::insert(
                    ctx,
                    layer,
                    AnchoredWidget {
                        anchor: anchor.into_var(),
                        mode: mode.into_var(),
                        widget,

                        anchor_info: None,

                        desired_size: PxSize::zero(),
                        interaction: false,
                        transform: RenderTransform::identity(),
                        transform_key: FrameBindingKey::new_unique(),
                        spatial_id: SpatialFrameId::new_unique(),
                    },
                );
            }

            /// Remove the widget from the layers overlay in the next update.
            ///
            /// The `id` must the widget id of a previous inserted widget, nothing happens if the widget is not found.
            pub fn remove(ctx: &mut WidgetContext, id: impl Into<WidgetId>) {
                ctx.window_state.req(WindowLayersKey).items.remove(ctx.updates, id);
            }
        }

        state_key! {
            struct WindowLayersKey: WindowLayers;
            struct LayerIndexKey: LayerIndex;
        }

        /// Wrap around the window outer-most event node to create the layers.
        ///
        /// This node is automatically included in the `window::new_event` constructor.
        pub fn layers(child: impl UiNode) -> impl UiNode {
            struct LayersNode<C> {
                children: C,
                layered: SortedWidgetVecRef,
            }
            #[impl_ui_node(children)]
            impl<C: UiNodeList> UiNode for LayersNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    ctx.window_state.set(
                        WindowLayersKey,
                        WindowLayers {
                            items: self.layered.clone(),
                        },
                    );

                    self.children.init_all(ctx);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    let mut changed = false;

                    self.children.update_all(ctx, &mut changed);

                    if changed {
                        ctx.updates.layout_and_render();
                    }
                }

                fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                    let mut desired_size = PxSize::zero();
                    self.children.measure_all(
                        ctx,
                        |_, _| available_size,
                        |_, args| {
                            if args.index == 0 {
                                desired_size = args.desired_size;
                            }
                        },
                    );
                    desired_size
                }
            }

            let layers_vec = SortedWidgetVec::new(|a, b| {
                let a = a.state().req(LayerIndexKey);
                let b = b.state().req(LayerIndexKey);

                a.cmp(b)
            });
            let layered = layers_vec.reference();

            LayersNode {
                children: nodes![child].chain_nodes(layers_vec),
                layered,
            }
        }

        /// Represents a layer in a window.
        ///
        /// See the [`WindowLayers`] struct for more information.
        #[derive(Default, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
        pub struct LayerIndex(pub u32);
        impl LayerIndex {
            /// The top-most layer.
            ///
            /// Only widgets that are pretending to be a child window should use this layer, including menus,
            /// drop-downs, pop-ups and tool-tips.
            ///
            /// This is the [`u32::MAX`] value.
            pub const TOP_MOST: LayerIndex = LayerIndex(u32::MAX);

            /// The layer for *adorner* display items.
            ///
            /// Adorner widgets are related to another widget but not as a visual part of it, examples of adorners
            /// are resize handles in a widget visual editor, or an interactive help/guide feature.
            ///
            /// This is the [`TOP_MOST - u16::MAX`] value.
            pub const ADORNER: LayerIndex = LayerIndex(Self::TOP_MOST.0 - u16::MAX as u32);

            /// The default layer, just above the normal window content.
            ///
            /// This is the `0` value.
            pub const DEFAULT: LayerIndex = LayerIndex(0);

            /// Compute `self + other` saturating at the [`TOP_MOST`] bound instead of overflowing.
            ///
            /// [`TOP_MOST`]: Self::TOP_MOST
            pub fn saturating_add(self, other: impl Into<LayerIndex>) -> Self {
                Self(self.0.saturating_add(other.into().0))
            }

            /// Compute `self - other` saturating at the [`DEFAULT`] bound instead of overflowing.
            ///
            /// [`DEFAULT`]: Self::DEFAULT
            pub fn saturating_sub(self, other: impl Into<LayerIndex>) -> Self {
                Self(self.0.saturating_sub(other.into().0))
            }
        }
        impl_from_and_into_var! {
            fn from(index: u32) -> LayerIndex {
                LayerIndex(index)
            }
        }
        /// Calls [`LayerIndex::saturating_add`].
        impl<T: Into<Self>> std::ops::Add<T> for LayerIndex {
            type Output = Self;

            fn add(self, rhs: T) -> Self::Output {
                self.saturating_add(rhs)
            }
        }
        /// Calls [`LayerIndex::saturating_sub`].
        impl<T: Into<Self>> std::ops::Sub<T> for LayerIndex {
            type Output = Self;

            fn sub(self, rhs: T) -> Self::Output {
                self.saturating_sub(rhs)
            }
        }
        /// Calls [`LayerIndex::saturating_add`].
        impl<T: Into<Self>> std::ops::AddAssign<T> for LayerIndex {
            fn add_assign(&mut self, rhs: T) {
                *self = *self + rhs;
            }
        }
        /// Calls [`LayerIndex::saturating_sub`].
        impl<T: Into<Self>> std::ops::SubAssign<T> for LayerIndex {
            fn sub_assign(&mut self, rhs: T) {
                *self = *self - rhs;
            }
        }

        /// Options for [`AnchorMode::transform`].
        #[derive(Debug, Clone, PartialEq)]
        pub enum AnchorTransform {
            /// Widget does not copy any position from the anchor widget.
            None,
            /// The point is resolved in the inner space of the anchor widget, transformed to the window space
            /// and then applied as a translate offset.
            InnerOffset(Point),
            /// The point is resolved in the inner space of the anchor widget offset by the anchor border widths, transformed
            /// to the window space and t hen applied as a translate offset.
            InnerBorderOffset(Point),

            /// The point is resolved in the outer space of the anchor widget, transformed to the window space
            /// and then applied as a translate offset.
            OuterOffset(Point),

            /// The full inner transform of the anchor object is applied to the widget.
            InnerTransform,

            /// The full inner transform of the anchor object is applied to the widget plus the border widths offset.
            InnerBorderTransform,

            /// The full outer transform of the anchor object is applied to the widget.
            OuterTransform,
        }
        impl_from_and_into_var! {
            /// `InnerOffset`.
            fn from(inner_offset: Point) -> AnchorTransform {
                AnchorTransform::InnerOffset(inner_offset)
            }
            /// `InnerOffset`.
            fn from<X: Into<Length> + Clone, Y: Into<Length> + Clone>(inner_offset: (X, Y)) -> AnchorTransform {
                Point::from(inner_offset).into()
            }
            /// `InnerOffset`.
            fn from(inner_offset: PxPoint) -> AnchorTransform {
                Point::from(inner_offset).into()
            }
            /// `InnerOffset`.
            fn from(inner_offset: DipPoint) -> AnchorTransform {
                Point::from(inner_offset).into()
            }
        }

        /// Options for [`AnchorMode::size`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum AnchorSize {
            /// Widget does not copy any size from the anchor widget, the available size is infinite, the
            /// final size is the desired size.
            ///
            /// Note that layered widgets do not affect the window size and a widget that overflows the content
            /// boundaries is clipped.
            Infinite,
            /// Widget does not copy any size from the anchor widget, the available size and final size
            /// are the window's root size.
            Window,
            /// The available size and final size is the anchor widget's outer size.
            OuterSize,
            /// The available size and final size is the anchor widget's inner size.
            InnerSize,
            /// The available size and final size is the anchor widget's inner size offset by the border widths.
            InnerBorder,
        }

        /// Defines what properties the layered widget takes from the anchor widget.
        #[derive(Debug, Clone, PartialEq)]
        pub struct AnchorMode {
            /// What transforms are copied from the anchor widget and applied as a *parent* transform of the widget.
            pub transform: AnchorTransform,
            /// What size is copied from the anchor widget and used as the available size and final size of the widget.
            pub size: AnchorSize,
            /// If the widget is only layout if the anchor widget is not [`Collapsed`] and is only rendered
            /// if the anchor widget is rendered.
            ///
            /// [`Collapsed`]: Visibility::Collapsed
            pub visibility: bool,
            /// The widget only allows interaction if the anchor widget [`allow_interaction`].
            ///
            /// [`allow_interaction`]: crate::core::widget_info::WidgetInfo::allow_interaction
            pub interaction: bool,

            /// The widget's corner radius is set for the layer.
            ///
            /// If `size` is [`InnerBorder`] the corner radius are deflated to fit the *inner* curve of the borders.
            ///
            /// [`InnerBorder`]: AnchorSize::InnerBorder
            pub corner_radius: bool,
        }
        impl AnchorMode {
            /// Mode where widget behaves like an unanchored widget, except that it is still only
            /// layout an rendered if the anchor widget exists in the same window.
            pub fn none() -> Self {
                AnchorMode {
                    transform: AnchorTransform::None,
                    size: AnchorSize::Window,
                    visibility: false,
                    interaction: false,
                    corner_radius: false,
                }
            }
        }
        impl Default for AnchorMode {
            /// Transform `InnerOffset` top-left, size infinite, copy visibility and corner-radius.
            fn default() -> Self {
                AnchorMode {
                    transform: AnchorTransform::InnerOffset(Point::top_left()),
                    size: AnchorSize::Infinite,
                    visibility: true,
                    interaction: false,
                    corner_radius: true,
                }
            }
        }
        impl_from_and_into_var! {
            /// Custom transform, all else default.
            fn from(transform: AnchorTransform) -> AnchorMode {
                AnchorMode {
                    transform,
                    ..AnchorMode::default()
                }
            }
            /// Transform `InnerOffset`, all else default.
            fn from(inner_offset: Point) -> AnchorMode {
                AnchorTransform::from(inner_offset).into()
            }
            /// Transform `InnerOffset`, all else default.
            fn from(inner_offset: PxPoint) -> AnchorMode {
                AnchorTransform::from(inner_offset).into()
            }
            /// Transform `InnerOffset`, all else default.
            fn from(inner_offset: DipPoint) -> AnchorMode {
                AnchorTransform::from(inner_offset).into()
            }

            /// Custom transform and size, all else default.
            fn from<T: Into<AnchorTransform> + Clone, S: Into<AnchorSize> + Clone>((transform, size): (T, S)) -> AnchorMode {
                AnchorMode {
                    transform: transform.into(),
                    size: size.into(),
                    ..AnchorMode::default()
                }
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            pub fn layer_index_ops() {
                let idx = LayerIndex::DEFAULT;

                let p1 = idx + 1;
                let m1 = idx - 1;

                let mut idx = idx;

                idx += 1;
                assert_eq!(idx, p1);

                idx -= 2;
                assert_eq!(idx, m1);
            }
        }
    }
}
