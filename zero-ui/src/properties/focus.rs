//! Keyboard focus properties, [`tab_index`](fn@tab_index), [`focusable`](fn@focusable),
//! [`on_focus`](fn@on_focus), [`is_focused`](fn@is_focused) and more.

use crate::core::focus::*;
use crate::prelude::new_property::*;

/// Enables a widget to receive focus.
#[property(context, default(false))]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    struct FocusableNode<C, E> {
        child: C,
        is_focusable: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for FocusableNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.is_focusable.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            widget.meta().entry(FocusInfoKey).or_default().focusable = Some(*self.is_focusable.get(ctx));
            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.is_focusable);
            self.child.subscriptions(ctx, subs);
        }
    }
    FocusableNode {
        child,
        is_focusable: focusable.into_var(),
    }
}

/// Customizes the widget order during TAB navigation.
#[property(context, default(TabIndex::default()))]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {
    struct TabIndexNode<C: UiNode, T: Var<TabIndex>> {
        child: C,
        tab_index: T,
    }
    #[impl_ui_node(child)]
    impl<C, T> UiNode for TabIndexNode<C, T>
    where
        C: UiNode,
        T: Var<TabIndex>,
    {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.tab_index.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            widget.meta().entry(FocusInfoKey).or_default().tab_index = Some(*self.tab_index.get(ctx));
            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.tab_index);
            self.child.subscriptions(ctx, subs);
        }
    }
    TabIndexNode {
        child,
        tab_index: tab_index.into_var(),
    }
}

/// Widget is a focus scope.
#[property(context, default(false))]
pub fn focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: is_scope.into_var(),
        is_alt: false,
    }
}
/// Widget is the ALT focus scope.
///
/// ALT focus scopes are also, `TabIndex::SKIP`, `skip_directional_nav`, `TabNav::Cycle` and `DirectionalNav::Cycle` by default.
#[property(context, default(false))]
pub fn alt_focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: is_scope.into_var(),
        is_alt: true,
    }
}
struct FocusScopeNode<C: UiNode, E: Var<bool>> {
    child: C,
    is_focus_scope: E,
    is_alt: bool,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: Var<bool>> UiNode for FocusScopeNode<C, E> {
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.is_focus_scope.is_new(ctx) {
            ctx.updates.info();
        }
        self.child.update(ctx);
    }

    fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
        let info = widget.meta().entry(FocusInfoKey).or_default();
        info.scope = Some(self.is_focus_scope.copy(ctx));
        if self.is_alt {
            info.alt_scope = true;

            if info.tab_index == None {
                info.tab_index = Some(TabIndex::SKIP);
            }
            if info.tab_nav == None {
                info.tab_nav = Some(TabNav::Cycle);
            }
            if info.directional_nav == None {
                info.directional_nav = Some(DirectionalNav::Cycle);
            }
            if info.skip_directional == None {
                info.skip_directional = Some(true);
            }
        }

        self.child.info(ctx, widget);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        subs.var(ctx, &self.is_focus_scope);
        self.child.subscriptions(ctx, subs);
    }
}

/// Behavior of a focus scope when it receives direct focus.
#[property(context, default(FocusScopeOnFocus::default()))]
pub fn focus_scope_behavior(child: impl UiNode, behavior: impl IntoVar<FocusScopeOnFocus>) -> impl UiNode {
    struct FocusScopeBehaviorNode<C: UiNode, B: Var<FocusScopeOnFocus>> {
        child: C,
        behavior: B,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, B: Var<FocusScopeOnFocus>> UiNode for FocusScopeBehaviorNode<C, B> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.behavior.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            let info = widget.meta().entry(FocusInfoKey).or_default();
            info.on_focus = self.behavior.copy(ctx);
            if info.scope.is_none() {
                info.scope = Some(true);
            }
            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.behavior);
            self.child.subscriptions(ctx, subs);
        }
    }
    FocusScopeBehaviorNode {
        child,
        behavior: behavior.into_var(),
    }
}

