pub mod property_user_declaration {
    use crate::*;
    use crate::var::*;
    use crate::units::*;

    struct MarginNode<C: UiNode, M: Var<SideOffsets>> {
        child: C,
        #[allow(unused)]
        margin: M
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, M: Var<SideOffsets>> MarginNode<C, M> {

    }

    /// Property
    ///
    /// This function is a property, this is how to use it : TODO.
    #[property(outer)]
    pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
        MarginNode {
            child,
            margin: margin.into_var(),
        }
    }
}
// EXPANDS TO:
pub mod property_expanded {    

    /// Property
    ///
    /// This function is a property, this is how to use it : TODO.
    pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
        MarginNode {
            child,
            margin: margin.into_var(),
        }
    }

    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    pub struct margin_NamedArgs<TMargin: IntoVar<SideOffsets>> {
        pub _phantom: std::marker::PhantomData<()>, // if required
        pub margin: TMargin,
    }
    impl<TMargin: IntoVar<SideOffsets> + 'static> margin_NamedArgs<TMargin> {
        pub fn new(margin: TMargin) -> Self {
            Self { _phantom: std::marker::PhantomData,  margin }
        }
        pub fn set(self, child: impl crate::UiNode) -> impl crate::UiNode {
            // impl UiNode assert happens here also.
            margin(child, self.margin)
        }        
        pub fn args(self) -> impl margin_Args {
            self // self as a type that carries its generic types internally.
        }

        #[cfg(debug_assertions)] 
        pub fn set_debug(
            self, 
            child: std::boxed::Box<dyn crate::UiNode>, 
            property_name: &'static str,
            instance_location: crate::debug::SourceLocation,
            user_assigned: bool
        ) -> crate::debug::PropertyInfoNode {
            let dbg_args = Box::new([
                crate::debug::debug_var(crate::var::IntoVar::into_var(std::clone::Clone::clone(&self.margin))),
            ]);

            let node = crate::UiNode::boxed(self.set(child));

            crate::debug::PropertyInfoNode::new_v1(
                node,
                crate::debug::PropertyPriority::Outer,
                "margin",
                crate::debug::source_location!(),
                property_name,
                instance_location,
                &["margin"],
                dbg_args,
                user_assigned
            )
        }
    }

    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    pub trait margin_Args {
        type TMargin: IntoVar<SideOffsets>;

        fn margin(&self) -> &Self::TMargin;
    
        fn unwrap(self) -> Self::TMargin;
    }
    impl<TMargin: IntoVar<SideOffsets> + 'static> margin_Args for margin_NamedArgs<TMargin> {
        type TMargin = TMargin;

        fn margin(&self) -> &TMargin {
            &self.margin
        }

        // returns a tuple for multiple
        fn unwrap(self) -> TMargin {
            self.margin
        }
    }

    #[doc(hidden)]
    #[macro_export]// if is pub
    macro_rules! margin_df18a4960c9c4924b503e192adb095ca {
        (named_new $property_path:path { $($fields:tt)+ }) => {
            // in case the struct has other fields like _phantom:
            $property_path {
                _phantom: std::marker::PhantomData<()>,
                $($fields)+
            }
        };
        (set outer, $node:ident, $args:ident, $property_name:expr, $source_location:expr, $user_assigned:tt) => {
            // don't  really use cfg here?
            #[cfg(debug_assertions)] 
            let $node = $args.set_debug($node, $property_name, $source_location, $user_assigned);
            #[cfg(not(debug_assertions))]
            let $node = $args.set($node); 
        };
        (set $other:ident, $($ignore:tt)+) => { };
        (assert allowed_in_when, $msg:tt) => {
            // if property is not allowed in when, otherwise is blank. 
            std::compile_error!{$msg}
        };
        (assert !capture_only, $msg:tt) => {
            // if property is capture only.
            std::compile_error!{$msg}
        };
        (if pub, $($tt:tt)*) => {
            // pass the tokens if pub or pub(..)
            $($tt:tt)*
        };
        (switch $property_path:path, $idx:ident, $($arg_n:ident),+) => {
            {
                // one switch for each field, use cloning for idx, except for last.
                $(let $arg_n = $arg_n.unwrap();)+
                // let other_field = $switch_var!(std::clone::Clone::clone(&$idx), $($arg_n.0),+);
                let enabled = $crate::zero_ui_macros::switch_var!($idx, $($arg_n),+);
                $property_path::new(enabled)
            }
        };
    }
    #[doc(hidden)]
    pub mod margin {
        pub use super::{
            margin as export,
            margin_NamedArgs as NamedArgs,
            margin_Args as Args,
        };
        pub use crate::margin_df18a4960c9c4924b503e192adb095ca as code_gen;
    }
}

