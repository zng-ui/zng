# Widget Cfg

* Widget proc-macros bug, default properties with `#[cfg(active_only_in_widget_crate)` are not set in user crate if the
  user crate does not have the same cfg flag.
  - Generate a `macro_rules! cfg_active_only_in_widget_crate` that discards or not the input?
    - Can always derive the macro name from the cfg span, but needs dedup on creation.
  - Generate a `macro_rules! __target_property_cfg` instead.
    - Can derive the macro name from the property name, only needs to know the property has cfg (already implemented).
* Implement widget property cfg docs.

* Document `cfg` support, we support toggle property, toggle capture in new event, we **don't** support alternate
  declarations, two items with same ident, but different cfg.

## Scenario Base

crate1 with default feature `foo`:
```rust
#[property(context)]
pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
  println!("created foo!");
  // ..
}

#[widget($crate::bar)]
pub mod bar {
  properties! {
    #[cfg(feature="foo")]
    foo = true;

    #[cfg(feature="foo")]
    foo_cap(impl IntoVar<bool>) = true;
  }

  fn new_event(child: impl UiNode, #[cfg(feature="foo")] foo_cap: impl IntoVar<bool>) -> iml UiNode {
    #[cfg(feature="foo")]
    let child = something(child, foo_cap);
    // ..
  }
}
```

crate2 derive from crate1 with defaults, but has not `foo` feature:
```rust
bar! {
  // expected: prints "crated foo!" and builds.
  // current: no build due to `new_event` still expecting a `foo_cap` that was not set.
}
```

current expansion:
```rust
// ..

let node = bar::new_event(child, #[cfg(feature = "foo")] some_cap_args);

#[cfg(feature = "foo")]
let node = foo(child, some_args);

// ..
``` 

new expansion:
```rust
// ..

let node = bar::new_event(child, bar::foo_cap_cfg! {
  some_cap_args
});

bar::foo_cfg! {
  let node = foo(child, some_args);
}

//..
```

## Scenario Set

crate2
```rust
  bar! {
    foo = false;
    // expected: build ok if the feature is enabled for the crate1
  }
```

## Scenario Not Enabled In Crate1

```rust
bar! {
  // expected: don't print anything, no args passed to cap.
}
```

## Scenario Enabled by Crate2 Feature

```toml
default = []
crate2_foo = ["crate1/foo"]
```

```rust
bar! {
  #[cfg(feature = "crate2_foo")]
  foo = true;

  // expected: `foo = true` is only valid if "crate1/foo" is active, that feature is toggled by "crate2_foo".
  //           same for `foo_cap`.
}
```

Expands to:

```rust
// ..

let node = bar::new_event(child, bar::foo_cap_cfg! {
  some_cap_args
});

#[cfg(feature = "crate2_foo")]
bar::foo_cfg! {
  let node = foo(child, some_args);
}

//..
```

# Other

* Finish smooth scrolling.
  - Chase animation, right now next scroll pos. calculated from var current value, need to record next value and compute from there.
  - Can we abstract this as a method in `Var`, seems useful.
  - Implement `smooth_scrolling` config property.

* Build Optimization, see `Optimizations.md`.
* Animation, see `Variables.md`.