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
    use super::*;

    #[doc(inline)]
    pub use super::properties::{self, selection, IsCheckedVar, SingleOptSel, SingleSel};
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
        properties::checked_opt;

        /// Values that is selected in the contextual [`selection`].
        ///
        /// The widget [`is_checked`] when the value is selected, on click and on value update the selection
        /// is updated according to the behavior defined in the contextual [`selection`]. If no contextual
        /// [`selection`] is the the widget is never checked.
        ///
        /// Note that the value can be any type, but must be one of the types accepted by the contextual [`selection`], type
        /// validation happens in run-time, an error is logged if the type is not compatible. Because any type can be used in
        /// this property type inference cannot resolve the type automatically and a *turbofish* annotation is required: `value::<T> = t;`.
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
        ///             value::<i32> = i;
        ///         }
        ///         .boxed_wgt()
        ///     }).collect::<WidgetVec>();
        /// }
        /// ```
        ///
        /// [`is_checked`]: #wp-is_checked
        /// [`selection`]: fn@selection
        properties::value;

        /// Enables `None` as an input value.
        ///
        /// Note that `None` is always accepted in `checked_opt`, this property controls if
        /// `None` is one of the values in the toggle cycle. If the widget is bound to the `checked` property
        /// this config is ignored.
        ///
        /// This is not enabled by default.
        properties::tristate = properties::IsTristateVar;

        /// If the toggle is checked from any of the three primary properties.
        ///
        /// Note to read the tristate use [`IsCheckedVar`].
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
        ///     // value::<i32> = 42;
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

        /// Toggle dark and light themes.
        ///
        /// Set to [`theme::pair`] of [`vis::DarkThemeVar`], [`vis::LightThemeVar`] by default.
        theme = theme::pair(vis::DarkThemeVar, vis::LightThemeVar);
    }

    fn new_context_dyn(child: impl UiNode, part: DynWidgetPart, tristate: impl IntoVar<bool>) -> impl UiNode {
        // ensure that the context var is set for other contexts.
        let child = properties::tristate(child, tristate);
        themable::new_context_dyn(child, part)
    }
}

/// Properties used in the toggle widget.
pub mod properties {
    use std::{any::Any, cell::RefCell, error::Error, fmt, marker::PhantomData, rc::Rc};

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

    /// Selects `value` on click and sets [`IsCheckedVar`] if the `value` is selected.
    ///
    /// This property interacts with the contextual [`selection`], when the widget is clicked or the `value` variable changes
    /// the contextual [`Selector`] is used to implement the behavior.
    #[property(context)]
    pub fn value<T: VarValue + PartialEq>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode {
        struct ValueNode<C, T, V> {
            child: C,
            value: V,
            checked: RcVar<Option<bool>>,
            _type: PhantomData<T>,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, T: VarValue + PartialEq, V: Var<T>> UiNode for ValueNode<C, T, V> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                let value = self.value.get(ctx.vars);
                let selected = SelectorVar::get(ctx.vars).instance.borrow().is_selected(&mut ctx.as_info(), value);
                self.checked.set_ne(ctx.vars, Some(selected));

                self.child.init(ctx);
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                if self.checked.copy(ctx.vars) == Some(true) {
                    // TODO, (de)select?
                }
                self.child.deinit(ctx);
            }

            fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                SelectorVar::get(ctx.vars).instance.borrow().subscribe(ctx, subs);
                subs.var(ctx, &self.value);
                subs.event(ClickEvent);
                self.child.subscriptions(ctx, subs);
            }

            fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                if let Some(args) = ClickEvent.update(args) {
                    self.child.event(ctx, args);

                    if args.is_primary() && !args.propagation().is_stopped() && args.is_enabled(ctx.path.widget_id()) {
                        args.propagation().stop();

                        let selected = self.checked.copy(ctx) == Some(true);
                        let value = self.value.get(ctx.vars);
                        let mut selector = SelectorVar::get(ctx.vars).instance.borrow_mut();
                        let r = if selected {
                            selector.deselect(ctx, value)
                        } else {
                            selector.select(ctx, Box::new(value.clone()))
                        };
                        match r {
                            Ok(()) => self.checked.set(ctx, Some(!selected)),
                            Err(e) => match e {
                                SelectorError::ReadOnly => {}
                                e => {
                                    self.checked.set_ne(ctx, None);
                                    tracing::error!("failed to {}select `{:?}`, {}", if selected { "de" } else { "" }, value, e);
                                }
                            },
                        }
                    }
                } else {
                    self.child.event(ctx, args)
                }
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                let value = self.value.get(ctx.vars);
                let selected = SelectorVar::get(ctx.vars).instance.borrow().is_selected(&mut ctx.as_info(), value);
                self.checked.set_ne(ctx.vars, selected);

