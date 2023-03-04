* Implement `trait Layout { fn layout(&self) -> T }` and `trait LayoutXY { fn layout(&self, for_x: bool) -> Px }`.
    - For every unit that currently has a `layout` function.
    - For every `Var<T: Layout>` using `Var::with` instead of cloning.

```rust
/// Represents a two-dimensional value that can be converted to a pixel value in a [`LAYOUT`] context.
pub trait Layout2d {
    /// Pixel type.
    type Px: Default;

    /// Compute the pixel value in the current [`LAYOUT`] context.
    fn layout(&self) -> Self::Px {
        self.layout_dft(Default::default())
    }

    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    fn layout_dft(&self, default: Self::Px) -> Self::Px;
}

/// Represents a one-dimensional length value that can be converted to a pixel length in a [`LAYOUT`] context.
pub trait Layout1d {
    /// Compute the pixel value in the current [`LAYOUT`] context.
    fn layout(&self, x_axis: bool) -> Px {
        self.layout_dft(x_axis, Px(0))
    }
    
    /// Compute the pixel value in the current [`LAYOUT`] context with `default`.
    fn layout_dft(&self, x_axis: bool, default: Px) -> Px;

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis.
    fn layout_x(&self) -> Px {
        self.layout(true)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis.
    fn layout_y(&self) -> Px {
        self.layout(false)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***x*** axis with `default`.
    fn layout_dft_x(&self, default: Px) -> Px {
        self.layout_dft(true, default)
    }

    /// Compute the pixel value in the current [`LAYOUT`] context ***y*** axis with `default`.
    fn layout_dft_y(&self, default: Px) -> Px {
        self.layout_dft(false, default)
    }
}

impl<T: Layout2d, V: Var<T>> Layout2d for V {
    type Px = T::Px;
    
    fn layout_dft(&self, default: Self::Px) -> Self::Px {
        self.with(move |v| v.layout_dft(default))
    }
}
```

* Test all.
* Merge.

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