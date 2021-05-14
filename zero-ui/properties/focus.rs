//! Keyboard focus properties, [`tab_index`](fn@tab_index), [`focusable`](fn@focusable),
//! [`on_focus`](fn@on_focus), [`is_focused`](fn@is_focused) and more.

use crate::core::event::EventListener;
use crate::core::focus::*;
use crate::prelude::new_property::*;

/// Enables a widget to receive focus.
#[property(context)]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    struct FocusableNode<C, E> {
        child: C,
        is_focusable: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for FocusableNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.is_focusable.is_new(ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.meta().entry::<FocusInfoKey>().or_default().focusable = Some(*self.is_focusable.get(ctx.vars));
            self.child.render(ctx, frame);
        }
    }
    FocusableNode {
        child,
        is_focusable: focusable.into_var(),
    }
}

/// Customizes the widget order during TAB navigation.
#[property(context)]
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
            if self.tab_index.is_new(ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.meta().entry::<FocusInfoKey>().or_default().tab_index = Some(*self.tab_index.get(ctx.vars));
            self.child.render(ctx, frame);
        }
    }
    TabIndexNode {
        child,
        tab_index: tab_index.into_var(),
    }
}

/// Widget is a focus scope.
#[property(context)]
pub fn focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: is_scope.into_var(),
        is_alt: false,
    }
}
// Widget is the ALT focus scope.
///
/// ALT focus scopes are also, `TabIndex::SKIP`, `skip_directional_nav`, `TabNav::Cycle` and `DirectionalNav::Cycle` by default.
#[property(context)]
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
        if self.is_focus_scope.is_new(ctx.vars) {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let info = frame.meta().entry::<FocusInfoKey>().or_default();
        info.scope = Some(*self.is_focus_scope.get(ctx.vars));
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
        self.child.render(ctx, frame);
    }
}

/// Behavior of a focus scope when it receives direct focus.
#[property(context)]
pub fn focus_scope_behavior(child: impl UiNode, behavior: impl IntoVar<FocusScopeOnFocus>) -> impl UiNode {
    struct FocusScopeBehaviorNode<C: UiNode, B: Var<FocusScopeOnFocus>> {
        child: C,
        behavior: B,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, B: Var<FocusScopeOnFocus>> UiNode for FocusScopeBehaviorNode<C, B> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.behavior.is_new(ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let info = frame.meta().entry::<FocusInfoKey>().or_default();
            info.on_focus = *self.behavior.get(ctx.vars);
            if info.scope.is_none() {
                info.scope = Some(true);
            }
            self.child.render(ctx, frame);
        }
    }
    FocusScopeBehaviorNode {
        child,
        behavior: behavior.into_var(),
    }
}

/// Tab navigation within this focus scope.
#[property(context)]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    struct TabNavNode<C, E> {
        child: C,
        tab_nav: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<TabNav>> UiNode for TabNavNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.tab_nav.is_new(ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.meta().entry::<FocusInfoKey>().or_default().tab_nav = Some(*self.tab_nav.get(ctx.vars));
            self.child.render(ctx, frame);
        }
    }
    TabNavNode {
        child,
        tab_nav: tab_nav.into_var(),
    }
}

/// Arrows navigation within this focus scope.
#[property(context)]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    struct DirectionalNavNode<C, E> {
        child: C,
        directional_nav: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<DirectionalNav>> UiNode for DirectionalNavNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.directional_nav.is_new(ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.meta().entry::<FocusInfoKey>().or_default().directional_nav = Some(*self.directional_nav.get(ctx.vars));
            self.child.render(ctx, frame);
        }
    }
    DirectionalNavNode {
        child,
        directional_nav: directional_nav.into_var(),
    }
}

/// Keyboard shortcuts that focus this widget.
///
/// When any of the `shortcuts` is pressed, focus this widget the parent focusable widget.
///
/// This property sets [`focusable`] to `true` if it is not set on the widget.
#[property(context)]
pub fn focus_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    struct FocusShortcutNode<C: UiNode, S: Var<Shortcuts>> {
        child: C,
        shortcuts: S,
        shortcut_listener: EventListener<ShortcutArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, S: Var<Shortcuts>> UiNode for FocusShortcutNode<C, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.shortcut_listener = ctx.events.listen::<ShortcutEvent>();
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            let shortcuts = self.shortcuts.get(ctx.vars);

            for args in self.shortcut_listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && shortcuts.0.contains(&args.shortcut) {
                    // focus on shortcut
                    ctx.services.req::<Focus>().focus_widget_or_parent(ctx.path.widget_id(), true);

                    args.stop_propagation();
                    break;
                }
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            let focus = frame.meta().entry::<FocusInfoKey>().or_default();
            if focus.focusable.is_none() {
                focus.focusable = Some(true);
            }
        }
    }
    FocusShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        shortcut_listener: ShortcutEvent::never(),
    }
}

