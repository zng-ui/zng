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
        // This function must return `impl UiNode`.
        //
        // This function is not required, if missing, the new_child of the last inherited widget is used.
        // If not widget is inherited the `zero_ui::core::widget_base::default_new_child` is used.
        //
        // The function does not need to be public, you can decide if it will show in the docs or not.
        fn new_child(content: impl Widget, custom: impl IntoVar<bool>) -> impl UiNode {
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
    /// widget attributes.
    /// custom widget sections docs.
    pub mod button {
        /// custom items get copied here.
        
        use crate::widgets::*;
        use crate::properties::*;
        
        pub const BACKGROUND: Rgba = colors::GRAY;
        pub const BACKGROUND_FOCUSED: Rgba = colors::LIGHT_GRAY;

        // and so do the custom new functions.

        fn new_child(content: impl Widget, custom: impl IntoVar<bool>) -> impl UiNode {
            SomeUiNode {
                custom,
                custom_multi
            }
        }

        pub fn new(child: impl UiNode, id: WidgetId, custom_multi: (impl IntoVar<bool>, impl IntoVar<u8>)) -> Buttom {
            Button {
                child,
                id,
                custom_multi
            }
        }

        // macros uses in macro_rules are reexported here.
        #[doc(hidden)]
        pub use zero_ui::widget_inherit as __widget_inherit;    
        #[doc(hidden)]
        #[macro_export]
        macro_rules! button_inherit_df18a4960c9c4924b503e192adb095ca {
            ( 
                mixin { $mixin:tt } 
                inherit { $($inherit:path;)* }
                $($rest:tt)+
            ) => {
                $crate::widgets::button::__widget_inherit! {
                    // if the widget that is inheriting this is a mixin.
                    mixin { $mixin }
                    // other inherited widgets to be processed after this.
                    inherit { $($inherit;)* }
                    // inherit data from this widget.
                    inherited { 
                        module { $crate::widgets::button }
                        mixin { false }
                        properties_child {
                            /// padding docs.
                            padding {
                                default true, // has default value
                                required false // not required, can `unset!`.
                            }
    
                            // .. + all child properties
                        }
                        properties {
                            /// background_color docs.
                            background_color {
                                default true
                                required false
                            }
                            content {
                                default false
                                required true // content is required, cannot `unset!`.
                            }
                            on_click {
                                default false
                                required false
                            }
                            is_focused { // when state properties are reexported.
                                // they don't have a default value defined in the widget
                                // but will be initialized automatically for the when expression.
                                default false 
                                // they are also not required, can they be `unset!`?
                                required false
                            }
    
                            // .. + all normal properties
                        }
                        whens {
                            /// w0_is_focused docs.
                            __w0_is_focused { // auto generated name tries to convert to expression to text.
                                // properties used in the when expression.
                                inputs { is_focused }
                                // properties set by the when block.
                                assigns { background_color }
                            }
                        }
    
                        // captured properties for each new function.
                        // these two entries are not present when `mixin { true }`
                        new_child { content custom }
                        new { id custom_multi }
                    }
                    $($rest)*
                }
            };
        }
        #[doc(hidden)]
        pub use crate::button_inherit_df18a4960c9c4924b503e192adb095ca as __inherit;

        // widget::__new!(..) is only generated if the widget is not a mixin.
        #[doc(hidden)]
        pub use zero_ui::widget_new as __widget_new;    
        #[doc(hidden)]
        #[macro_export]
        macro_rules! button_new_df18a4960c9c4924b503e192adb095ca {
            ($($tt:tt)*) => {
                $crate::widgets::button::__widget_new!  {
                    widget {
                        module { $crate::widgets::button }
                        
                        // no mixin section.

                        properties_child {
                            // no property docs in new
                            padding {
                                default true,
                                required false 
                            }
                        }
                        properties {
                            // same as inherit but without the docs.
                        }
    
                        whens {
                            // no docs here.
                            __w0_is_focused { // auto generated name tries to convert to expression to text.
                                // properties used in the when expression.
                                inputs { is_focused }
                                // properties set by the when block.
                                assigns { background_color }
                            }
                        }
                        // captured properties for each new function.
                        // these two entries are required in new.
                        new_child { content custom }
                        new { id custom_multi }
                    }
                    user {
                        // user tokens.
                        $($tt)*
                    }
                }
            };
        }
        #[doc(hidden)]
        pub use crate::button_new_df18a4960c9c4924b503e192adb095ca as __new_macro;

        // properties are reexported using the `__p_#ident` format.  
        // #[doc(inline)] so we have the default docs for properties without docs, the docs are hidden
        // before they actually show in screen, properties with defined docs are doc(hidden) from the start.      

        #[doc(inline)] 
        pub use crate::layout::margin::export as __p_padding;

        // reexports inherited properties.
        #[doc(inline)]
        pub use zero_ui::core::widget_base::implicit_mixin::__p_enabled;

        // reexports with local paths too.
        #[doc(inline)]
        pub use background_color::export as __p_background_color;

        // declares custom properties with the same name format.
        #[doc(hidden)]
        #[zero_ui::core::property(capture_only)]
        pub fn __p_custom(custom: impl IntoVar<bool>) -> !;

        // default values are functions with the `__d_#ident` format.

        #[doc(hidden)]
        pub fn __d_padding() -> impl self::__p_padding::Args {
            self::__p_padding::ArgsImpl::new(10)
        }

        #[doc(hidden)]
        pub fn __d_multi() -> impl self::__p_multi::Args {
            self::__p_multi::code_gen! {named_new self::__p_multi {
                field0: false,
                field1: 200,
            }}
        }

        #[doc(hidden)]
        pub fn __d_background_color() -> impl self::__p_background_color::Args {
            self::__p_background_color::ArgsImpl::new(BACKGROUND)
        }

        // when condition expressions become functions with the `__w#i_#expr_as_str` format.
        // we also do the allowed_in_when asserts.

        self::__p_is_focused::code_gen!{assert allowed_in_when=> "property `is_focused` is not allowed in when condition"}
        #[doc(hidden)]
        pub fn __w0_is_focused(__self_is_focused: impl self::__p_is_focused::Args) -> impl zero_ui::core::var::Var<bool> {
            // # the expression converted to var return, map or merge.
            __self_is_focused.unwrap()
        }

        #[doc(hidden)]
        pub fn __w0_d_background_color() -> impl self::__p_background_color::Args {
            self::__p_background_color::ArgsImpl::new(BACKGROUND_FOCUSED)
        }

        // new functions are wrapped in a call that unwraps the args and validates the types.

        // can also be a reexport of an inherited __new_child.
        #[doc(hidden)]
        pub fn __new_child(
            content: impl self::__p_content::Args, 
            custom: impl self::__p_custom::Args
        ) -> impl zero_ui::core::UiNode {
            // type validation is done by rustc here.
            self::new_child(
                self::__p_content::Args::unwrap(content),
                self::__p_custom::Args::unwrap(custom)
            )
        }

        #[doc(hidden)]
        pub fn __new(child: impl zero_ui::core::UiNode, 
            id: impl self::__p_id::Args, 
            custom_multi: impl self::__p_custom_multi::Args)
         -> Buttom {// the return type is copied from the function `new`.
            self::new(child, 
                self::__p_id::Args::unwrap(id),
                self::__p_custom_multi::Args::unwrap(custom_multi)
            )
        }
    }
    // if widget is not a mixin.
    #[doc(hidden)]
    pub use button::__new_macro as button;
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
        let __id = path::button::__d_id();
        let __padding = path::button::__d_padding();
        let __custom = path::button::__d_custom();
        let __content = path::button::__p_content::ArgsImpl::new(text("click me!"));
        let __text_color = {
            let s0__ = text_color::ArgsImpl::new(colors::LIGHT_BLUE);
            let s1__ = text_color::ArgsImpl::new(colors::GRAY);// OR path::button::__w0_d_text_color();
            let w0__ = when_expr_var!(same as before);
            let idx__ = w0__.map(|b| if b { 0 } else { 1 });
            // (switch $property_path:path, $idx:ident, $($arg_n:ident),+) => {
            text_color::code_gen!(switch text_color, idx__, s0__, s1__) 
        };     
        let __on_click = path::button::__p_on_click::ArgsImpl::new(|ctx, args| println!("button clicked!"));
        
        let node__ = path::button::__new_child(__content, __custom);
        let node__ = __padding.set(node__);

        let node__ = text_color.set(node__);
        let node__ = on_click.set(node__);
        path::button::new(node__, __id, __custom_multi)
    }
}