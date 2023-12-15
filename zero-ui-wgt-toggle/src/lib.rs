//! Toggle widget, properties and commands.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zero_ui_wgt::enable_widget_macros!();

use std::ops;
use std::{any::Any, error::Error, fmt, marker::PhantomData, sync::Arc};

use task::parking_lot::Mutex;
use zero_ui_ext_font::FontNames;
use zero_ui_ext_input::{
    gesture::CLICK_EVENT,
    mouse::{ClickMode, MOUSE_INPUT_EVENT},
    pointer_capture::CaptureMode,
};
use zero_ui_ext_l10n::lang;
use zero_ui_var::VarIsReadOnlyError;
use zero_ui_wgt::{align, border, border_align, border_over, corner_radius, hit_test_mode, is_inited, prelude::*, Wgt};
use zero_ui_wgt_access::{access_role, accessible, AccessRole};
use zero_ui_wgt_container::{child_align, child_insert_end, child_insert_start, padding};
use zero_ui_wgt_fill::background_color;
use zero_ui_wgt_filter::opacity;
use zero_ui_wgt_input::{click_mode, is_hovered, pointer_capture::capture_pointer_on_init};
use zero_ui_wgt_layer::popup::{PopupState, POPUP};
use zero_ui_wgt_size_offset::{size, x, y};
use zero_ui_wgt_style::{Style, StyleFn};
use zero_ui_wgt_transform::scale_y;

pub mod cmd;

/// A toggle button that flips a `bool` or `Option<bool>` variable on click, or selects a value.
///
/// This widget has three primary properties, [`checked`], [`checked_opt`] and [`value`], setting one
/// of the checked properties to a read-write variable enables the widget and it will set the variables
/// on click, setting [`value`] turns the toggle in a selection item that is inserted/removed in a contextual [`selector`].
///
/// [`checked`]: fn@checked
/// [`checked_opt`]: fn@checked_opt
/// [`value`]: fn@value
/// [`selector`]: fn@selector
#[widget($crate::Toggle)]
pub struct Toggle(zero_ui_wgt_button::Button);
impl Toggle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// The toggle button checked state.
    pub static IS_CHECKED_VAR: Option<bool> = false;

    /// If toggle button cycles between `None`, `Some(false)` and `Some(true)` on click.
    pub static IS_TRISTATE_VAR: bool = false;
}

/// Toggle cycles between `true` and `false`, updating the variable.
///
/// # Examples
///
/// The variable `foo` is toggled on click and it also controls the checked state of the widget.
///
/// ```
/// # macro_rules! _demo { () => {
/// let foo = var(false);
///
/// Toggle! {
///     checked = foo.clone();
///
///     child = Text!(foo.map(|b| formatx!("foo = {b}")));
/// }
/// # }}
/// ```
///
/// Note that you can read the checked state of the widget using [`is_checked`].
///
/// [`is_checked`]: fn@is_checked
#[property(CONTEXT, default(false), widget_impl(Toggle))]
pub fn checked(child: impl UiNode, checked: impl IntoVar<bool>) -> impl UiNode {
    let checked = checked.into_var();
    let mut _toggle_handle = CommandHandle::dummy();
    let mut access_handle = VarHandle::dummy();
    let node = match_node(
        child,
        clmv!(checked, |child, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_event(&CLICK_EVENT);
                _toggle_handle = cmd::TOGGLE_CMD.scoped(WIDGET.id()).subscribe(true);
            }
            UiNodeOp::Deinit => {
                _toggle_handle = CommandHandle::dummy();
                access_handle = VarHandle::dummy();
            }
            UiNodeOp::Info { info } => {
                if let Some(mut a) = info.access() {
                    if access_handle.is_dummy() {
                        access_handle = checked.subscribe(UpdateOp::Info, WIDGET.id());
                    }
                    a.set_checked(Some(checked.get()));
                }
            }
            UiNodeOp::Event { update } => {
                child.event(update);

                if let Some(args) = CLICK_EVENT.on(update) {
                    if args.is_primary()
                        && checked.capabilities().contains(VarCapabilities::MODIFY)
                        && !args.propagation().is_stopped()
                        && args.is_enabled(WIDGET.id())
                    {
                        args.propagation().stop();

                        let _ = checked.set(!checked.get());
                    }
                } else if let Some(args) = cmd::TOGGLE_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                    if let Some(b) = args.param::<bool>() {
                        args.propagation().stop();
                        let _ = checked.set(*b);
                    } else if let Some(b) = args.param::<Option<bool>>() {
                        if let Some(b) = b {
                            args.propagation().stop();
                            let _ = checked.set(*b);
                        }
                    } else if args.param.is_none() {
                        args.propagation().stop();
                        let _ = checked.set(!checked.get());
                    }
                }
            }
            _ => {}
        }),
    );
    with_context_var(node, IS_CHECKED_VAR, checked.map_into())
}

