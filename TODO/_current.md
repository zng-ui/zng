# All Dyn Rewrite

* Countdown animation did not work.
    - If the icon is removed it works..
    - Disabling `INIT_CONTEXT` works.
Minimal:
```rust
use zero_ui::prelude::*;

fn main() {
    let mut app = App::default().run_headless(false);

    let source = var(0u32);
    let mapped = source.map(|n| n + 1);
    let mapped2 = mapped.map(|n| n - 1); // double contextual here.
    let mapped2_copy = mapped2.clone();

    // init, same effect as subscribe in widgets, the last to init breaks the other.
    assert_eq!(0, mapped2.get());
    assert_eq!(0, mapped2_copy.get());

    source.set(&app, 10u32);
    let mut updated = false;
    app.update_observe(
        |ctx| {
            updated = true;
            assert_eq!(Some(10), mapped2.get_new(ctx));
            assert_eq!(Some(10), mapped2_copy.get_new(ctx));
        },
        false,
    )
    .assert_wait();

    assert!(updated);
}
```

* Refactor to minimal docs generation that does not require custom post-processing?
* Update docs of new macros.
* Merge.

# Other

* Update Rust, dependencies.
* Update webrender to fx-106
    - https://github.com/servo/webrender/pull/4724
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

* If `ui_list![]` auto boxes in the macro params, but the returned type does not auto-boxes on `.push`.
    - Before refactor we had `WidgetVec(pub Vec<BoxedWidget>)`.

* Implement `widget::path.property` syntax support in widget instantiation and `when` expressions.
    - in when expressions: `when #foo.foo.1`, has ambiguity with `when #foo.foo`.
        - Mostly want it to support `#self.exported_prop`, nice callback to the previous syntax.
        - If we can't establish unambiguity, assume `#property_ident`, support `#::widget_ident` to select a widget.

* Implement pre-bind for when expressions.
* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Rename `toggle::selection` to `toggle::selector` or rename `Selector` to `Selection`.
* Merge `Property.new/new_when` into a single constructor that uses some kind of provider interface.
    - Like the nightly Any provider.
* Improve widget property imports, when inheriting from widgets a `use self::*;` can override inherited properties.
    - In the `image` example we need the full path to set the window size because of this.
* Refactor property priority into constants: `#[property(CONTEXT)]`.
* Test property generics `value::<bool> = true; when *#is_something { value::<u32> = 32; }`.
* Review `IntoVar` and `IntoVarValue` constrains, we don't need then to be debug/clone anymore?
* Const errors don't show if  there is a compile error, so the when `!foo::ALLOWED_IN_WHEN_EXPR` does not show.
    - Generate a placeholder `__w_0__`?