                self.child.update(ctx);
            }
        }
        let checked = var(Some(false));
        let child = with_context_var(child, IsCheckedVar, checked.clone());
        ValueNode {
            child,
            value: value.into_var(),
            checked,
            _type: PhantomData::<T>,
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
        with_context_var(child, SelectorVar, SelectorInstance::new(selector))
    }

    context_var! {
        struct SelectorVar: SelectorInstance = SelectorInstance::new(NilSel);
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
        /// Subscribe the selector in a [`value`] widget to receive selection updates.
        ///
        /// The [`value`] property checks [`is_selected`] every update.
        ///
        /// [`value`]: fn@value
        /// [`is_selected`]: Selector::is_selected
        fn subscribe(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions);

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
                SelectorError::Custom(e) => fmt::Display::fmt(e, f),
            }
        }
    }
    impl Error for SelectorError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                SelectorError::WrongType => None,
                SelectorError::ReadOnly => None,
                SelectorError::Custom(e) => Some(&**e),
            }
        }
    }
    impl From<VarIsReadOnly> for SelectorError {
        fn from(_: VarIsReadOnly) -> Self {
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
        fn subscribe(&self, _: &mut InfoContext, _: &mut WidgetSubscriptions) {}

        fn select(&mut self, _: &mut WidgetContext, _: Box<dyn Any>) -> Result<(), SelectorError> {
            Err(SelectorError::custom_str("no selection enabled"))
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
        fn subscribe(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.target);
        }

        fn select(&mut self, ctx: &mut WidgetContext, value: Box<dyn Any>) -> Result<(), SelectorError> {
            match value.downcast::<T>() {
                Ok(value) => match self.target.set_ne(ctx, *value) {
                    Ok(_) => Ok(()),
                    Err(VarIsReadOnly) => Err(SelectorError::ReadOnly),
                },
                Err(_) => Err(SelectorError::WrongType),
            }
        }

        fn deselect(&mut self, ctx: &mut WidgetContext, value: &dyn Any) -> Result<(), SelectorError> {
            if self.is_selected(&mut ctx.as_info(), value) {
                Err(SelectorError::custom_str("cannot unset selection"))
            } else {
                Ok(())
            }
        }

        fn is_selected(&self, ctx: &mut InfoContext, value: &dyn Any) -> bool {
            match value.downcast_ref::<T>() {
                Some(value) => self.target.get(ctx) == value,
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
        fn subscribe(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.target);
        }

        fn select(&mut self, ctx: &mut WidgetContext, value: Box<dyn Any>) -> Result<(), SelectorError> {
            match value.downcast::<T>() {
                Ok(value) => match self.target.set_ne(ctx, Some(*value)) {
                    Ok(_) => Ok(()),
                    Err(VarIsReadOnly) => Err(SelectorError::ReadOnly),
                },
                Err(value) => match value.downcast::<Option<T>>() {
                    Ok(value) => match self.target.set_ne(ctx, *value) {
                        Ok(_) => Ok(()),
                        Err(VarIsReadOnly) => Err(SelectorError::ReadOnly),
                    },
                    Err(_) => Err(SelectorError::WrongType),
                },
            }
        }

        fn deselect(&mut self, ctx: &mut WidgetContext, value: &dyn Any) -> Result<(), SelectorError> {
            match value.downcast_ref::<T>() {
                Some(value) => {
                    if self.target.get(ctx).as_ref() == Some(value) {
                        match self.target.set(ctx, None) {
                            Ok(_) => Ok(()),
                            Err(VarIsReadOnly) => Err(SelectorError::ReadOnly),
                        }
                    } else {
                        Ok(())
                    }
                }
                None => match value.downcast_ref::<Option<T>>() {
                    Some(value) => {
                        if self.target.get(ctx) == value {
                            if value.is_none() {
                                Ok(())
                            } else {
                                match self.target.set(ctx, None) {
                                    Ok(_) => Ok(()),
                                    Err(VarIsReadOnly) => Err(SelectorError::ReadOnly),
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

        fn is_selected(&self, ctx: &mut InfoContext, value: &dyn Any) -> bool {
            match value.downcast_ref::<T>() {
                Some(value) => self.target.get(ctx).as_ref() == Some(value),
                None => match value.downcast_ref::<Option<T>>() {
                    Some(value) => self.target.get(ctx) == value,
                    None => false,
                },
            }
        }
    }
}

/// Toggle themes, visual properties and context vars.
pub mod vis {
    use super::*;

    use crate::widgets::button::vis as btn_vis;

    /// Default toggle dark theme.
    #[widget($crate::widgets::toggle::vis::dark_theme)]
    pub mod dark_theme {
        use super::*;

        inherit!(btn_vis::dark_theme);

        properties! {
            properties::is_checked;

            /// When the toggle is checked.
            when self.is_checked  {
                background_color = btn_vis::DarkColorVar::pressed();
                border = {
                    widths: 1,
                    sides: btn_vis::DarkColorVar::pressed().map_into(),
                };
            }
        }
    }

    /// Default toggle light theme.
    #[widget($crate::widgets::toggle::vis::light_theme)]
    pub mod light_theme {
        use super::*;

        inherit!(btn_vis::light_theme);

        properties! {
            properties::is_checked;

            /// When the toggle is checked.
            when self.is_checked  {
                background_color = btn_vis::LightColorVar::pressed();
                border = {
                    widths: 1,
                    sides: btn_vis::LightColorVar::pressed().map_into(),
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
