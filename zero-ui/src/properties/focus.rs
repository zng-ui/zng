//! Keyboard focus properties, [`tab_index`](fn@tab_index), [`focusable`](fn@focusable),
//! [`on_focus`](fn@on_focus), [`is_focused`](fn@is_focused) and more.

use crate::core::focus::*;
use crate::prelude::new_property::*;

/// Enables a widget to receive focus.
#[property(CONTEXT, default(false))]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct FocusableNode {
        child: impl UiNode,
        #[var] focusable: impl Var<bool>,
    })]
    impl UiNode for FocusableNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.focusable.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            FocusInfoBuilder::get(info).focusable(self.focusable.get());
            self.child.info(ctx, info);
        }
    }
    FocusableNode {
        child,
        focusable: focusable.into_var(),
    }
}

/// Customizes the widget order during TAB navigation.
#[property(CONTEXT, default(TabIndex::default()))]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {
    #[ui_node(struct TabIndexNode {
        child: impl UiNode,
        #[var] tab_index: impl Var<TabIndex>,
    })]
    impl UiNode for TabIndexNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.tab_index.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            FocusInfoBuilder::get(widget).tab_index(self.tab_index.get());
            self.child.info(ctx, widget);
        }
    }
    TabIndexNode {
        child,
        tab_index: tab_index.into_var(),
    }
}

/// Widget is a focus scope.
#[property(CONTEXT, default(false))]
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
#[property(CONTEXT, default(false))]
pub fn alt_focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: is_scope.into_var(),
        is_alt: true,
    }
}

#[ui_node(struct FocusScopeNode {
    child: impl UiNode,
    #[var] is_focus_scope: impl Var<bool>,
    is_alt: bool,
})]
impl UiNode for FocusScopeNode {
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        if self.is_focus_scope.is_new(ctx) {
            ctx.updates.info();
        }
        self.child.update(ctx, updates);
    }

    fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
        let mut info = FocusInfoBuilder::get(widget);
        if self.is_alt {
            info.alt_scope(self.is_focus_scope.get());
        } else {
            info.scope(self.is_focus_scope.get());
        }

        self.child.info(ctx, widget);
    }
}

/// Behavior of a focus scope when it receives direct focus.
#[property(CONTEXT, default(FocusScopeOnFocus::default()))]
pub fn focus_scope_behavior(child: impl UiNode, behavior: impl IntoVar<FocusScopeOnFocus>) -> impl UiNode {
    #[ui_node(struct FocusScopeBehaviorNode {
        child: impl UiNode,
        #[var] behavior: impl Var<FocusScopeOnFocus>,
    })]
    impl UiNode for FocusScopeBehaviorNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.behavior.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            let mut info = FocusInfoBuilder::get(widget);
            info.on_focus(self.behavior.get());
            self.child.info(ctx, widget);
        }
    }
    FocusScopeBehaviorNode {
        child,
        behavior: behavior.into_var(),
    }
}

/// Tab navigation within this focus scope.
#[property(CONTEXT, default(TabNav::Continue))]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    #[ui_node(struct TabNavNode {
        child: impl UiNode,
        #[var] tab_nav: impl Var<TabNav>,
    })]
    impl UiNode for TabNavNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.tab_nav.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            FocusInfoBuilder::get(widget).tab_nav(self.tab_nav.get());
            self.child.info(ctx, widget);
        }
    }
    TabNavNode {
        child,
        tab_nav: tab_nav.into_var(),
    }
}

/// Arrows navigation within this focus scope.
#[property(CONTEXT, default(DirectionalNav::Continue))]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    #[ui_node(struct DirectionalNavNode {
        child: impl UiNode,
        #[var] directional_nav: impl Var<DirectionalNav>,
    })]
    impl UiNode for DirectionalNavNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.directional_nav.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            FocusInfoBuilder::get(widget).directional_nav(self.directional_nav.get());
            self.child.info(ctx, widget);
        }
    }
    DirectionalNavNode {
        child,
        directional_nav: directional_nav.into_var(),
    }
}

/// Keyboard shortcuts that focus this widget or its first focusable descendant or its first focusable parent.
#[property(CONTEXT, default(Shortcuts::default()))]
pub fn focus_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    #[ui_node(struct FocusShortcutNode {
        child: impl UiNode,
        #[var] shortcuts: impl Var<Shortcuts>,
        handle: Option<ShortcutsHandle>,
    })]
    impl UiNode for FocusShortcutNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);
            self.child.init(ctx);
            let s = self.shortcuts.get();
            self.handle = Some(Gestures::req(ctx.services).focus_shortcut(s, ctx.path.widget_id()));
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if let Some(s) = self.shortcuts.get_new(ctx) {
                self.handle = Some(Gestures::req(ctx.services).focus_shortcut(s, ctx.path.widget_id()));
            }
        }
    }
    FocusShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        handle: None,
    }
}

/// If directional navigation from outside this widget skips over it and its descendants.
///
/// Setting this to `true` is the directional navigation equivalent of setting `tab_index` to `SKIP`.
#[property(CONTEXT, default(false))]
pub fn skip_directional(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct SkipDirectionalNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,
    })]
    impl UiNode for SkipDirectionalNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.enabled.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }

        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            FocusInfoBuilder::get(widget).skip_directional(self.enabled.get());

            self.child.info(ctx, widget);
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
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
    }

    /// Widget got direct keyboard focus.
    pub fn focus {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus(ctx.path.widget_id()),
    }

    /// Widget lost direct keyboard focus.
    pub fn blur {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_blur(ctx.path.widget_id()),
    }

    /// Widget or one of its descendants got focus.
    pub fn focus_enter {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus_enter(ctx.path.widget_id())
    }

    /// Widget or one of its descendants lost focus.
    pub fn focus_leave {
        event: FOCUS_CHANGED_EVENT,
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
#[property(CONTEXT)]
pub fn is_focused(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |ctx, args| {
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
#[property(CONTEXT)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |ctx, args| {
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
#[property(CONTEXT)]
pub fn is_focused_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |ctx, args| {
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
#[property(CONTEXT)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |ctx, args| {
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
#[property(CONTEXT)]
pub fn is_return_focus(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_is_state(child, state, false, RETURN_FOCUS_CHANGED_EVENT, |ctx, args| {
        if args.is_return_focus(ctx.path.widget_id()) {
            Some(true)
        } else if args.was_return_focus(ctx.path.widget_id()) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants will be focused when a focus scope is focused.
///
/// To check if only the widget is the return focus use [`is_return_focus`].
///
/// [`is_return_focus`]: fn@zero_ui::properties::focus::is_return_focus
#[property(CONTEXT)]
pub fn is_return_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    event_is_state(child, state, false, RETURN_FOCUS_CHANGED_EVENT, |ctx, args| {
        if args.is_return_focus_enter(ctx.path.widget_id()) {
            Some(true)
        } else if args.is_return_focus_leave(ctx.path.widget_id()) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget is focused on init.
///
/// When the widget is inited a [`Focus::focus_widget_or_related`] request is made for the widget.
#[property(CONTEXT, default(false))]
pub fn focus_on_init(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node( struct FocusOnInitNode {
        child: impl UiNode,
        enabled: impl Var<bool>,
    })]
    impl UiNode for FocusOnInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            if self.enabled.get() {
                Focus::req(ctx.services).focus_widget_or_related(ctx.path.widget_id(), false);
            }
        }
    }
    FocusOnInitNode {
        child: child.cfg_boxed(),
        enabled: enabled.into_var(),
    }
    .cfg_boxed()
}
