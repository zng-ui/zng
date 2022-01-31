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

    /// Setup layers.
    pub fn new_outer(child: impl UiNode) -> impl UiNode {
        nodes::layers(child)
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
    pub use nodes::{AnchorMode, LayerIndex, WindowLayers};

    /// UI nodes used for building a window widget.
    pub mod nodes {
        use crate::prelude::new_property::*;

        /// Windows layers.
        ///
        /// TODO describe (no window context except root_id and WindowVars, etc.).
        ///
        /// [`WindowContext`]:crate::core::context::WindowContext
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

                    fn outer_bounds(&self) -> &BoundsInfo {
                        self.widget.outer_bounds()
                    }

                    fn inner_bounds(&self) -> &BoundsInfo {
                        self.widget.inner_bounds()
                    }

                    fn visibility(&self) -> Visibility {
                        self.widget.visibility()
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
                }
                #[impl_ui_node(
                    delegate = &self.widget,
                    delegate_mut = &mut self.widget,
                )]
                impl<A: Var<WidgetId>, M: Var<AnchorMode>, W: Widget> UiNode for AnchoredWidget<A, M, W> {
                    fn update(&mut self, ctx: &mut WidgetContext) {
                        if let Some(anchor) = self.anchor.copy_new(ctx) {
                            ctx.updates.layout_and_render();
                        }
                        if self.mode.is_new(ctx) {
                            ctx.updates.layout_and_render();
                        }
                        self.widget.update(ctx);
                    }

                    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                        self.widget.measure(ctx, available_size)
                    }

                    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                        self.widget.arrange(ctx, widget_layout, final_size);
                    }

                    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                        self.widget.render(ctx, frame);
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

                    fn outer_bounds(&self) -> &BoundsInfo {
                        self.widget.outer_bounds()
                    }

                    fn inner_bounds(&self) -> &BoundsInfo {
                        self.widget.inner_bounds()
                    }

                    fn visibility(&self) -> Visibility {
                        self.widget.visibility()
                    }
                }

                Self::insert(
                    ctx,
                    layer,
                    AnchoredWidget {
                        anchor: anchor.into_var(),
                        mode: mode.into_var(),
                        widget,
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

        /// Wrap around the window outer-most context node to create the layers.
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

        bitflags::bitflags! {
            /// Defines how the layered widget is anchored to another widget.
            pub struct AnchorMode: u16 {
                /// The widget is layout as if it is the only content of the window, the available size is the window content area.
                const DEFAULT = 0b0;

                /// The other widget stacked transform is applied to the widget origin point, so it is not scaled and
                /// rotated like the other widget but it is positioned at the transform point, the available size is *infinite*.
                const OFFSET = 0b1;

                /// The other widget stacked transform is applied the widget, this flag overrides [`OFFSET`], the available size is *infinite*.
                ///
                /// [`OFFSET`]: LayerMode::OFFSET
                const TRANSFORM = 0b11;
            }
        }
        impl Default for AnchorMode {
            fn default() -> Self {
                AnchorMode::DEFAULT
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
