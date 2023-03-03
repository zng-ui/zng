* Reduce boilerplate of core units `layout` methods.
    - They all request the metrics and default.
    - Some units are one-dimensional so the caller must define what dimension they operate in.
    - Default needs to be a closure because some defaults make use of metrics.
        - This is rare, but the idea is that the default metrics usage is only flagged if the default is used.
    - Default evaluation needs to be injected as-well, because `LengthExpr` may need it we can't just return `None` or something.
    - `Leftover` is set on the context, but default is set on the input.
        - Maybe we can just make the default be a value (not closure) and set on the context like leftover.
        - And if not set the default is zero.
        - In some rare cases we may default only metrics dependency, but the perf loss is minimal.
```rust
// normal layout, with zero default
let size = self.size.get().layout();
let width = self.width.get().layout_x();

// with default
let size = LAYOUT.with_default(Px(10), Px(10), || Size::default().layout());
let width = LAYOUT.with_default_x(Px(10), || Length::Default.layout_x());

// impl looks like:
impl Size {
    fn layout(&self) -> PxSize {
        PxSize::new(self.width.layout_x(), self.height.layout_y())
    }
}
impl Length {
    fn layout(&self, x_axis: bool) -> Px {
        match self {
            Length::Default => LAYOUT.default_for(x_axis),
            ..
        }
    }

    fn layout_x(&self) -> Px {
        self.layout(true)
    }
    fn layout_y(&self) -> Px {
        self.layout(true)
    }
}
impl LAYOUT {
    fn with_default<R>(&self, x: Px, y: Px, f: impl FnOnce() -> R) -> R {

    }
    fn default_x(&self) -> Px {
        self.default_for(true)
    }
    fn default_y(&self) -> Px {
        self.default_for(false)
    }
}
```

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Integrate `ThreadContext` with `rayon`.
    - Need to capture and load contexts for all `rayon::join` and `rayon::scope`.
    - See issue https://github.com/rayon-rs/rayon/issues/915
* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.