use crate::prelude::new_widget::*;

/// A toggle button that flips a `bool` or `Option<bool>` variable on click.
///
/// This widget has two primary properties, [`checked`] and [`checked_opt`], setting one
/// of these properties to a read-write variable enables the widget and it will set the variables
/// on click.
///
/// [`checked`]: #wp-checked
/// [`checked_opt`]: #wp-checked_opt
#[widget($crate::widgets::toggle)]
pub mod toggle {
    use super::*;

    #[doc(inline)]
    pub use super::properties::{self, IsCheckedVar};
    #[doc(inline)]
    pub use super::vis;

    inherit!(crate::widgets::button);

    properties! {
        remove { on_click }

        /// Toggle cycles between `true` and `false`, updating the variable.
        properties::checked;

        /// Toggle cycles between `Some(true)` and `Some(false)` and accepts `None`, if the
        /// widget is `tristate` also sets to `None` in the toggle cycle.
        properties::checked_opt;

        /// Enables `None` as an input value.
        ///
        /// Note that `None` is always accepted in `checked_opt`, this property controls if
        /// `None` is one of the values in the toggle cycle. If the widget is bound to the `checked` property
        /// this config is ignored.
        ///
        /// This is not enabled by default.
        properties::tristate = properties::IsTristateVar;

        properties::is_checked;

        /// Toggle dark and light themes.
        ///
        /// Set to [`theme::pair`] of [`vis::DarkThemeVar`], [`vis::LightThemeVar`] by default.
        theme = theme::pair(vis::DarkThemeVar, vis::LightThemeVar);
    }

    fn new_context_dyn(child: impl UiNode, properties: Vec<DynProperty>, tristate: impl IntoVar<bool>) -> impl UiNode {
        // ensure that the context var is set for other contexts.
        let child = properties::tristate(child, tristate);
        themable::new_context_dyn(child, properties)
    }
}

/// Properties used in the toggle widget.
pub mod properties {
    use crate::prelude::new_property::*;

    context_var! {
        /// The toggle button checked state.
        pub struct IsCheckedVar: Option<bool> = Some(false);

        /// If toggle button cycles between `None`, `Some(false)` and `Some(true)` on click.
        pub struct IsTristateVar: bool = false;
    }

    /// Toggle `checked` on click and sets the [`IsCheckedVar`], disables the widget if `checked` is read-only.
    #[property(context, default(false))]
    pub fn checked(child: impl UiNode, checked: impl IntoVar<bool>) -> impl UiNode {
        struct CheckedNode<C, B> {
            child: C,
            checked: B,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, B: Var<bool>> UiNode for CheckedNode<C, B> {
            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                subs.event(ClickEvent);
                self.child.subscriptions(ctx, subs);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some(args) = ClickEvent.update(args) {
                    self.child.event(ctx, args);

                    if args.is_primary()
                        && !self.checked.is_read_only(ctx)
                        && !args.propagation().is_stopped()
                        && args.is_enabled(ctx.path.widget_id())
                    {
                        args.propagation().stop();

                        let _ = self.checked.modify(ctx, |mut c| *c = !*c);
                    }
                } else {
                    self.child.event(ctx, args)
                }
            }
        }

        let checked = checked.into_var();
        let node = CheckedNode {
            child: child.cfg_boxed(),
            checked: checked.clone(),
        }
        .cfg_boxed();
        with_context_var(node, IsCheckedVar, checked.map_into())
    }

