# Proc-Macros TODO

Proc-macros are mostly implemented, there are some improvements we can make:

* Replace `when self.foo` with `#foo`, to allow widgets from associated value, e.g: `fn to_view(self) -> impl Widget { }`
* Add `#base::new_*` syntax to allow calling the overridden constructor from inside the new constructor.
    - This lets us avoid needing to make each constructor public and documented for each widget.
    - Generate docs in a `ctor` module?
    - Instead of re-exporting `__new_*` we could re-export in `ctor`?

* Review constructor function errors.
    - Override dyn with static is an error?

* Add doc(cfg) badges to properties.
* Improve property `allowed_in_when` validation for generics, generate a `new` like call for each
  argument, instead of all at once.
* Allow "property as new_name" syntax in widget_new? Can be used for things like double fancy borders.
* Custom lints for when widgets do not delegate to parent constructor functions that have custom nodes.
* False positive, `deny_(zero_ui::missing_delegate)` fails for delegate inside macro, test `cfg!(self.child.layout())`.
* Allow trailing semicolon in widget_new (those are only warnings in Rust, not errors)

* Review all error span hacks when this issue https://github.com/rust-lang/rust/issues/54725 is stable.

## Widget Bind-Self

```rust
#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        b_property = 1 + self.a_property + 3;
    }
}

// # Can we allow handler capture too?

#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        b_property = hn!(self.a_property, |ctx, _| {
            println!(a_property.get(ctx));
        });
    }
}

// We can't reuse the the `when` code for handlers because they are not allowed in `when`.
```

# Auto Node

Improvements to `#[impl_ui_node(struct Node { .. } )]`.

* Rename to `#[ui_node(..)]`.

## Refactor auto-handles to a pseudo attribute

* Allows any member name.
* Is building block for other "auto" stuff.
    - Like `#[var(layout)]` to automatically generate a layout request on update.

```rust
#[impl_ui_node(struct BarNode {
    child: impl UiNode,

    #[var] a: impl Var<bool>,    
    #[var] b: impl Var<bool>,

    #[event] click: Event<ClickArgs>,

    custom: Vec<bool>,
})]
impl UiNode for BarNode { }
```

## Custom delegate in auto node

* Allows auto-node and nonstandard delegation.

```rust
#[impl_ui_node(struct FooNode {
    #[delegate(
        delegate: self.foo.borrow(),
        delegate_mut: self.foo.borrow_mut(),
    )]
    foo: RefCell<BoxedUiNode>,
})]
impl UiNode for FooNode { }

// OR

#[impl_ui_node(struct BarNode {
    #[delegate(child)]
    foo: impl UiNode,
})]
impl UiNode for BarNode { }
```

# Difficult

* Improve `cfg` to support declaring multiple properties with the same name, but different cfg.
* Figure out a way to enable auto-complete and hover tooltip inside macro code?
* Pre-build to wasm: 
    Need $crate support, or to be able to read cargo.toml from wasm,
    both aren't natively supported with [`watt`](https://crates.io/crates/watt).