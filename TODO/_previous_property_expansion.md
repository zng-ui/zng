```rust
// Recursive expansion of property macro
// ======================================

#[doc = "<strong title='This function is also a widget property'><code>P</code></strong>   Customizes the widget order during TAB navigation."]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {

}
#[doc = "<strong title='This function is also a widget property'><code>P</code></strong>   Customizes the widget order during TAB navigation."]
#[derive(std::clone::Clone)]
#[allow(non_camel_case_types)]
pub struct tab_index {
    __instance__: zero_ui::core::widget_builder::PropertyInstInfo,
    tab_index: zero_ui::core::var::BoxedVar<TabIndex>,
}
impl tab_index {
    pub const ALLOWED_IN_WHEN_EXPR: bool = true;
    pub const ALLOWED_IN_WHEN_ASSIGN: bool = true;
    #[doc(hidden)]
    pub fn __id__(name: &'static str) -> zero_ui::core::widget_builder::PropertyId {
        static impl_id: zero_ui::core::widget_builder::StaticPropertyImplId =
            zero_ui::core::widget_builder::StaticPropertyImplId::new_unique();
        zero_ui::core::widget_builder::PropertyId {
            impl_id: impl_id.get(),
            name,
        }
    }
    #[doc(hidden)]
    pub fn __property__() -> zero_ui::core::widget_builder::PropertyInfo {
        zero_ui::core::widget_builder::PropertyInfo {
            group: {
                use zero_ui::core::widget_builder::nest_group_items::*;
                CONTEXT
            },
            capture: false,
            impl_id: Self::__id__("").impl_id,
            name: std::stringify!(tab_index),
            location: zero_ui::core::widget_builder::source_location!(),
            default: Some(Self::__default__),
            new: Self::__new_dyn__,
            inputs: std::boxed::Box::new([zero_ui::core::widget_builder::PropertyInput {
                name: stringify!(tab_index),
                kind: zero_ui::core::widget_builder::InputKind::Var,
                ty: std::any::TypeId::of::<TabIndex>(),
                ty_name: std::any::type_name::<TabIndex>(),
            }]),
        }
    }
    #[doc(hidden)]
    pub const fn __input_types__() -> zero_ui::core::widget_builder::PropertyInputTypes<(zero_ui::core::var::BoxedVar<TabIndex>,)> {
        zero_ui::core::widget_builder::PropertyInputTypes::unit()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn __new__(tab_index: impl IntoVar<TabIndex>) -> Self {
        Self {
            __instance__: zero_ui::core::widget_builder::PropertyInstInfo::none(),
            tab_index: Self::tab_index(tab_index),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn __new_sorted__(tab_index: impl IntoVar<TabIndex>) -> Self {
        Self::__new__(tab_index)
    }
    pub fn __new_dyn__(
        __args__: zero_ui::core::widget_builder::PropertyNewArgs,
    ) -> std::boxed::Box<dyn zero_ui::core::widget_builder::PropertyArgs> {
        let mut __inputs__ = __args__.args.into_iter();
        Box::new(Self {
            __instance__: __args__.inst_info,
            tab_index: {
                let __actions__ = zero_ui::core::widget_builder::iter_input_build_actions(
                    &__args__.build_actions,
                    &__args__.build_actions_when_data,
                    0usize,
                );
                zero_ui::core::widget_builder::new_dyn_var(&mut __inputs__, __actions__)
            },
        })
    }
    pub fn __build__(
        mut self,
        info: zero_ui::core::widget_builder::PropertyInstInfo,
    ) -> std::boxed::Box<dyn zero_ui::core::widget_builder::PropertyArgs> {
        self.__instance__ = info;
        Box::new(self)
    }
    pub fn __default__(
        info: zero_ui::core::widget_builder::PropertyInstInfo,
    ) -> std::boxed::Box<dyn zero_ui::core::widget_builder::PropertyArgs> {
        Self::__new__(TabIndex::default()).__build__(info)
    }
    pub fn __default_fn__() -> std::option::Option<
        fn(info: zero_ui::core::widget_builder::PropertyInstInfo) -> std::boxed::Box<dyn zero_ui::core::widget_builder::PropertyArgs>,
    > {
        Some(Self::__default__)
    }
    pub fn tab_index(tab_index: impl IntoVar<TabIndex>) -> zero_ui::core::var::BoxedVar<TabIndex> {
        zero_ui::core::widget_builder::var_to_args(tab_index)
    }
    pub fn __w_tab_index__() -> (zero_ui::core::widget_builder::WhenInputVar, impl zero_ui::core::var::Var<TabIndex>) {
        zero_ui::core::widget_builder::WhenInputVar::new::<TabIndex>()
    }
    pub fn __w_0__() -> (zero_ui::core::widget_builder::WhenInputVar, impl zero_ui::core::var::Var<TabIndex>) {
        zero_ui::core::widget_builder::WhenInputVar::new::<TabIndex>()
    }
}
impl zero_ui::core::widget_builder::PropertyArgs for tab_index {
    fn clone_boxed(&self) -> std::boxed::Box<dyn zero_ui::core::widget_builder::PropertyArgs> {
        Box::new(std::clone::Clone::clone(self))
    }
    fn property(&self) -> zero_ui::core::widget_builder::PropertyInfo {
        Self::__property__()
    }
    fn instance(&self) -> zero_ui::core::widget_builder::PropertyInstInfo {
        std::clone::Clone::clone(&self.__instance__)
    }
    fn instantiate(&self, __child__: zero_ui::core::widget_instance::BoxedUiNode) -> zero_ui::core::widget_instance::BoxedUiNode {
        let __node__ = tab_index(__child__, std::clone::Clone::clone(&self.tab_index));
        zero_ui::core::widget_instance::UiNode::boxed(__node__)
    }
    fn var(&self, __index__: usize) -> &dyn zero_ui::core::var::AnyVar {
        match __index__ {
            0usize => &self.tab_index,
            n => zero_ui::core::widget_builder::panic_input(&self.property(), n, zero_ui::core::widget_builder::InputKind::Var),
        }
    }
}
```