/// Tab navigation within this focus scope.
#[property(context, default(TabNav::Continue))]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    struct TabNavNode<C, E> {
        child: C,
        tab_nav: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<TabNav>> UiNode for TabNavNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.tab_nav.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            widget.meta().entry(FocusInfoKey).or_default().tab_nav = Some(self.tab_nav.copy(ctx));
            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.tab_nav);
            self.child.subscriptions(ctx, subs);
        }
    }
    TabNavNode {
        child,
        tab_nav: tab_nav.into_var(),
    }
}

/// Arrows navigation within this focus scope.
#[property(context, default(DirectionalNav::Continue))]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    struct DirectionalNavNode<C, E> {
        child: C,
        directional_nav: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<DirectionalNav>> UiNode for DirectionalNavNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.directional_nav.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            widget.meta().entry(FocusInfoKey).or_default().directional_nav = Some(self.directional_nav.copy(ctx));
            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.directional_nav);
            self.child.subscriptions(ctx, subs);
        }
    }
    DirectionalNavNode {
        child,
        directional_nav: directional_nav.into_var(),
    }
}

/// Keyboard shortcuts that focus this widget or its first focusable descendant or its first focusable parent.
///
/// When any of the `shortcuts` is pressed, does a [`focus_widget_or_related`](Focus::focus_widget_or_related)
/// request using the current widget ID and with highlight.
#[property(context, default(Shortcuts::default()))]
pub fn focus_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    struct FocusShortcutNode<C, S> {
        child: C,
        shortcuts: S,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, S: Var<Shortcuts>> UiNode for FocusShortcutNode<C, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(ShortcutEvent);
            self.child.subscriptions(ctx, subs);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = ShortcutEvent.update(args) {
                self.child.event(ctx, args);
                if !args.stop_propagation_requested() && self.shortcuts.get(ctx).contains(args.shortcut) {
                    // focus on shortcut
                    ctx.services.focus().focus_widget_or_related(ctx.path.widget_id(), true);
                    args.stop_propagation();
                }
            } else {
                self.child.event(ctx, args);
            }
        }
    }
    FocusShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
    }
}

/// If directional navigation from outside this widget skips over it and its descendants.
///
/// Setting this to `true` is the directional navigation equivalent of setting `tab_index` to `SKIP`.
#[property(context, default(false))]
pub fn skip_directional(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct SkipDirectionalNode<C: UiNode, E: Var<bool>> {
        child: C,
        enabled: E,
    }
    #[impl_ui_node(child)]
    impl<C, E> UiNode for SkipDirectionalNode<C, E>
    where
        C: UiNode,
        E: Var<bool>,
    {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.enabled.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            widget.meta().entry(FocusInfoKey).or_default().skip_directional = Some(self.enabled.copy(ctx));

            self.child.info(ctx, widget);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.enabled);
            self.child.subscriptions(ctx, subs);
        }
    }
    SkipDirectionalNode {
        child,
        enabled: enabled.into_var(),
    }
}