/*
* widget!
*/

pub mod widget_user_declaration {
    #[widget($crate::widgets::button)]
    pub mod button {
        // items can be declared at any order, some *macro!* items have some special meaning.

        // normal import.
        use crate::widgets::*;
        use crate::properties::*;

        // inherit import, includes all properties from inherited widget or mixin.
        // later inherit clauses override properties of previous clauses.
        inherit!(crate::core::widget_core::implicit_mixin);// always included, not needed
        inherit!(crate::focus::focusable_mixin);
        inherit!(container);// container is imported in use::crate::widgets::*;

        // properties! set the properties, it works almost like an widget
        // but with extra stuff.
        // multiple properties! items are permitted they are merged when building.
        properties! {
            // optional child { } context for properties that will be applied with extra priority.
            child {
                /// Docs for `padding` property that internally uses the `margin` *outer* property.
                /// but because it is applied with `child` priority it will be *outer* around the 
                /// 
                crate::layout::margin as padding = 10;
            }

            /// Docs for the `on_click` property.
            /// Users will not need to import this event to set on_click but it is not set by default.
            crate::gesture::on_click;

            enabled = false;// enabled is inherited from implicit_mixin.

            blink = true;// blind is imported from crate::properties::*;

            /// New *capture_only* property named `custom` and used by this widget. 
            custom: impl IntoVar<bool> = true;
            // you can also omit the custom *capture_only* property because it is defined in one of the new functions.

            /// New *capture_only* property named `custom` and used by this widget with multiple named fields. 
            custom_multi: { 
                field0: impl IntoVar<bool>, 
                field1: impl IntoVar<u8> 
            } = (true, 255);

            /// positional assign for multi.
            multi = false, 200;

            /// named assign for multi
            multi = {
                field0: false,
                field1: 200,
            } // ; optional in last property.
        }

        use crate::core::color::*;

        /// A custom item that is scoped in the button module, a real use case
        /// would be declaring static_var! keys.
        pub const BACKGROUND: Rgba = colors::GRAY;
        pub const BACKGROUND_FOCUSED: Rgba = colors::LIGHT_GRAY;

        // another properties! item. They are all merged during build, all property
        // named must be unique across all properties! items and properties! { child { } } sections.
        properties! {
            background_color = BACKGROUND;

            /// `required!` properties must be set by widget users.
            widget_node as content = required!;

            /// `unset!` removes an inherited property.
            inherited_property = unset!;

            /// properties! can contain when blocks.
            when self.is_focused {
                background_color = BACKGROUND_FOCUSED;
                // the when block can set any property declared in the widget or inherited.
            }
        }

        // Initializer for the inner most node of the widget, child properties
        // will be applied to the return node, then normal properties, then that goes to
        // the `new` function that build the widget outer wrap.
        //
        // This function must return `impl UiNode`, you don't need to import `UiNode`.
        //
        // This function is not required, if missing, the new_child of the last inherited widget is used.
        // If not widget is inherited the `zero_ui::core::widget_base::default_new_child` is used.
        pub fn new_child(content: impl Widget, custom: impl IntoVar<bool>) -> impl UiNode {
            SomeUiNode {
                custom,
                custom_multi
            }
        }

