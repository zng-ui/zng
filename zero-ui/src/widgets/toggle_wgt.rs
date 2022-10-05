use crate::prelude::new_widget::*;

/// A toggle button that flips a `bool` or `Option<bool>` variable on click, or selects a value.
///
/// This widget has three primary properties, [`checked`], [`checked_opt`] and [`value`], setting one
/// of the checked properties to a read-write variable enables the widget and it will set the variables
/// on click, setting [`value`] turns the toggle in a selection item that sets a contextual [`selection`].
///
/// [`checked`]: #wp-checked
/// [`checked_opt`]: #wp-checked_opt
/// [`value`]: #wp-value
/// [`selection`]: toggle::selection
#[widget($crate::widgets::toggle)]
pub mod toggle {
    #[doc(inline)]
    pub use super::properties::{self, selection, SingleOptSel, SingleSel, IS_CHECKED_VAR};
    #[doc(inline)]
    pub use super::vis;

    inherit!(crate::widgets::button);

    properties! {
        remove { on_click }

        /// Toggle cycles between `true` and `false`, updating the variable.
        ///
        /// # Examples
        ///
        /// The variable `foo` is toggled on click and it also controls the checked state of the widget.
        ///
        /// ```
        /// # use zero_ui::prelude::*;
        /// let foo = var(false);
        ///
        /// toggle! {
        ///     checked = foo.clone();
        ///
        ///     content = text(foo.map(|b| formatx!("foo = {b}")));
        /// }
        /// # ;
        /// ```
        ///
        /// Note that you can read the checked state of the widget using [`is_checked`].
        ///
        /// [`is_checked`]: #wp-is_checked
        properties::checked;

        /// Toggle cycles between `Some(true)` and `Some(false)` and accepts `None`, if the
        /// widget is `tristate` also sets to `None` in the toggle cycle.
        ///
        /// # Examples
        ///
        /// The variable `foo` is cycles the three states on click.
        ///
        /// ```
        /// # use zero_ui::prelude::*;
        /// let foo = var(Some(false));
        ///
        /// toggle! {
        ///     checked_opt = foo.clone();
        ///     tristate = true;
        ///
        ///     content = text(foo.map(|b| formatx!("foo = {b:?}")));
        /// }
        /// # ;
        /// ```
        #[priority_index = 999] // force property to be inside tristate.
        properties::checked_opt;

        /// Values that is selected in the contextual [`selection`].
        ///
        /// The widget [`is_checked`] when the value is selected, on click and on value update the selection
        /// is updated according to the behavior defined in the contextual [`selection`]. If no contextual
        /// [`selection`] is the the widget is never checked.
        ///
        /// Note that the value can be any type, but must be one of the types accepted by the contextual [`selection`], type
        /// validation happens in run-time, an error is logged if the type is not compatible. Because any type can be used in
        /// this property type inference cannot resolve the type automatically and a type annotation is required: `value<T> = t;`.
        ///
        /// # Examples
        ///
        /// The variable `foo` is set to a `value` clone on click, or if the `value` updates when the previous was selected.
        ///
        /// ```
        /// # use zero_ui::prelude::*;
        /// let foo = var(1_i32);
        ///
        /// v_stack! {
        ///     toggle::selection = toggle::SingleSel::new(foo.clone());
        ///
        ///     spacing = 5;
        ///     items = (1..=10_i32).map(|i| {
        ///         toggle! {
        ///             content = text(formatx!("Item {i}"));
        ///             value<i32> = i;
        ///         }
        ///         .boxed_wgt()
        ///     }).collect::<WidgetVec>();
        /// }
        /// # ;
        /// ```
        ///
        /// [`is_checked`]: #wp-is_checked
        /// [`selection`]: fn@selection
        #[priority_index = 999] // force property to be inside select_on_init and others.
        properties::value;

        /// Enables `None` as an input value.
        ///
        /// Note that `None` is always accepted in `checked_opt`, this property controls if
        /// `None` is one of the values in the toggle cycle. If the widget is bound to the `checked` property
        /// this config is ignored.
        ///
        /// This is not enabled by default.
        properties::tristate;

        /// If [`value`] is selected when the widget is inited.
        ///
        /// [`value`]: #wp-value
        properties::select_on_init;

        /// If the toggle is checked from any of the three primary properties.
        ///
        /// Note to read the tristate use [`IS_CHECKED_VAR`].
        ///
        /// # Examples
        ///
        /// The `is_checked` state is set when the [`checked`] is `true`, or [`checked_opt`] is `Some(true)` or the [`value`]
        /// is selected.
        ///
        /// ```
        /// # use zero_ui::prelude::*;
        /// toggle! {
        ///     checked = var(false);
        ///     // checked_opt = var(Some(false));
        ///     // value<i32> = 42;
        ///
        ///     content = text("Toggle Background");
        ///     background_color = colors::RED;
        ///     when self.is_checked {
        ///         background_color = colors::GREEN;
        ///     }
        /// }
        /// # ;
        /// ```
        ///
        /// [`checked`]: #wp-checked
        /// [`checked_opt`]: #wp-checked_opt
        /// [`value`]: #wp-value
        properties::is_checked;

        /// Toggle style.
        ///
        /// Set to [`vis::STYLE_VAR`] by default.
        style = vis::STYLE_VAR;
    }
}