/// Toggle cycles between `Some(true)` and `Some(false)` and accepts `None`, if the
/// widget is `tristate` also sets to `None` in the toggle cycle.
///
/// # Examples
///
/// The variable `foo` is cycles the three states on click.
///
/// ```
/// # macro_rules! _demo { () => {
/// let foo = var(Some(false));
///
/// Toggle! {
///     checked_opt = foo.clone();
///     tristate = true;
///
///     child = Text!(foo.map(|b| formatx!("foo = {b:?}")));
/// }
/// # }}
/// ```
#[property(CONTEXT + 1, default(None), widget_impl(Toggle))]
pub fn checked_opt(child: impl UiNode, checked: impl IntoVar<Option<bool>>) -> impl UiNode {
    let checked = checked.into_var();
    let mut _toggle_handle = CommandHandle::dummy();
    let mut access_handle = VarHandle::dummy();

    let node = match_node(
        child,
        clmv!(checked, |child, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_event(&CLICK_EVENT);
                _toggle_handle = cmd::TOGGLE_CMD.scoped(WIDGET.id()).subscribe(true);
            }
            UiNodeOp::Deinit => {
                _toggle_handle = CommandHandle::dummy();
                access_handle = VarHandle::dummy();
            }
            UiNodeOp::Info { info } => {
                if let Some(mut a) = info.access() {
                    if access_handle.is_dummy() {
                        access_handle = checked.subscribe(UpdateOp::Info, WIDGET.id());
                    }
                    a.set_checked(checked.get());
                }
            }
            UiNodeOp::Event { update } => {
                child.event(update);

                let mut cycle = false;

                if let Some(args) = CLICK_EVENT.on(update) {
                    if args.is_primary()
                        && checked.capabilities().contains(VarCapabilities::MODIFY)
                        && !args.propagation().is_stopped()
                        && args.is_enabled(WIDGET.id())
                    {
                        args.propagation().stop();

                        cycle = true;
                    }
                } else if let Some(args) = cmd::TOGGLE_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                    if let Some(b) = args.param::<bool>() {
                        args.propagation().stop();
                        let _ = checked.set(Some(*b));
                    } else if let Some(b) = args.param::<Option<bool>>() {
                        if IS_TRISTATE_VAR.get() {
                            args.propagation().stop();
                            let _ = checked.set(*b);
                        } else if let Some(b) = b {
                            args.propagation().stop();
                            let _ = checked.set(Some(*b));
                        }
                    } else if args.param.is_none() {
                        args.propagation().stop();

                        cycle = true;
                    }
                }

                if cycle {
                    if IS_TRISTATE_VAR.get() {
                        let _ = checked.set(match checked.get() {
                            Some(true) => None,
                            Some(false) => Some(true),
                            None => Some(false),
                        });
                    } else {
                        let _ = checked.set(match checked.get() {
                            Some(true) | None => Some(false),
                            Some(false) => Some(true),
                        });
                    }
                }
            }
            _ => {}
        }),
    );

    with_context_var(node, IS_CHECKED_VAR, checked)
}

/// Enables `None` as an input value.
///
/// Note that `None` is always accepted in `checked_opt`, this property controls if
/// `None` is one of the values in the toggle cycle. If the widget is bound to the `checked` property
/// this config is ignored.
///
/// This is not enabled by default.
///
/// [`checked_opt`]: fn@checked_opt
#[property(CONTEXT, default(IS_TRISTATE_VAR), widget_impl(Toggle))]
pub fn tristate(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IS_TRISTATE_VAR, enabled)
}

/// If the toggle is checked from any of the three primary properties.
///
/// Note to read the tristate use [`IS_CHECKED_VAR`] directly.
///
/// # Examples
///
/// The `is_checked` state is set when the [`checked`] is `true`, or [`checked_opt`] is `Some(true)` or the [`value`]
/// is selected.
///
/// ```
/// # macro_rules! _demo { () => {
/// Toggle! {
///     checked = var(false);
///     // checked_opt = var(Some(false));
///     // value<i32> = 42;
///
///     child = Text!("Toggle Background");
///     background_color = colors::RED;
///     when *#is_checked {
///         background_color = colors::GREEN;
///     }
/// }
/// # }}
/// ```
///
/// [`checked`]: fn@checked
/// [`checked_opt`]: fn@checked_opt
/// [`value`]: fn@value.
#[property(EVENT, widget_impl(Toggle))]
pub fn is_checked(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, IS_CHECKED_VAR.map(|s| *s == Some(true)), state)
}