        // Initializer for the outer wrap of the widget.
        //
        // The first argument must be `$child : impl UiNode` in that the `impl UiNode` is required but you can
        // use any name, you also don't need to import `UiNode`.
        //
        // Next arguments are optional property captures, you name the property to capture and write a type that is compatible
        // with the property args .unwrap() result. Because you need property names you can't deconstruct in the signature.
        //
        // The return value can be any type, only requirement is that the return type must be explicitly written.
        //
        // The type does not need to implement `UiNode` or `Widget`, `Window` doesn't.
        pub fn new(child: impl UiNode, id: WidgetId, custom_multi: (impl IntoVar<bool>, impl IntoVar<u8>)) -> Buttom {
            Button {
                child,
                id,
                custom_multi
            }
        }
    }
}

pub mod widget_expanded {
    #[macro_export]
    macro_rules! button_df18a4960c9c4924b503e192adb095ca {
        // widget inherit! branch.
        (=> inherit { $widget_path:path; $($inherit_next:tt)* } $($already_inherited:tt)*) => {
            $crate::zero_ui_macros::widget_stage2! {
                => { $($inherit_next:tt)* }

                // tokens already inherited by the widget deriving from button!.
                // implicit_mixin! at least is already in here.
                $($already_inherited:tt)*
                
                // append our own tokens.
                button {
                    // full path to module.
                    module: $widget_path; 
                    // we are not mixin, if this is true the `new_child` and `new` functions are not included
                    // the rest is the same,
                    mixin: false;
                    
                    properties_child {
                        /// padding docs.
                        padding = default,
                        // + all child properties in order of appearance.
                    }
                    properties {
                        /// background_color docs.
                        background_color = default,
                        content = required,
                        on_click,
                        is_focused, // when state properties are reexported.
                        // + all normal properties in order of appearance.
                    }
                    whens {
                        /// when docs.
                        (is_focused) { background_color }
                    }

                    // captured properties for each new function.
                    new_child(content, custom),
                    new(id, custom_multi)
                }
            }
        };
        // widget new branch. widget_mixins don't have this branch.
        ($($tt:tt)*) => {
            /// every crate that exports widgets must call something in their lib.rs to generate zero_ui_macros.
            $crate::zero_ui_macros::widget_new! {
                widget_tt {
                    /// path to the module path in the declaration.
                    $crate::path::to::button;

                    properties_child {
                        // no docs here.
                        padding = default,
                        // + all child properties in order of appearance.
                    }
                    properties {
                        // no docs here.
                        background_color = default,
                        content = required,
                        on_click,
                        is_focused, // when state properties are reexported.
                        // + all normal properties in order of appearance.
                    }

                    whens {
                        // no docs here.
                        // first when, (<comma separated list of properties used in the expression>)
                        (is_focused) {
                            // comma separated list of properties affected by the when.
                            background_color,
                        }
                        // + all other when clauses in order of appearance.
                    }
                    // captured properties for each new function.
                    new_child(content, custom),
                    new(id, custom_multi)
                }
                new_tt {
                    $($tt)*
                }
            }
        };
    }
    pub use crate::button_df18a4960c9c4924b503e192adb095ca as button;

    /// widget attributes.
    /// custom widget sections docs.
    pub mod button {
        /// custom items get copied here.
        
        use crate::widgets::*;
        use crate::properties::*;
        
        pub const BACKGROUND: Rgba = colors::GRAY;

        // reexport each property as their name in widget.
        #[doc(hidden)]
        pub mod __properties {
            use super::*;

            pub use crate::layout::margin::export as padding;
            pub use background_color::export as background_color;

            // inherited, the path comes from the inherit! clause.
            pub use crate::widget_base::implicit_mixin::__properties::id;