/// If directional navigation from outside this widget skips over it and its descendants.
///
/// Setting this to `true` is the directional navigation equivalent of setting `tab_index` to `SKIP`.
#[property(context)]
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
            if self.enabled.is_new(ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.meta().entry::<FocusInfoKey>().or_default().skip_directional = Some(*self.enabled.get(ctx.vars));
            self.child.render(ctx, frame);
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
/// You can use [`is_focus_within`](fn@zero_ui::properties::focus::is_focus_within) to check if the focused widget is within this one.
///
/// # Highlighting
///
/// TODO
#[property(context)]
pub fn is_focused(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsFocusedNode<C: UiNode> {
        child: C,
        state: StateVar,
        focus_changed: EventListener<FocusChangedArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsFocusedNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(u) = self.focus_changed.updates(ctx.events).last() {
                let was_focused = *self.state.get(ctx.vars);
                let is_focused = u
                    .new_focus
                    .as_ref()
                    .map(|p| p.widget_id() == ctx.path.widget_id())
                    .unwrap_or_default();
                if was_focused != is_focused {
                    self.state.set(ctx.vars, is_focused);
                }
            }
            self.child.update(ctx);
        }
    }
    IsFocusedNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`](fn@zero_ui::properties::focus::is_focused).
#[property(context)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsFocusWithinNode<C: UiNode> {
        child: C,
        state: StateVar,
        focus_changed: EventListener<FocusChangedArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsFocusWithinNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(u) = self.focus_changed.updates(ctx.events).last() {
                let was_focused = *self.state.get(ctx.vars);
                let is_focused = u.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or_default();

                if was_focused != is_focused {
                    self.state.set(ctx.vars, is_focused);
                }
            }
            self.child.update(ctx);
        }
    }
    IsFocusWithinNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget has keyboard focus and focus highlighting is enabled.
///
/// This is only `true` if the widget itself is focused and focus highlighting is enabled.
/// You can use [`is_focus_within_hgl`](fn@zero_ui::properties::focus::is_focus_within_hgl) to check if the focused widget is within this one.
///
/// Also see [`is_focused`](fn@zero_ui::properties::focus::is_focused) to check if the widget is focused regardless of highlighting.
#[property(context)]
pub fn is_focused_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsFocusedHglNode<C: UiNode> {
        child: C,
        state: StateVar,
        focus_changed: EventListener<FocusChangedArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsFocusedHglNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(u) = self.focus_changed.updates(ctx.events).last() {
                let was_focused_hgl = *self.state.get(ctx.vars);
                let is_focused_hgl = u.highlight
                    && u.new_focus
                        .as_ref()
                        .map(|p| p.widget_id() == ctx.path.widget_id())
                        .unwrap_or_default();
                if was_focused_hgl != is_focused_hgl {
                    self.state.set(ctx.vars, is_focused_hgl);
                }
            }
            self.child.update(ctx);
        }
    }
    IsFocusedHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget or one of its descendants has keyboard focus and focus highlighting is enabled.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`](fn@zero_ui::properties::focus::is_focused_hgl).
///
/// Also see [`is_focus_within`](fn@zero_ui::properties::focus::is_focus_within) to check if the widget has
/// focus within regardless of highlighting.
#[property(context)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsFocusWithinHglNode<C: UiNode> {
        child: C,
        state: StateVar,
        focus_changed: EventListener<FocusChangedArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsFocusWithinHglNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(u) = self.focus_changed.updates(ctx.events).last() {
                let was_focused_hgl = *self.state.get(ctx.vars);
                let is_focused_hgl = u.highlight && u.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or_default();

                if was_focused_hgl != is_focused_hgl {
                    self.state.set(ctx.vars, is_focused_hgl);
                }
            }
            self.child.update(ctx);
        }
    }
    IsFocusWithinHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget is focused when a parent scope is focused.
#[property(context)]
pub fn is_return_focus(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsReturnFocusNode<C: UiNode> {
        child: C,
        state: StateVar,
        return_focus_changed: EventListener<ReturnFocusChangedArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsReturnFocusNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.return_focus_changed = ctx.events.listen::<ReturnFocusChangedEvent>();
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.return_focus_changed = ReturnFocusChangedEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            let state = *self.state.get(ctx.vars);
            let mut new_state = state;
            for args in self.return_focus_changed.updates(ctx.events) {
                if args
                    .prev_return
                    .as_ref()
                    .map(|p| p.widget_id() == ctx.path.widget_id())
                    .unwrap_or_default()
                {
                    new_state = false;
                }
                if args
                    .new_return
                    .as_ref()
                    .map(|p| p.widget_id() == ctx.path.widget_id())
                    .unwrap_or_default()
                {
                    new_state = true;
                }
            }

            if new_state != state {
                self.state.set(ctx.vars, new_state);
            }
        }
    }
    IsReturnFocusNode {
        child,
        state,
        return_focus_changed: ReturnFocusChangedEvent::never(),
    }
}