/// Values that is selected in the contextual [`selector`].
///
/// The widget [`is_checked`] when the value is selected, on click and on value update, the selection
/// is updated according to the behavior defined in the contextual [`selector`]. If no contextual
/// [`selector`] is the the widget is never checked.
///
/// Note that the value can be any type, but must be one of the types accepted by the contextual [`selector`], type
/// validation happens in run-time, an error is logged if the type is not compatible. Because any type can be used in
/// this property type inference cannot resolve the type automatically and a type annotation is required: `value<T> = t;`.
///
/// # Examples
///
/// The variable `foo` is set to a `value` clone on click, or if the `value` updates when the previous was selected.
///
/// ```
/// # macro_rules! _demo { () => {
/// let foo = var(1_i32);
///
/// Stack! {
///     toggle::selector = toggle::Selector::single(foo.clone());
///
///     spacing = 5;
///     children = (1..=10_i32).map(|i| {
///         Toggle! {
///             child = Text!("Item {i}");
///             value::<i32> = i;
///         }
///         .boxed()
///     }).collect::<Vec<_>>();
/// }
/// # }}
/// ```
///
/// [`is_checked`]: fn@is_checked
/// [`selector`]: fn@selector
///
/// This property interacts with the contextual [`selector`], when the widget is clicked or the `value` variable changes
/// the contextual [`Selector`] is used to implement the behavior.
///
/// [`selector`]: fn@selector
#[property(CONTEXT+2, widget_impl(Toggle))]
pub fn value<T: VarValue + PartialEq>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
    // Returns `true` if selected.
    let select = |value: &T| {
        let selector = SELECTOR.get();
        match selector.select(Box::new(value.clone())) {
            Ok(()) => true,
            Err(e) => {
                let selected = selector.is_selected(value);
                if selected {
                    tracing::error!("selected `{value:?}` with error, {e}");
                } else if let SelectorError::ReadOnly | SelectorError::CannotClear = e {
                    // ignore
                } else {
                    tracing::error!("failed to select `{value:?}`, {e}");
                }
                selected
            }
        }
    };
    // Returns `true` if deselected.
    let deselect = |value: &T| {
        let selector = SELECTOR.get();
        match selector.deselect(value) {
            Ok(()) => true,
            Err(e) => {
                let deselected = !selector.is_selected(value);
                if deselected {
                    tracing::error!("deselected `{value:?}` with error, {e}");
                } else if let SelectorError::ReadOnly | SelectorError::CannotClear = e {
                    // ignore
                } else {
                    tracing::error!("failed to deselect `{value:?}`, {e}");
                }
                deselected
            }
        }
    };
    let is_selected = |value: &T| SELECTOR.get().is_selected(value);

    let value = value.into_var();
    let checked = var(Some(false));
    let child = with_context_var(child, IS_CHECKED_VAR, checked.clone());
    let mut prev_value = None;

    let mut _click_handle = None;
    let mut _toggle_handle = CommandHandle::dummy();
    let mut _select_handle = CommandHandle::dummy();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            let id = WIDGET.id();
            WIDGET.sub_var(&value).sub_var(&DESELECT_ON_NEW_VAR).sub_var(&checked);
            SELECTOR.get().subscribe();

            value.with(|value| {
                let selected = if SELECT_ON_INIT_VAR.get() {
                    select(value)
                } else {
                    is_selected(value)
                };
                checked.set(Some(selected));

                if DESELECT_ON_DEINIT_VAR.get() {
                    prev_value = Some(value.clone());
                }
            });

            _click_handle = Some(CLICK_EVENT.subscribe(id));
            _toggle_handle = cmd::TOGGLE_CMD.scoped(id).subscribe(true);
            _select_handle = cmd::SELECT_CMD.scoped(id).subscribe(true);
        }
        UiNodeOp::Deinit => {
            if checked.get() == Some(true) && DESELECT_ON_DEINIT_VAR.get() {
                value.with(|value| {
                    if deselect(value) {
                        checked.set(Some(false));
                    }
                });
            }

            prev_value = None;
            _click_handle = None;
            _toggle_handle = CommandHandle::dummy();
            _select_handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = CLICK_EVENT.on(update) {
                if args.is_primary() && !args.propagation().is_stopped() && args.is_enabled(WIDGET.id()) {
                    args.propagation().stop();

                    let selected = value.with(|value| {
                        let selected = checked.get() == Some(true);
                        if selected {
                            !deselect(value)
                        } else {
                            select(value)
                        }
                    });
                    checked.set(Some(selected))
                }
            } else if let Some(args) = cmd::TOGGLE_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                if args.param.is_none() {
                    args.propagation().stop();

                    let selected = value.with(|value| {
                        let selected = checked.get() == Some(true);
                        if selected {
                            !deselect(value)
                        } else {
                            select(value)
                        }
                    });
                    checked.set(Some(selected))
                } else {
                    let s = if let Some(s) = args.param::<Option<bool>>() {
                        Some(s.unwrap_or(false))
                    } else {
                        args.param::<bool>().copied()
                    };
                    if let Some(s) = s {
                        args.propagation().stop();

                        let selected = value.with(|value| if s { select(value) } else { !deselect(value) });
                        checked.set(Some(selected))
                    }
                }
            } else if let Some(args) = cmd::SELECT_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                if args.param.is_none() {
                    args.propagation().stop();
                    value.with(|value| {
                        let selected = checked.get() == Some(true);
                        if !selected && select(value) {
                            checked.set(Some(true));
                        }
                    });
                }
            }
        }
        UiNodeOp::Update { .. } => {
            let selected = value.with_new(|new| {
                // auto select new.
                let selected = if checked.get() == Some(true) && SELECT_ON_NEW_VAR.get() {
                    select(new)
                } else {
                    is_selected(new)
                };

                // auto deselect prev, need to be done after potential auto select new to avoid `CannotClear` error.
                if let Some(prev) = prev_value.take() {
                    if DESELECT_ON_NEW_VAR.get() {
                        deselect(&prev);
                        prev_value = Some(new.clone());
                    }
                }

                selected
            });
            let selected = selected.unwrap_or_else(|| {
                // contextual selector can change in any update.
                value.with(is_selected)
            });
            checked.set(selected);

            if DESELECT_ON_NEW_VAR.get() && selected {
                // save a clone of the value to reference it on deselection triggered by variable value changing.
                if prev_value.is_none() {
                    prev_value = Some(value.get());
                }
            } else {
                prev_value = None;
            }

            if let Some(Some(true)) = checked.get_new() {
                if SCROLL_ON_SELECT_VAR.get() {
                    use zero_ui_wgt_scroll::cmd::*;
                    scroll_to(WIDGET.id(), ScrollToMode::minimal(10));
                }
            }
        }
        _ => {}
    })
}