/// Properties used in the toggle widget.
pub mod properties {
    use std::{any::Any, cell::RefCell, error::Error, fmt, marker::PhantomData, rc::Rc};

    use crate::prelude::new_property::*;

    context_var! {
        /// The toggle button checked state.
        pub static IS_CHECKED_VAR: Option<bool> = false;

        /// If toggle button cycles between `None`, `Some(false)` and `Some(true)` on click.
        pub static IS_TRISTATE_VAR: bool = false;
    }

    /// Toggle `checked` on click and sets the [`IsCheckedVar`], disables the widget if `checked` is read-only.
    #[property(context, default(false))]
    pub fn checked(child: impl UiNode, checked: impl IntoVar<bool>) -> impl UiNode {
        struct CheckedNode<C, B> {
            child: C,
            checked: B,
            click_handle: Option<EventWidgetHandle>,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, B: Var<bool>> UiNode for CheckedNode<C, B> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.click_handle = Some(CLICK_EVENT.subscribe(ctx.path.widget_id()));
                self.child.init(ctx);
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.child.deinit(ctx);
            }

            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                self.child.event(ctx, update);
                if let Some(args) = CLICK_EVENT.on(update) {
                    if args.is_primary()
                        && self.checked.capabilities().contains(VarCapabilities::MODIFY)
                        && !args.propagation().is_stopped()
                        && args.is_enabled(ctx.path.widget_id())
                    {
                        args.propagation().stop();

                        let _ = self.checked.modify(ctx, |c| *c.get_mut() = !*c.get());
                    }
                }
            }
        }

        let checked = checked.into_var();
        let node = CheckedNode {
            child: child.cfg_boxed(),
            checked: checked.clone(),
            click_handle: None,
        }
        .cfg_boxed();
        with_context_var(node, IS_CHECKED_VAR, checked.map_into())
    }

    /// Three state toggle `checked` on click and sets the [`IS_CHECKED_VAR`], disables the widget if `checked` is read-only.
    ///
    /// Sets to `None` if [`IsTristateVar`] is `true`.
    #[property(context, default(None))]
    pub fn checked_opt(child: impl UiNode, checked: impl IntoVar<Option<bool>>) -> impl UiNode {
        struct CheckedOptNode<C, B> {
            child: C,
            checked: B,
            click_handle: Option<EventWidgetHandle>,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, B: Var<Option<bool>>> UiNode for CheckedOptNode<C, B> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.click_handle = Some(CLICK_EVENT.subscribe(ctx.path.widget_id()));
                self.child.init(ctx);
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.child.deinit(ctx);
            }

            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                self.child.event(ctx, update);
                if let Some(args) = CLICK_EVENT.on(update) {
                    if args.is_primary()
                        && !self.checked.capabilities().contains(VarCapabilities::MODIFY)
                        && !args.propagation().is_stopped()
                        && args.is_enabled(ctx.path.widget_id())
                    {
                        args.propagation().stop();

                        if IS_TRISTATE_VAR.get() {
                            let _ = self.checked.modify(ctx, |c| {
                                *c.get_mut() = match *c.get() {
                                    Some(true) => None,
                                    Some(false) => Some(true),
                                    None => Some(false),
                                }
                            });
                        } else {
                            let _ = self.checked.modify(ctx, |c| {
                                *c.get_mut() = match *c.get() {
                                    Some(true) | None => Some(false),
                                    Some(false) => Some(true),
                                }
                            });
                        }
                    }
                }
            }
        }

        let checked = checked.into_var();
        let node = CheckedOptNode {
            child: child.cfg_boxed(),
            checked: checked.clone(),
            click_handle: None,
        }
        .cfg_boxed();

        with_context_var(node, IS_CHECKED_VAR, checked)
    }

    /// Enables `None` as an input on toggle.
    ///
    /// If the toggle button is checking using [`checked_opt`] and this is enabled the toggle cycles between `None`, `Some(false)` and `Some(true)`.
    #[property(context, default(IS_TRISTATE_VAR))]
    pub fn tristate(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, IS_TRISTATE_VAR, enabled)
    }

    /// If [`IS_CHECKED_VAR`] is `Some(true)`.
    #[property(event)]
    pub fn is_checked(child: impl UiNode, state: StateVar) -> impl UiNode {
        bind_state(child, IS_CHECKED_VAR.map(|s| *s == Some(true)), state)
    }

    /// Selects `value` on click and sets [`IS_CHECKED_VAR`] if the `value` is selected.
    ///
    /// This property interacts with the contextual [`selection`], when the widget is clicked or the `value` variable changes
    /// the contextual [`Selector`] is used to implement the behavior.
    #[property(context, allowed_in_when = false)]
    pub fn value<T: VarValue + PartialEq>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
        #[impl_ui_node(struct ValueNode<T: VarValue + PartialEq> {
            child: impl UiNode,
            value: impl Var<T>,
            checked: RcVar<Option<bool>>,
            prev_value: Option<T>,
            click_handle: Option<EventWidgetHandle>,
        })]
        impl ValueNode {
            // Returns `true` if selected.
            fn select(ctx: &mut WidgetContext, value: &T) -> bool {
                SELECTOR_VAR.with(|selector| {
                    let mut selector = selector.instance.borrow_mut();

                    match selector.select(ctx, Box::new(value.clone())) {
                        Ok(()) => true,
                        Err(e) => {
                            let selected = selector.is_selected(&mut ctx.as_info(), value);
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
                })
            }

            /// Returns `true` if deselected.
            fn deselect(ctx: &mut WidgetContext, value: &T) -> bool {
                SELECTOR_VAR.with(|selector| {
                    let mut selector = selector.instance.borrow_mut();
                    match selector.deselect(ctx, value) {
                        Ok(()) => true,
                        Err(e) => {
                            let deselected = !selector.is_selected(&mut ctx.as_info(), value);
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
                })
            }

            fn is_selected(ctx: &mut WidgetContext, value: &T) -> bool {
                SELECTOR_VAR.with(|sel| sel.instance.borrow().is_selected(&mut ctx.as_info(), value))
            }

            #[UiNode]
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.sub_var(&self.value).sub_var(&SELECTOR_VAR).sub_var(&DESELECT_ON_NEW_VAR);
                SELECTOR_VAR.with(|s| s.instance.borrow().subscribe(&mut ctx.handles));

                self.value.with(|value| {
                    let selected = if SELECT_ON_INIT_VAR.get() {
                        Self::select(ctx, value)
                    } else {
                        Self::is_selected(ctx, value)
                    };
                    self.checked.set_ne(ctx.vars, Some(selected)).unwrap();

                    if DESELECT_ON_DEINIT_VAR.get() {
                        self.prev_value = Some(value.clone());
                    }
                });

                self.click_handle = Some(CLICK_EVENT.subscribe(ctx.path.widget_id()));

                self.child.init(ctx);
            }

            #[UiNode]
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                if self.checked.get() == Some(true) && DESELECT_ON_DEINIT_VAR.get() {
                    self.value.with(|value| {
                        if Self::deselect(ctx, value) {
                            self.checked.set_ne(ctx, Some(false)).unwrap();
                        }
                    });
                }

                self.prev_value = None;
                self.click_handle = None;

                self.child.deinit(ctx);
            }

            #[UiNode]
            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                self.child.event(ctx, update);
                if let Some(args) = CLICK_EVENT.on(update) {
                    if args.is_primary() && !args.propagation().is_stopped() && args.is_enabled(ctx.path.widget_id()) {
                        args.propagation().stop();

                        let selected = self.value.with(|value| {
                            let selected = self.checked.get() == Some(true);
                            if selected {
                                !Self::deselect(ctx, value)
                            } else {
                                Self::select(ctx, value)
                            }
                        });
                        self.checked.set_ne(ctx, Some(selected)).unwrap()
                    }
                }
            }

            #[UiNode]
            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if SELECTOR_VAR.is_new(ctx) {
                    todo!("reload widget?")
                }

                let selected = self.value.with_new(ctx.vars, |new| {
                    // auto select new.
                    let selected = if self.checked.get() == Some(true) && SELECT_ON_NEW_VAR.get() {
                        Self::select(ctx, new)
                    } else {
                        Self::is_selected(ctx, new)
                    };

                    // auto deselect prev, need to be done after potential auto select new to avoid `CannotClear` error.
                    if let Some(prev) = self.prev_value.take() {
                        if DESELECT_ON_NEW_VAR.get() {
                            Self::deselect(ctx, &prev);
                            self.prev_value = Some(new.clone());
                        }
                    }

                    selected
                });
                let selected = selected.unwrap_or_else(|| {
                    // contextual selection can change in any update.
                    self.value.with(|val| Self::is_selected(ctx, val))
                });
                self.checked.set_ne(ctx.vars, selected).unwrap();

                if DESELECT_ON_NEW_VAR.get() && selected {
                    // save a clone of the value to reference it on deselection triggered by variable value changing.
                    if self.prev_value.is_none() {
                        self.prev_value = Some(self.value.get());
                    }
                } else {
                    self.prev_value = None;
                }

                self.child.update(ctx, updates);
            }
        }
        let checked = var(Some(false));
        let child = with_context_var(child, IS_CHECKED_VAR, checked.clone());
        ValueNode {
            child,
            value: value.into_var(),
            checked,
            prev_value: None,
            click_handle: None,
        }
    }

    /// Sets the contextual selection target that all inner widgets will target from the [`value`] property.
    ///
    /// All [`value`] properties declared in widgets inside `child` will call the methods of the [`Selector`] interface to manipulate
    /// the selection, some common selection patterns are provided, see [`SingleSel`], [`SingleOptSel`].
    ///
    /// Selection in a context can be blocked by setting the selector to [`NilSel`], this is also the default selector so the [`value`]
    /// property only works if a contextual selection is present.
    ///
    /// [`value`]: fn@value
    #[property(context, allowed_in_when = false, default(NilSel))]
    pub fn selection(child: impl UiNode, selector: impl Selector) -> impl UiNode {
        with_context_var(child, SELECTOR_VAR, SelectorInstance::new(selector))
    }

    /// If [`value`] is selected when the widget that has the value is inited.
    ///
    /// [`value`]: fn@value
    #[property(context, default(SELECT_ON_INIT_VAR))]
    pub fn select_on_init(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, SELECT_ON_INIT_VAR, enabled)
    }

    /// If [`value`] is deselected when the widget that has the value is deinited and the value was selected.
    ///
    /// [`value`]: fn@value
    #[property(context, default(DESELECT_ON_DEINIT_VAR))]
    pub fn deselect_on_deinit(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, DESELECT_ON_DEINIT_VAR, enabled)
    }

    /// If [`value`] selects the new value when the variable changes and the previous value was selected.
    ///
    /// [`value`]: fn@value
    #[property(context, default(SELECT_ON_NEW_VAR))]
    pub fn select_on_new(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, SELECT_ON_NEW_VAR, enabled)
    }

    /// If [`value`] deselects the previously selected value when the variable changes.
    ///
    /// [`value`]: fn@value
    #[property(context, default(DESELECT_ON_NEW_VAR))]
    pub fn deselect_on_new(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, DESELECT_ON_NEW_VAR, enabled)
    }

    context_var! {
        static SELECTOR_VAR: SelectorInstance = SelectorInstance::new(NilSel);

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
    }

    #[derive(Clone)]
    struct SelectorInstance {
        instance: Rc<RefCell<Box<dyn Selector>>>,
    }
    impl fmt::Debug for SelectorInstance {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("SelectorInstance").finish_non_exhaustive()
        }
    }
    impl SelectorInstance {
        pub fn new<S: Selector>(selector: S) -> Self {
            SelectorInstance {
                instance: Rc::new(RefCell::new(Box::new(selector))),
            }
        }
    }

    /// Represents the contextual selection behavior of [`value`] selection.
    ///
    /// A selector can be set using [`selection`], all [`value`] widgets in context will target it.
    ///
    /// [`value`]: fn@value
    /// [`selection`]: fn@selection
    pub trait Selector: 'static {
        /// Add the selector subscriptions for a widget.
        fn subscribe(&self, widget_id: WidgetId, handles: &mut WidgetHandles);

        /// Insert the `value` in the selection, returns `Ok(())` if the value was inserted or was already selected.
        fn select(&mut self, ctx: &mut WidgetContext, value: Box<dyn Any>) -> Result<(), SelectorError>;

        /// Remove the `value` from the selection, returns `Ok(())` if the value was removed or was not selected.
        fn deselect(&mut self, ctx: &mut WidgetContext, value: &dyn Any) -> Result<(), SelectorError>;

        /// Returns `true` if the `value` is selected.
        fn is_selected(&self, ctx: &mut InfoContext, value: &dyn Any) -> bool;
    }

    /// Error for [`Selector`] operations.
    #[derive(Debug, Clone)]
    pub enum SelectorError {
        /// Cannot select item because it is not of the same type as the selection.
        WrongType,
        /// Cannot (de)select item because the selection is read-only.
        ReadOnly,
        /// Cannot deselect item because the selection cannot be empty.
        CannotClear,
        /// Cannot select item because of a selector specific reason.
        Custom(Rc<dyn Error + 'static>),
    }
    impl SelectorError {
        /// New custom error from string.
        pub fn custom_str(str: impl Into<String>) -> SelectorError {
            let str = str.into();
            let e: Box<dyn Error> = str.into();
            let e: Rc<dyn Error> = e.into();
            SelectorError::Custom(e)
        }
    }
    impl fmt::Display for SelectorError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SelectorError::WrongType => write!(f, "wrong value type for selection"),
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

    /// Represents no selection and the inability to select any item.
    ///  
    /// See [`selection`] for more details.
    ///
    /// [`selection`]: fn@selection
    pub struct NilSel;

    impl Selector for NilSel {
        fn subscribe(&self, _: WidgetId, _: &mut WidgetHandles) {}

        fn select(&mut self, _: &mut WidgetContext, _: Box<dyn Any>) -> Result<(), SelectorError> {
            Err(SelectorError::custom_str("no contextual `selection`"))
        }

        fn deselect(&mut self, _: &mut WidgetContext, _: &dyn Any) -> Result<(), SelectorError> {
            Ok(())
        }

        fn is_selected(&self, _: &mut InfoContext, __r: &dyn Any) -> bool {
            false
        }
    }

    /// Represents the "radio" selection of a single item.
    ///
    /// See [`selection`] for more details.
    ///
    /// [`selection`]: fn@selection
    pub struct SingleSel<T, S> {
        target: S,
        _type: PhantomData<T>,
    }
    impl<T, S> SingleSel<T, S>
    where
        T: VarValue + PartialEq,
        S: Var<T>,
    {
        /// New single selector that sets a `target` var with the selected value.
        pub fn new(target: S) -> Self {
            SingleSel {
                target,
                _type: PhantomData,
            }
        }
    }
    impl<T, S> Selector for SingleSel<T, S>
    where
        T: VarValue + PartialEq,
        S: Var<T>,
    {
        fn subscribe(&self, widget_id: WidgetId, handles: &mut WidgetHandles) {
            handles.push_var(self.target.subscribe(widget_id));
        }

        fn select(&mut self, ctx: &mut WidgetContext, value: Box<dyn Any>) -> Result<(), SelectorError> {
            match value.downcast::<T>() {
                Ok(value) => match self.target.set_ne(ctx, *value) {
                    Ok(_) => Ok(()),
                    Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                },
                Err(_) => Err(SelectorError::WrongType),
            }
        }

        fn deselect(&mut self, ctx: &mut WidgetContext, value: &dyn Any) -> Result<(), SelectorError> {
            if self.is_selected(&mut ctx.as_info(), value) {
                Err(SelectorError::CannotClear)
            } else {
                Ok(())
            }
        }

        fn is_selected(&self, _: &mut InfoContext, value: &dyn Any) -> bool {
            match value.downcast_ref::<T>() {
                Some(value) => self.target.with(|t| t == value),
                None => false,
            }
        }
    }

    /// Represents the "radio" selection of a single item that is optional.
    ///
    /// See [`selection`] for more details.
    ///
    /// [`selection`]: fn@selection
    pub struct SingleOptSel<T, S> {
        target: S,
        _type: PhantomData<T>,
    }
    impl<T, S> SingleOptSel<T, S>
    where
        T: VarValue + PartialEq,
        S: Var<Option<T>>,
    {
        /// New single selector that sets a `target` var with the selected value or `None`.
        pub fn new(target: S) -> Self {
            SingleOptSel {
                target,
                _type: PhantomData,
            }
        }
    }
    impl<T, S> Selector for SingleOptSel<T, S>
    where
        T: VarValue + PartialEq,
        S: Var<Option<T>>,
    {
        fn subscribe(&self, widget_id: WidgetId, handles: &mut WidgetHandles) {
            handles.push_var(self.target.subscribe(widget_id));
        }

        fn select(&mut self, ctx: &mut WidgetContext, value: Box<dyn Any>) -> Result<(), SelectorError> {
            match value.downcast::<T>() {
                Ok(value) => match self.target.set_ne(ctx, Some(*value)) {
                    Ok(_) => Ok(()),
                    Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                },
                Err(value) => match value.downcast::<Option<T>>() {
                    Ok(value) => match self.target.set_ne(ctx, *value) {
                        Ok(_) => Ok(()),
                        Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                    },
                    Err(_) => Err(SelectorError::WrongType),
                },
            }
        }

        fn deselect(&mut self, ctx: &mut WidgetContext, value: &dyn Any) -> Result<(), SelectorError> {
            match value.downcast_ref::<T>() {
                Some(value) => {
                    if self.target.with(|t| t.as_ref() == Some(value)) {
                        match self.target.set(ctx, None) {
                            Ok(_) => Ok(()),
                            Err(VarIsReadOnlyError { .. }) => Err(SelectorError::ReadOnly),
                        }
                    } else {
                        Ok(())
                    }
                }
                None => match value.downcast_ref::<Option<T>>() {
                    Some(value) => {
                        if self.target.with(|t| t == value) {
                            if value.is_none() {
                                Ok(())
                            } else {
                                match self.target.set(ctx, None) {
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

        fn is_selected(&self, _: &mut InfoContext, value: &dyn Any) -> bool {
            match value.downcast_ref::<T>() {
                Some(value) => self.target.with(|t| t.as_ref() == Some(value)),
                None => match value.downcast_ref::<Option<T>>() {
                    Some(value) => self.target.with(|t| t == value),
                    None => false,
                },
            }
        }
    }
}

/// Toggle style, visual properties and context vars.
pub mod vis {
    use super::*;

    use crate::widgets::button::vis as btn_vis;

    context_var! {
        /// Toggle style in a context.
        ///
        /// Is the [`default_style!`] by default.
        ///
        /// [`default_style!`]: mod@default_style
        pub static STYLE_VAR: StyleGenerator = StyleGenerator::new(|_, _| default_style!());
    }

    /// Sets the toggle style in a context, the parent style is fully replaced.
    #[property(context, default(STYLE_VAR))]
    pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        with_context_var(child, STYLE_VAR, style)
    }

    /// Extends the toggle style in a context, the parent style is used, properties of the same name set in
    /// `style` override the parent style.
    #[property(context, default(StyleGenerator::nil()))]
    pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        styleable::with_style_extension(child, STYLE_VAR, style)
    }

    /// Default toggle style.
    ///
    /// Extends the [`button::vis::default_style`] to have the *pressed* look when [`is_checked`].
    ///
    /// [`button::vis::default_style`]: mod@crate::widgets::button::vis::default_style
    /// [`is_checked`]: fn@toggle::is_checked
    #[widget($crate::widgets::toggle::vis::default_style)]
    pub mod default_style {
        use super::*;

        inherit!(btn_vis::default_style);

        properties! {
            properties::is_checked;

            /// When the toggle is checked.
            when self.is_checked  {
                background_color = crate::widgets::button::vis::color_scheme_pressed(btn_vis::BASE_COLORS_VAR);
                border = {
                    widths: 1,
                    sides: crate::widgets::button::vis::color_scheme_pressed(btn_vis::BASE_COLORS_VAR).map_into(),
                };
            }
        }
    }
}

/// A checkbox toggle.
#[widget($crate::widgets::checkbox)]
pub mod checkbox {
    inherit!(super::toggle);

    pub use super::toggle::IS_CHECKED_VAR;

    use super::*;

    properties! {
        content_align = Align::LEFT;
        padding = 0;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }
}