event_property! {
    /// Focus changed in the widget or its descendants.
    pub fn focus_changed {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
    }

    /// Widget got direct keyboard focus.
    pub fn focus {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus(ctx.path.widget_id()),
    }

    /// Widget lost direct keyboard focus.
    pub fn blur {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_blur(ctx.path.widget_id()),
    }

    /// Widget or one of its descendants got focus.
    pub fn focus_enter {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus_enter(ctx.path.widget_id())
    }

    /// Widget or one of its descendants lost focus.
    pub fn focus_leave {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus_leave(ctx.path.widget_id())
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// You can use [`is_focus_within`] to include focused widgets inside this one.
///
/// # Highlighting
///
/// This property is always `true` when the widget has focus, ignoring what device was used to move the focus,
/// usually when the keyboard is used a special visual indicator is rendered, a dotted line border is common,
/// this state is called *highlighting* and is tracked by the focus manager. To implement such a visual you can use the
/// [`is_focused_hgl`] property.
///
/// # Return Focus
///
/// Usually widgets that have a visual state for this property also have one for [`is_return_focus`], a common example is the
/// *text-input* or *text-box* widget that shows an emphasized border and blinking cursor when focused and still shows the
/// emphasized border without cursor when a menu is open and it is only the return focus.
///
/// [`is_focus_within`]: fn@zero_ui::properties::focus::is_focus_within
/// [`is_focused_hgl`]: fn@zero_ui::properties::focus::is_focused_hgl
/// [`is_return_focus`]: fn@zero_ui::properties::focus::is_return_focus
#[property(context)]
pub fn is_focused(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, FocusChangedEvent, |ctx, args| {
        if args.is_focus(ctx.path.widget_id()) {
            Some(true)
        } else if args.is_blur(ctx.path.widget_id()) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`].
///
/// To track *highlighted* focus within use [`is_focus_within_hgl`] property.
///
/// [`is_focused`]: fn@zero_ui::properties::focus::is_focused
/// [`is_focus_within_hgl`]: fn@zero_ui::properties::focus::is_focus_within_hgl
#[property(context)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, FocusChangedEvent, |ctx, args| {
        if args.is_focus_enter(ctx.path.widget_id()) {
            Some(true)
        } else if args.is_focus_leave(ctx.path.widget_id()) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget has keyboard focus and the user is using the keyboard to navigate.
///
/// This is only `true` if the widget itself is focused and the focus was acquired by keyboard navigation.
/// You can use [`is_focus_within_hgl`] to include widgets inside this one.
///
/// # Highlighting
///
/// Usually when the keyboard is used to move the focus a special visual indicator is rendered, a dotted line border is common,
/// this state is called *highlighting* and is tracked by the focus manager, this property is only `true`.
///
/// [`is_focus_within_hgl`]: fn@zero_ui::properties::focus::is_focus_within_hgl
/// [`is_focused`]: fn@zero_ui::properties::focus::is_focused
#[property(context)]
pub fn is_focused_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, FocusChangedEvent, |ctx, args| {
        if args.is_focus(ctx.path.widget_id()) {
            Some(args.highlight)
        } else if args.is_blur(ctx.path.widget_id()) {
            Some(false)
        } else if args.is_hightlight_changed()
            && args
                .new_focus
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or(false)
        {
            Some(args.highlight)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants has keyboard focus and the user is using the keyboard to navigate.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`].
///
/// Also see [`is_focus_within`] to check if the widget has focus within regardless of highlighting.
///
/// [`is_focused_hgl`]: fn@zero_ui::properties::focus::is_focused_hgl
/// [`is_focus_within`]: fn@zero_ui::properties::focus::is_focus_within
#[property(context)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, FocusChangedEvent, |ctx, args| {
        if args.is_focus_enter(ctx.path.widget_id()) {
            Some(args.highlight)
        } else if args.is_focus_leave(ctx.path.widget_id()) {
            Some(false)
        } else if args.is_hightlight_changed() && args.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or(false) {
            Some(args.highlight)
        } else {
            None
        }
    })
}

/// If the widget will be focused when a parent scope is focused.
///
/// Focus scopes can be configured to remember the last focused widget inside then, the focus than *returns* to
/// this widget when the scope receives focus. Alt scopes also remember the widget from which the *alt* focus happened
/// and can also return focus back to that widget.
///
/// Usually input widgets that have a visual state for [`is_focused`] also have a visual for this, a common example is the
/// *text-input* or *text-box* widget that shows an emphasized border and blinking cursor when focused and still shows the
/// emphasized border without cursor when a menu is open and it is only the return focus.
///
/// Note that a widget can be [`is_focused`] and `is_return_focus`, this property is `true` if any focus scope considers the
/// widget its return focus, you probably want to declare the widget visual states in such a order that [`is_focused`] overrides
/// the state of this property.
///
/// [`is_focused`]: fn@zero_ui::properties::focus::is_focused_hgl
/// [`is_focused_hgl`]: fn@zero_ui::properties::focus::is_focused_hgl
#[property(context)]
pub fn is_return_focus(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_state(child, state, false, ReturnFocusChangedEvent, |ctx, args| {
        if args.is_return_focus(ctx.path.widget_id()) {
            Some(true)
        } else if args.was_return_focus(ctx.path.widget_id()) {
            Some(false)
        } else {
            None
        }
    })
}