/// If the scrolls into view when the [`value`] selected.
///
/// This is enabled by default.
///
/// [`value`]: fn@value
#[property(CONTEXT, default(SCROLL_ON_SELECT_VAR), widget_impl(Toggle))]
pub fn scroll_on_select(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, SCROLL_ON_SELECT_VAR, enabled)
}

/// Sets the contextual selector that all inner widgets will target from the [`value`] property.
///
/// All [`value`] properties declared in widgets inside `child` will use the [`Selector`] to manipulate
/// the selection.
///
/// Selection in a context can be blocked by setting the selector to [`Selector::nil()`], this is also the default
/// selector so the [`value`] property only works if a contextual selector is present.
///
/// This property sets the [`SELECTOR`] context and handles [`cmd::SelectOp`] requests. It also sets the widget
/// access role to [`AccessRole::RadioGroup`].
///
/// [`value`]: fn@value
#[property(CONTEXT, default(Selector::nil()), widget_impl(Toggle))]
pub fn selector(child: impl UiNode, selector: impl IntoValue<Selector>) -> impl UiNode {
    let mut _select_handle = CommandHandle::dummy();
    let child = match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            _select_handle = cmd::SELECT_CMD.scoped(WIDGET.id()).subscribe(true);
        }
        UiNodeOp::Info { info } => {
            if let Some(mut info) = info.access() {
                info.set_role(AccessRole::RadioGroup);
            }
        }
        UiNodeOp::Deinit => {
            _select_handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = cmd::SELECT_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                if let Some(p) = args.param::<cmd::SelectOp>() {
                    args.propagation().stop();

                    p.call();
                }
            }
        }
        _ => {}
    });
    with_context_local(child, &SELECTOR, selector)
}

/// If [`value`] is selected when the widget that has the value is inited.
///
/// [`value`]: fn@value
#[property(CONTEXT, default(SELECT_ON_INIT_VAR), widget_impl(Toggle))]
pub fn select_on_init(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, SELECT_ON_INIT_VAR, enabled)
}

/// If [`value`] is deselected when the widget that has the value is deinited and the value was selected.
///
/// [`value`]: fn@value
#[property(CONTEXT, default(DESELECT_ON_DEINIT_VAR), widget_impl(Toggle))]
pub fn deselect_on_deinit(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, DESELECT_ON_DEINIT_VAR, enabled)
}

/// If [`value`] selects the new value when the variable changes and the previous value was selected.
///
/// [`value`]: fn@value
#[property(CONTEXT, default(SELECT_ON_NEW_VAR), widget_impl(Toggle))]
pub fn select_on_new(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, SELECT_ON_NEW_VAR, enabled)
}

/// If [`value`] deselects the previously selected value when the variable changes.
///
/// [`value`]: fn@value
#[property(CONTEXT, default(DESELECT_ON_NEW_VAR), widget_impl(Toggle))]
pub fn deselect_on_new(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, DESELECT_ON_NEW_VAR, enabled)
}

context_local! {
    /// Contextual [`Selector`].
    pub static SELECTOR: Selector = Selector::nil();
}

context_var! {
    /// If [`value`] is selected when the widget that has the value is inited.
    ///
    /// Use the [`select_on_init`] property to set. By default is `false`.
    ///
    /// [`value`]: fn@value
    /// [`select_on_init`]: fn@select_on_init
    pub static SELECT_ON_INIT_VAR: bool = false;

    /// If [`value`] is deselected when the widget that has the value is deinited and the value was selected.
    ///
    /// Use the [`deselect_on_deinit`] property to set. By default is `false`.
    ///
    /// [`value`]: fn@value
    /// [`deselect_on_deinit`]: fn@deselect_on_deinit
    pub static DESELECT_ON_DEINIT_VAR: bool = false;

    /// If [`value`] selects the new value when the variable changes and the previous value was selected.
    ///
    /// Use the [`select_on_new`] property to set. By default is `true`.
    ///
    /// [`value`]: fn@value
    /// [`select_on_new`]: fn@select_on_new
    pub static SELECT_ON_NEW_VAR: bool = true;

    /// If [`value`] deselects the previously selected value when the variable changes.
    ///
    /// Use the [`deselect_on_new`] property to set. By default is `false`.
    ///
    /// [`value`]: fn@value
    /// [`select_on_new`]: fn@select_on_new
    pub static DESELECT_ON_NEW_VAR: bool = false;

    /// If [`value`] scrolls into view when selected.
    ///
    /// This is enabled by default.
    ///
    /// [`value`]: fn@value
    pub static SCROLL_ON_SELECT_VAR: bool = true;
}

