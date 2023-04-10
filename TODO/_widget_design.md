```rust
/// Docs.
#[widget($crate::widgets::Button!)]
pub struct Button(crate::widgets::Container);

impl Button {
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.builder().push_intrinsic(todo!());
        // called by generated `start`, if not provided calls endup derefing parent that has it.
    }

    /// Build widget to custom type.
    /// 
    /// If not provided call derefs to parent that implements it.
    pub fn build(&mut self) -> impl UiNode {
        let mut wgt = self.0.take_builder();
        wgt.build()
    }
}

/// Foo docs.
#[property(CONTEXT, default(Foo), impl(Button))] // impl Button { fn foo }
pub fn foo(child: impl UiNode, foo: impl IntoVar<Foo>) -> impl UiNode {

}

/// Bar docs.
#[property(CONTEXT, default(Foo), for(Button))] // impl foo for Button
pub fn bar(child: impl UiNode, foo: impl IntoVar<Foo>) -> impl UiNode {

}
```

Expands to

```rust
#[doc(hidden)]
#[macro_export]
macro_rules! Button {
    ($($tt:tt)*) => {
        #zero_ui::core::widget_new! {
            path { $crate::widgets::Button }
            input { $($tt)* }
        }
    }
}

/// Docs.
/// 
/// # Macro
/// 
/// You can use the `Button! {  }` macro to instantiate this widget, see [widget macro syntax].
pub struct Button {
    base: crate::widgets::Container,
    started: bool,
}

impl std::ops::Deref for Button {
    Target = crate::widgets::Container;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl std::ops::DerefMut for Button {
    fn deref_mut(&self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Button {
    /// Start building a new instance.
    pub fn start() -> Self {
        Self::inherit(WidgetType {
            id: std::any::TypeId::of::<Self>(),
            path: "$crate::widgets::Button",
            location: #zero_ui::source_location!(),
        })
    }

    /// Start building a widget derived from this one.
    pub fn inherit(widget: WidgetType) -> Self {
        let mut wgt = Self {
            base: crate::widgets::Container::inherit(widget),
            started: false,
        };
        wgt.on_start__();
        wgt
    }
}
impl Button {
    #[doc(hidden)]
    fn on_start__(&mut self) {
        if !self.started {
            self.started = true;
            self.on_start();
        }
    }

    fn on_start(&mut self) {
        self.builder().push_intrinsic(todo!());

        widget_defaults! {
            self =>
            background_color = colors::RED;
        }
        // called by generated `start`, if not provided calls endup derefing parent that has it.
    }

    /// Build widget to custom type.
    /// 
    /// If not provided call derefs to parent that implements it.
    pub fn build(&mut self) -> impl UiNode {
        let mut wgt = self.take_builder();
        wgt.build()
    }
}

impl Button {
    /// Foo docs.
    pub fn foo(&self, foo: impl IntoVar<Foo>) {
        self.push_property(Self::foo_args__(foo));
    }
    #[doc(hidden)]
    fn foo_args__(foo: impl IntoVar<Foo>) -> Box<dyn PropertyArgs> {
        struct foo {
            foo: BoxedVar<Foo>,
        }
        impl PropertyArgs for foo {
            fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode {
                $crate::widgets::Button::foo_impl__(child, self.foo.clone()).boxed()
            }
        }
        Box::new(foo {
            foo: foo.into_var()
        })
    }
    fn foo_impl__(child: impl UiNode, foo: impl IntoVar<Foo>) -> impl UiNode {

    }
}

/// Bar docs.
pub fn bar(child: impl UiNode, foo: impl IntoVar<Foo>) -> impl UiNode {

}
#[doc(hidden)]
pub trait bar {
    /// Bar docs.
    fn bar(&mut self, foo: impl IntoVar<Foo>);
    fn bar_args__(foo: impl IntoVar<Foo>) -> Box<dyn PropertyArgs> {
        struct bar {
            foo: BoxedVar<Foo>,
        }
        impl PropertyArgs for bar {
            fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode {
                self::bar(child, self.foo.clone()).boxed()
            }
        }
        Box::new(bar {
            foo: foo.into_var()
        })
    }
}
impl bar for Button {
    fn bar(&mut self, foo: impl IntoVar<Foo>) {
        self.push_property(Importance::INSTANCE, <Button as foo>::bar_args__(foo))
    }
}
```

Usage:

```rust
use crate::widgets::Button;

let btn = Button! {
    #[cfg(test)]
    id = "btn";

    background_color = colors::RED;
    border = {
        widths: 1,
        sides: colors::DARK_RED,
    };

    text::color = colors::WHITE;

    when *#is_hovered {
        background_color = colors::GREEN;
    }
};
```

Expands to:

```rust
let btn = {
    let mut w = Button::start();
    
    #[cfg(test)]
    w.id("btn");
    
    w.background_color(colors::RED);
    {
        // init in call order
        let widths = 1;
        let sides = colors::DARK_RED;
        // input in member name sort order.
        w.border_named__(sides, widths);
    }

    <$crate::widgets::Button as text::color>::color(&mut w, colors::WHITE);

    w.begin_when(when_condition!(wgt, *#is_hovered));
    w.background_color(colors::GREEN);
    w.end_when();
    
    w.build()
};
```

Open questions:

* Path properties `properties::size = 50;`. Is just a localized use?
    - Right now we can do `window::size = 10` to get the window property.
    - Because window size is directly associated with the type there is no need to this.
* Widget modules that contain stuff (like `text::TXT_COLOR_VAR`).
    - Can still have it, but negates `Text! { text = ""; }` name collision.
    - Maybe we can have the `text_wgt` suffix?
* Mix-ins
    - They are a `#[widget] trait FocusableMixin: WidgetProperties { }`, then Button can manually `impl Focusable for Button`.
* Intrinsic
    - Generated new calls `self.intrinsic(&mut WidgetBuilder)`?
    - Need to always include the parent widgets.
    - Cannot insert doubled.
* Importance::WIDGET
    - How are widget defaults defined?