    /// Three state toggle `checked` on click and sets the [`IsCheckedVar`], disables the widget if `checked` is read-only.
    ///
    /// Sets to `None` if [`IsTristateVar`] is `true`.
    #[property(context, default(None))]
    pub fn checked_opt(child: impl UiNode, checked: impl IntoVar<Option<bool>>) -> impl UiNode {
        struct CheckedOptNode<C, B> {
            child: C,
            checked: B,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, B: Var<Option<bool>>> UiNode for CheckedOptNode<C, B> {
            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                subs.event(ClickEvent);
                self.child.subscriptions(ctx, subs);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some(args) = ClickEvent.update(args) {
                    self.child.event(ctx, args);

                    if args.is_primary()
                        && !self.checked.is_read_only(ctx)
                        && !args.propagation().is_stopped()
                        && args.is_enabled(ctx.path.widget_id())
                    {
                        args.propagation().stop();

                        if *IsTristateVar::get(ctx) {
                            let _ = self.checked.modify(ctx, |mut c| {
                                *c = match *c {
                                    Some(true) => None,
                                    Some(false) => Some(true),
                                    None => Some(false),
                                }
                            });
                        } else {
                            let _ = self.checked.modify(ctx, |mut c| {
                                *c = match *c {
                                    Some(true) | None => Some(false),
                                    Some(false) => Some(true),
                                }
                            });
                        }
                    }
                } else {
                    self.child.event(ctx, args)
                }
            }
        }

        let checked = checked.into_var();
        let node = CheckedOptNode {
            child: child.cfg_boxed(),
            checked: checked.clone(),
        }
        .cfg_boxed();

        with_context_var(node, IsCheckedVar, checked)
    }

    /// Enables `None` as an input on toggle.
    ///
    /// If the toggle button is checking using [`checked_opt`] and this is enabled the toggle cycles between `None`, `Some(false)` and `Some(true)`.
    #[property(context, default(IsTristateVar))]
    pub fn tristate(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, IsTristateVar, enabled)
    }

    /// If [`IsCheckedVar`] is `Some(true)`.
    #[property(event)]
    pub fn is_checked(child: impl UiNode, state: StateVar) -> impl UiNode {
        bind_state(child, IsCheckedVar::new().map(|s| *s == Some(true)), state)
    }
}

/// Toggle themes, visual properties and context vars.
pub mod vis {
    use super::*;

    /// Default toggle dark theme.
    #[widget($crate::widgets::toggle::vis::dark_theme)]
    pub mod dark_theme {
        use super::*;

        inherit!(crate::widgets::button::vis::dark_theme);

        properties! {
            properties::is_checked;

            /// When the toggle is checked.
            when self.is_checked  {
                background_color = crate::widgets::button::vis::DarkBaseColorVar::new().map(|c| c.lighten(60.pct()));
                border = {
                    widths: 1,
                    sides: crate::widgets::button::vis::DarkBaseColorVar::new().map(|c| c.lighten(60.pct()).into()),
                };
            }
        }
    }

    /// Default toggle light theme.
    #[widget($crate::widgets::toggle::vis::light_theme)]
    pub mod light_theme {
        use super::*;

        inherit!(crate::widgets::button::vis::light_theme);

        properties! {
            properties::is_checked;

            /// When the toggle is checked.
            when self.is_checked  {
                background_color = crate::widgets::button::vis::DarkBaseColorVar::new().map(|c| c.lighten(60.pct()));
                border = {
                    widths: 1,
                    sides: crate::widgets::button::vis::DarkBaseColorVar::new().map(|c| c.lighten(60.pct()).into()),
                };
            }
        }
    }

    context_var! {
        /// Toggle dark theme.
        ///
        /// Use the [`toggle::vis::dark`] property to set.
        ///
        /// [`toggle::vis::dark`]: fn@dark
        pub struct DarkThemeVar: ThemeGenerator = ThemeGenerator::new(|_, _| dark_theme!());

        /// Toggle light theme.
        ///
        /// Use the [`toggle::vis::light`] property to set.
        ///
        /// [`toggle::vis::light`]: fn@light
        pub struct LightThemeVar: ThemeGenerator = ThemeGenerator::new(|_, _| light_theme!());
    }

    /// Sets the [`DarkThemeVar`] that affects all toggle buttons inside the widget.
    #[property(context, default(DarkThemeVar))]
    pub fn dark(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, DarkThemeVar, theme)
    }

    /// Sets the [`LightThemeVar`] that affects all toggle buttons inside the widget.
    #[property(context, default(LightThemeVar))]
    pub fn light(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, LightThemeVar, theme)
    }
}

/// A checkbox toggle.
#[widget($crate::widgets::checkbox)]
pub mod checkbox {
    inherit!(super::toggle);

    pub use super::toggle::IsCheckedVar;

    use super::*;

    properties! {
        content_align = Align::LEFT;
        padding = 0;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }
}