/// Represents a [`Selector`] implementation.
pub trait SelectorImpl: Send + 'static {
    /// Add the selector subscriptions in the [`WIDGET`].
    fn subscribe(&self);

    /// Insert the `value` in the selection, returns `Ok(())` if the value was inserted or was already selected.
    fn select(&mut self, value: Box<dyn Any>) -> Result<(), SelectorError>;

    /// Remove the `value` from the selection, returns `Ok(())` if the value was removed or was not selected.
    fn deselect(&mut self, value: &dyn Any) -> Result<(), SelectorError>;

    /// Returns `true` if the `value` is selected.
    fn is_selected(&self, value: &dyn Any) -> bool;
}

/// Represents the contextual selector behavior of [`value`] selector.
///
/// A selector can be set using [`selector`], all [`value`] widgets in context will target it.
///
/// [`value`]: fn@value
/// [`selector`]: fn@selector
#[derive(Clone)]
pub struct Selector(Arc<Mutex<dyn SelectorImpl>>);
impl Selector {
    /// New custom selector.
    pub fn new(selector: impl SelectorImpl) -> Self {
        Self(Arc::new(Mutex::new(selector)))
    }

    /// Represents no selector and the inability to select any item.
    pub fn nil() -> Self {
        struct NilSel;
        impl SelectorImpl for NilSel {
            fn subscribe(&self) {}

            fn select(&mut self, _: Box<dyn Any>) -> Result<(), SelectorError> {
                Err(SelectorError::custom_str("no contextual `selector`"))
            }

            fn deselect(&mut self, _: &dyn Any) -> Result<(), SelectorError> {
                Ok(())
            }

            fn is_selected(&self, __r: &dyn Any) -> bool {
                false
            }
        }
        Self::new(NilSel)
    }

    /// Represents the "radio" selection of a single item.
    pub fn single<T>(selection: impl IntoVar<T>) -> Self
    where
        T: VarValue,
    {
        struct SingleSel<T, S> {
            selection: S,
            _type: PhantomData<T>,
        }
        impl<T, S> SelectorImpl for SingleSel<T, S>
        where
            T: VarValue,
            S: Var<T>,
        {
            fn subscribe(&self) {
                WIDGET.sub_var(&self.selection);
            }

            fn select(&mut self, value: Box<dyn Any>) -> Result<(), SelectorError> {
                match value.downcast::<T>() {
                    Ok(value) => match self.selection.set(*value) {
                        Ok(_) => Ok(()),
                        Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                    },
                    Err(_) => Err(SelectorError::WrongType),
                }
            }

            fn deselect(&mut self, value: &dyn Any) -> Result<(), SelectorError> {
                if self.is_selected(value) {
                    Err(SelectorError::CannotClear)
                } else {
                    Ok(())
                }
            }

            fn is_selected(&self, value: &dyn Any) -> bool {
                match value.downcast_ref::<T>() {
                    Some(value) => self.selection.with(|t| t == value),
                    None => false,
                }
            }
        }
        Self::new(SingleSel {
            selection: selection.into_var(),
            _type: PhantomData,
        })
    }

    /// Represents the "radio" selection of a single item that is optional.
    pub fn single_opt<T>(selection: impl IntoVar<Option<T>>) -> Self
    where
        T: VarValue,
    {
        struct SingleOptSel<T, S> {
            selection: S,
            _type: PhantomData<T>,
        }
        impl<T, S> SelectorImpl for SingleOptSel<T, S>
        where
            T: VarValue,
            S: Var<Option<T>>,
        {
            fn subscribe(&self) {
                WIDGET.sub_var(&self.selection);
            }

            fn select(&mut self, value: Box<dyn Any>) -> Result<(), SelectorError> {
                match value.downcast::<T>() {
                    Ok(value) => match self.selection.set(Some(*value)) {
                        Ok(_) => Ok(()),
                        Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                    },
                    Err(value) => match value.downcast::<Option<T>>() {
                        Ok(value) => match self.selection.set(*value) {
                            Ok(_) => Ok(()),
                            Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                        },
                        Err(_) => Err(SelectorError::WrongType),
                    },
                }
            }

            fn deselect(&mut self, value: &dyn Any) -> Result<(), SelectorError> {
                match value.downcast_ref::<T>() {
                    Some(value) => {
                        if self.selection.with(|t| t.as_ref() == Some(value)) {
                            match self.selection.set(None) {
                                Ok(_) => Ok(()),
                                Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                            }
                        } else {
                            Ok(())
                        }
                    }
                    None => match value.downcast_ref::<Option<T>>() {
                        Some(value) => {
                            if self.selection.with(|t| t == value) {
                                if value.is_none() {
                                    Ok(())
                                } else {
                                    match self.selection.set(None) {
                                        Ok(_) => Ok(()),
                                        Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                                    }
                                }
                            } else {
                                Ok(())
                            }
                        }
                        None => Ok(()),
                    },
                }
            }

            fn is_selected(&self, value: &dyn Any) -> bool {
                match value.downcast_ref::<T>() {
                    Some(value) => self.selection.with(|t| t.as_ref() == Some(value)),
                    None => match value.downcast_ref::<Option<T>>() {
                        Some(value) => self.selection.with(|t| t == value),
                        None => false,
                    },
                }
            }
        }
        Self::new(SingleOptSel {
            selection: selection.into_var(),
            _type: PhantomData,
        })
    }