            pub use on_click::export as on_click;

            pub use is_focused::export as is_focused;// when state properties are also reexported.

            // custom capture properties are declared here.

            #[zero_ui_path::property(capture_only)]
            pub fn custom(arg0: impl IntoVar<bool>) -> !;
        }

        // default values for properties.
        #[doc(hidden)]
        pub mod __defaults {
            use super::*;

            #[inline]
            pub fn background_color() -> impl __properties::background_color::Args {
                __properties::background_color::NamedArgs::new(BACKGROUND)

                // OR

                //__properties::background_color::code_gen! {
                //    named_new __properties::background_color { 
                //        field0: "default0",
                //        field1: "default1",
                //     }
                //}
            }

            #[inline]
            pub fn id() -> impl __properties::id::Args {
                // default inherited.
                crate::widget_base::implicit_mixin::__defaults::id()
            }
        }

        #[doc(hidden)]
        pub mod __whens {
            use super::*;

            pub fn w0(is_focused: &impl __properties::is_focused::Args) -> impl zero_ui::var::Var<bool> {
                todo!("same transform as current widget!")
            }

            // first when block assign values.
            pub mod w0 {
                use super::*;

                pub fn background_color() -> impl __properties::background_color::Args {
                    __properties::background_color::NamedArgs::new(BACKGROUND_FOCUSED)
                }
            }

            #[cfg(debug_assertions)]
            pub fn w0_info(condition_var:
                                   zero_ui_core::var::BoxedVar<bool>,
                               instance_location:
                                   zero_ui_core::debug::SourceLocation) -> zero_ui_core::debug::WhenInfoV1 
            {
                todo!("same as before")
            }
        }

        #[doc(hidden)]
        pub fn new_child(content: impl Widget, custom: impl IntoVar<bool>) -> impl zero_ui_path::UiNode {
            SomeUiNode {
                custom,
                custom_multi
            }
        }

        #[doc(hidden)]
        pub fn new(child: impl zero_ui_path::UiNode, id: WidgetId, custom_multi: (impl IntoVar<bool>, impl IntoVar<u8>)) -> Buttom {
            Button {
                child,
                id,
                custom_multi
            }
        }

        // doc inline so we have the default help for properties that are not documented in the widget.
        // a script should remove this before it is visible.
        #[doc(inline)]
        pub use __properties::is_focused as __doc_is_focused;
    }
}

/*
* button! (widget instantiation)
*/

pub fn widget_user_instantiation() -> impl Widget {
    button! {
        content = text("click me!");
        text_color = colors::LIGHT_BLUE;

        on_click = |ctx, args| {
            println!("button clicked!");
        }// ; not required in the last property or before a when block

        when !self.is_enabled {
            text_color = colors::GRAY;
        }
    }
}

pub fn widget_instantiation_expanded() -> impl Widget {
    {
        let id = path::button::__defaults::id();
        let padding = path::button::__defaults::padding();
        let custom = path::button::__defaults::custom();
        let content = path::button::__properties::content::NamedArgs::new(text("click me!"));
        let text_color = {
            let s0 = text_color::NamedArgs::new(colors::LIGHT_BLUE);
            let s1 = text_color::NamedArgs::new(colors::GRAY);// OR path::button::__whens::w0::text_color();
            let w0 = when_expr_var!(same as before);
            let idx = w0.map(|b| if b { 0 } else { 1 });
            // (switch $property_path:path, $idx:ident, $($arg_n:ident),+) => {
            text_color::code_gen!(switch text_color, idx, s0, s1) 
        };     
        let on_click = path::button::__properties::on_click::NamedArgs::new(|ctx, args| println!("button clicked!"));
        
        let node = path::button::new_child(content, custom);
        let node = padding.set(node);

        let node = text_color.set(node);
        let node = on_click.set(node);
        path::button::new(node, id, custom_multi)
    }
}