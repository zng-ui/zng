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

    pub use super::properties::{self, IsCheckedVar};

    inherit!(crate::widgets::button);

    use crate::widgets::button::theme;

    properties! {
        remove { on_click }

        /// Toggle cycles between `true` and `false`, updating the variable.
        properties::checked;

        /// Toggle cycles between `Some(true)` and `Some(false)` and accepts `None`, if the
        /// widget is `three_state` also sets to `None` in the toggle cycle.
        properties::checked_opt;

        /// Enables `None` as an input value.
        ///
        /// Note that the `None` value is always accepted in `checked_opt`, this property controls if
        /// `None` is one of the values in the toggle cycle. If the widget is bound to the `checked` property
        /// this config is ignored.
        ///
        /// This is not enabled by default.
        properties::three_state = properties::IsThreeStateVar;

        properties::is_checked;

        /// When toggle is `Some(true)`.
        when self.is_checked {
            background_color = theme::hovered::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::hovered::BorderSidesVar,
            };
            text_color = theme::hovered::TextColorVar;
        }
    }

    fn new_context(child: impl UiNode, three_state: impl IntoVar<bool>) -> impl UiNode {
        // ensure that the context var is set for other contexts.
        properties::three_state(child, three_state)
    }
}

/// Properties used in the toggle widget.
pub mod properties {
    use crate::prelude::new_property::*;

    context_var! {
        /// The toggle button checked state.
        pub struct IsCheckedVar: Option<bool> = Some(false);

        /// If toggle button cycles between `None`, `Some(false)` and `Some(true)` on click.
        pub struct IsThreeStateVar: bool = false;
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
    /// Sets to `None` if [`IsThreeStateVar`] is `true`.
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

                        if *IsThreeStateVar::get(ctx) {
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
    #[property(context, default(IsThreeStateVar))]
    pub fn three_state(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, IsThreeStateVar, enabled)
    }

    /// If [`IsCheckedVar`] is `Some(true)`.
    #[property(event)]
    pub fn is_checked(child: impl UiNode, state: StateVar) -> impl UiNode {
        bind_state(child, IsCheckedVar::new().map(|s| *s == Some(true)), state)
    }
}

/// A checkbox toggle.
#[widget($crate::widgets::checkbox)]
pub mod checkbox {
    inherit!(super::toggle);

    pub use super::toggle::IsCheckedVar;

    use super::*;

    properties! {
        remove { background_color; border }

        content_align = Align::LEFT;
        padding = 0;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }
}