    /// Represents the "check list" selection of bitflags.
    pub fn bitflags<T>(selection: impl IntoVar<T>) -> Self
    where
        T: VarValue + ops::BitOr<Output = T> + ops::BitAnd<Output = T> + ops::Not<Output = T>,
    {
        struct BitflagsSel<T, S> {
            selection: S,
            _type: PhantomData<T>,
        }
        impl<T, S> SelectorImpl for BitflagsSel<T, S>
        where
            T: VarValue + ops::BitOr<Output = T> + ops::BitAnd<Output = T> + ops::Not<Output = T>,
            S: Var<T>,
        {
            fn subscribe(&self) {
                WIDGET.sub_var(&self.selection);
            }

            fn select(&mut self, value: Box<dyn Any>) -> Result<(), SelectorError> {
                match value.downcast::<T>() {
                    Ok(value) => self
                        .selection
                        .modify(move |m| {
                            let value = *value;
                            let new = m.as_ref().clone() | value;
                            if m.as_ref() != &new {
                                m.set(new);
                            }
                        })
                        .map_err(|_| SelectorError::ReadOnly),
                    Err(_) => Err(SelectorError::WrongType),
                }
            }

            fn deselect(&mut self, value: &dyn Any) -> Result<(), SelectorError> {
                match value.downcast_ref::<T>() {
                    Some(value) => self
                        .selection
                        .modify(clmv!(value, |m| {
                            let new = m.as_ref().clone() & !value;
                            if m.as_ref() != &new {
                                m.set(new);
                            }
                        }))
                        .map_err(|_| SelectorError::ReadOnly),
                    None => Err(SelectorError::WrongType),
                }
            }

            fn is_selected(&self, value: &dyn Any) -> bool {
                match value.downcast_ref::<T>() {
                    Some(value) => &(self.selection.get() & value.clone()) == value,
                    None => false,
                }
            }
        }

        Self::new(BitflagsSel {
            selection: selection.into_var(),
            _type: PhantomData,
        })
    }

    /// Add the selector subscriptions in [`WIDGET`].
    pub fn subscribe(&self) {
        self.0.lock().subscribe();
    }

    /// Insert the `value` in the selection, returns `Ok(())` if the value was inserted or was already selected.
    pub fn select(&self, value: Box<dyn Any>) -> Result<(), SelectorError> {
        self.0.lock().select(value)
    }

    /// Remove the `value` from the selection, returns `Ok(())` if the value was removed or was not selected.
    pub fn deselect(&self, value: &dyn Any) -> Result<(), SelectorError> {
        self.0.lock().deselect(value)
    }

    /// Returns `true` if the `value` is selected.
    pub fn is_selected(&self, value: &dyn Any) -> bool {
        self.0.lock().is_selected(value)
    }
}
impl<S: SelectorImpl> From<S> for Selector {
    fn from(sel: S) -> Self {
        Selector::new(sel)
    }
}
impl fmt::Debug for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Selector(_)")
    }
}
impl PartialEq for Selector {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Error for [`Selector`] operations.
#[derive(Debug, Clone)]
pub enum SelectorError {
    /// Cannot select item because it is not of type that the selector can handle.
    WrongType,
    /// Cannot (de)select item because the selection is read-only.
    ReadOnly,
    /// Cannot deselect item because the selection cannot be empty.
    CannotClear,
    /// Cannot select item because of a selector specific reason.
    Custom(Arc<dyn Error + Send + Sync>),
}
impl SelectorError {
    /// New custom error from string.
    pub fn custom_str(str: impl Into<String>) -> SelectorError {
        let str = str.into();
        let e: Box<dyn Error + Send + Sync> = str.into();
        let e: Arc<dyn Error + Send + Sync> = e.into();
        SelectorError::Custom(e)
    }
}
impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectorError::WrongType => write!(f, "wrong value type for selector"),
            SelectorError::ReadOnly => write!(f, "selection is read-only"),
            SelectorError::CannotClear => write!(f, "selection cannot be empty"),
            SelectorError::Custom(e) => fmt::Display::fmt(e, f),
        }
    }
}
impl Error for SelectorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SelectorError::WrongType => None,
            SelectorError::ReadOnly => None,
            SelectorError::CannotClear => None,
            SelectorError::Custom(e) => Some(&**e),
        }
    }
}
impl From<VarIsReadOnlyError> for SelectorError {
    fn from(_: VarIsReadOnlyError) -> Self {
        SelectorError::ReadOnly
    }
}

context_var! {
    /// Toggle style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Sets the toggle style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the toggle style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    zero_ui_wgt_style::with_style_extension(child, STYLE_VAR, style)
}

/// Default toggle style.
///
/// Extends the [`button::DefaultStyle`] to have the *pressed* look when [`is_checked`].
///
/// [`button::DefaultStyle`]: struct@crate::widgets::button::DefaultStyle
/// [`is_checked`]: fn@is_checked
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(zero_ui_wgt_button::DefaultStyle);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            when *#is_checked  {
                background_color = zero_ui_wgt_button::color_scheme_pressed(zero_ui_wgt_button::BASE_COLORS_VAR);
                border = {
                    widths: 1,
                    sides: zero_ui_wgt_button::color_scheme_pressed(zero_ui_wgt_button::BASE_COLORS_VAR).map_into(),
                };
            }
        }
    }
}

/// Checkmark toggle style.
///
/// Style a [`Toggle!`] widget to look like a *checkbox*.
///
/// [`Toggle!`]: struct@Toggle
#[widget($crate::CheckStyle)]
pub struct CheckStyle(Style);
impl CheckStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            child_insert_start = {
                insert: {
                    let parent_hovered = var(false);
                    is_hovered(checkmark_visual(parent_hovered.clone()), parent_hovered)
                },
                spacing: CHECK_SPACING_VAR,
            };
            access_role = AccessRole::CheckBox;
        }
    }
}
context_var! {
    /// Spacing between the checkmark and the content.
    pub static CHECK_SPACING_VAR: Length = 2;
}

/// Spacing between the checkmark and the content.
#[property(CONTEXT, default(CHECK_SPACING_VAR), widget_impl(CheckStyle))]
pub fn check_spacing(child: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, CHECK_SPACING_VAR, spacing)
}

fn checkmark_visual(parent_hovered: impl Var<bool>) -> impl UiNode {
    zero_ui_wgt_text::Text! {
        hit_test_mode = false;
        accessible = false;
        size = 1.2.em();
        font_family = FontNames::system_ui(&lang!(und));
        txt_align = Align::CENTER;
        align = Align::CENTER;
        corner_radius = 0.1.em();

        txt = "✓";
        when #{IS_CHECKED_VAR}.is_none() {
            txt = "━";
        }

        font_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.transparent());
        when #{IS_CHECKED_VAR}.unwrap_or(true) {
            font_color = zero_ui_wgt_text::FONT_COLOR_VAR;
        }

        #[easing(150.ms())]
        background_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
        when *#{parent_hovered} {
            #[easing(0.ms())]
            background_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()));
        }
    }
}

/// Popup toggle style.
///
/// Style a [`Toggle!`] widget to look like the popup toggle button in a *combo-box*.
///
/// [`Toggle!`]: struct@Toggle
#[widget($crate::ComboStyle)]
pub struct ComboStyle(DefaultStyle);
impl ComboStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            access_role = AccessRole::ComboBox;
            child_align = Align::FILL;
            border_over = false;
            border_align = 1.fct();
            padding = COMBO_SPACING_VAR.map(|e| SideOffsets::new(-1, e.clone(), -1, -1));
            checked = var(false);
            child_insert_end = {
                insert: combomark_visual(),
                spacing: COMBO_SPACING_VAR,
            };

            click_mode = ClickMode::press();

            zero_ui_wgt_button::extend_style = Style! { // button in child.
                click_mode = ClickMode::default();
                corner_radius = (4, 0, 0, 4);
            };

            zero_ui_wgt_layer::popup::extend_style = Style! {
                zero_ui_wgt_button::extend_style = Style! {
                    click_mode = ClickMode::release();

                    corner_radius = 0;
                    padding = 2;
                    border = unset!;
                };
                crate::extend_style = Style! {
                    click_mode = ClickMode::release();

                    corner_radius = 0;
                    padding = 2;
                    border = unset!;
                };

                // supports gesture of press-and-drag to select.
                //
                // - `Toggle!` inherits `capture_pointer = true` from `Button!`.
                // - `ComboStyle!` sets `click_mode = press`.
                // - `ComboStyle!` sets popup descendant `Button!` to `click_mode = release`.
                //
                // So the user can press to open the drop-down, then drag over an option and release to select it.
                capture_pointer_on_init = CaptureMode::Subtree;

                #[easing(100.ms())]
                opacity = 0.pct();
                #[easing(100.ms())]
                y = -10;

                when *#is_inited {
                    opacity = 100.pct();
                    y = 0;
                }

                zero_ui_wgt_layer::popup::close_delay = 100.ms();
                when *#zero_ui_wgt_layer::popup::is_close_delaying {
                    opacity = 0.pct();
                    y = -10;
                }
            };
        }
    }
}
context_var! {
    /// Spacing between the arrow symbol and the content.
    pub static COMBO_SPACING_VAR: Length = 2;
}

/// Spacing between the arrow symbol and the content.
#[property(CONTEXT, default(COMBO_SPACING_VAR), widget_impl(ComboStyle))]
pub fn combo_spacing(child: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, COMBO_SPACING_VAR, spacing)
}

/// Popup open when the toggle button is checked.
///
/// This property can be used together with the [`ComboStyle!`] to implement a *combo-box* flyout widget.
///
/// The `popup` can be any widget, that will be open using [`POPUP`], a [`Popup!`] or derived widget is recommended.
///
/// [`ComboStyle!`]: struct@ComboStyle
/// [`Popup!`]: struct@zero_ui_wgt_layer::popup::Popup
/// [`LAYERS`]: layers::LAYERS
#[property(CHILD, widget_impl(Toggle))]
pub fn checked_popup(child: impl UiNode, popup: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    let popup = popup.into_var();
    let mut state = var(PopupState::Closed).read_only();
    let mut _state_handle = VarHandle::dummy();
    match_node(child, move |_, op| {
        let new = match op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&IS_CHECKED_VAR).sub_event(&MOUSE_INPUT_EVENT);
                IS_CHECKED_VAR.get()
            }
            UiNodeOp::Deinit => {
                _state_handle = VarHandle::dummy();
                Some(false)
            }
            UiNodeOp::Event { update } => {
                if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    // close on mouse down to avoid issue when the popup closes on mouse-down (due to focus loss),
                    // but a click is formed (down+up) on the toggle that immediately opens the popup again.
                    if args.is_mouse_down() && args.is_primary() && IS_CHECKED_VAR.get() == Some(true) {
                        args.propagation().stop();
                        cmd::TOGGLE_CMD.scoped(WIDGET.id()).notify_param(Some(false));
                    }
                }
                None
            }
            UiNodeOp::Update { .. } => {
                if let Some(s) = state.get_new() {
                    if matches!(s, PopupState::Closed) {
                        if IS_CHECKED_VAR.get() != Some(false) {
                            cmd::TOGGLE_CMD.scoped(WIDGET.id()).notify_param(Some(false));
                        }
                        _state_handle = VarHandle::dummy();
                    }
                    None
                } else {
                    IS_CHECKED_VAR.get_new().map(|o| o.unwrap_or(false))
                }
            }
            _ => None,
        };
        if let Some(open) = new {
            if open {
                if matches!(state.get(), PopupState::Closed) {
                    state = POPUP.open(popup.get()(()));
                    _state_handle = state.subscribe(UpdateOp::Update, WIDGET.id());
                }
            } else if let PopupState::Open(id) = state.get() {
                POPUP.close_id(id);
            }
        }
    })
}

#[allow(non_snake_case)]
fn combomark_visual() -> impl UiNode {
    zero_ui_wgt_text::Text! {
        hit_test_mode = false;
        accessible = false;
        font_family = FontNames::system_ui(&lang!(und));
        txt_align = Align::CENTER;
        align = Align::CENTER;
        font_size = 0.6.em();

        txt = "▼";

        when #{IS_CHECKED_VAR}.unwrap_or(true) {
            scale_y = -1.0f32;
        }
    }
}

/// Switch toggle style.
///
/// Style a [`Toggle!`] widget to look like a *switch*.
///
/// [`Toggle!`]: struct@crate::widgets::Toggle
#[widget($crate::SwitchStyle)]
pub struct SwitchStyle(Style);
impl SwitchStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            child_insert_start = {
                insert: {
                    let parent_hovered = var(false);
                    is_hovered(switch_visual(parent_hovered.clone()), parent_hovered)
                },
                spacing: SWITCH_SPACING_VAR,
            };
        }
    }
}
context_var! {
    /// Spacing between the switch and the content.
    pub static SWITCH_SPACING_VAR: Length = 2;
}

/// Spacing between the switch and the content.
#[property(CONTEXT, default(SWITCH_SPACING_VAR), widget_impl(SwitchStyle))]
pub fn switch_spacing(child: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, SWITCH_SPACING_VAR, spacing)
}

fn switch_visual(parent_hovered: impl Var<bool>) -> impl UiNode {
    zero_ui_wgt_container::Container! {
        hit_test_mode = false;
        size = (2.em(), 1.em());
        align = Align::CENTER;
        corner_radius = 1.em();
        padding = 2;
        child = Wgt! {
            size = 1.em() - Length::from(4);
            align = Align::LEFT;
            background_color = zero_ui_wgt_text::FONT_COLOR_VAR;

            #[easing(150.ms())]
            x = 0.em();
            when *#is_checked {
                x = 1.em();
            }
        };

        #[easing(150.ms())]
        background_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
        when *#{parent_hovered} {
            #[easing(0.ms())]
            background_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()));
        }
    }
}

/// Radio toggle style.
///
/// Style a [`Toggle!`] widget to look like a *radio button*.
///
/// [`Toggle!`]: struct@Toggle
#[widget($crate::RadioStyle)]
pub struct RadioStyle(Style);
impl RadioStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            access_role = AccessRole::Radio;
            child_insert_start = {
                insert: {
                    let parent_hovered = var(false);
                    is_hovered(radio_visual(parent_hovered.clone()), parent_hovered)
                },
                spacing: RADIO_SPACING_VAR,
            };
        }
    }
}

context_var! {
    /// Spacing between the radio and the content.
    pub static RADIO_SPACING_VAR: Length = 2;
}

/// Spacing between the radio and the content.
#[property(CONTEXT, default(RADIO_SPACING_VAR), widget_impl(RadioStyle))]
pub fn radio_spacing(child: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, RADIO_SPACING_VAR, spacing)
}

fn radio_visual(parent_hovered: impl Var<bool>) -> impl UiNode {
    Wgt! {
        hit_test_mode = false;
        size = 0.9.em();
        corner_radius = 0.9.em();
        align = Align::CENTER;
        border_align = 100.pct();

        #[easing(150.ms())]
        background_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
        when *#{parent_hovered} {
            #[easing(0.ms())]
            background_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()));
        }

        when *#is_checked {
            border = {
                widths: 2,
                sides: zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()).into()),
            };
            #[easing(0.ms())]
            background_color = zero_ui_wgt_text::FONT_COLOR_VAR;
        }
    }
}
