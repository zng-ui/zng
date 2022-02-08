# Border/Background Problem

All the visual layers are in `inner`, this causes problems with order or `border` and `background`, the button `focus_highlight` changes
size depending on when you set for example.

Need to:

* Split `inner` into `border` and `fill`?
    - CSS backgrounds extend under the border, the border can clip the background but the content is offset to be inside the border.
        in our model both background and content would be inside the borders. 
        - Could optionally reverse offset just for background?
* Widget inner clip?
    - Aggregated clip  that is applied on inner.
* Widget rounded corners?
    - ContextVar that defines the `corner_radius`of the outer-most border so that inner borders can calculate their
        own to fit automatically (also integrates with clip)?

### Our Model

0 - `context`
1 - `event` 
2 - `layout`
3 - `size`
? - *background* = CSS by default extends the background *under* the borders.
4 - `border`
5 - `fill` = CSS by default offsets the content to be inside the borders (different from background).
--- Repeat Above for the `child` wrapper in a widget, so *padding* is in `child_layout`.

Main details, size defines the actual visual size, CSS grows out of the size ending in a computed final size,
out model allows mimicking CSS by setting the size in the `child` that is the equivalent to the HTML "content".

### Background Question

Users may want to extend the background under the border, if we place background properties in `fill` this becomes tricky, 
we would need to know the border offsets and apply an inverse transform in the background to extend it.

If we add a `background` priority we then need to maintain two sets of *background* properties, one that extends under and one that does not.

We could track border offsets in `WidgetLayout` to define theirs combined offset, then have a `context` property that sets config for `new_fill`
to insert the inverted transform, **the corner rounding clip is an open question**.

### Round Corners

Should corner angles be a `context` config property for the widget?

As it is right now if we have more then one border we need to compute the radius of each one to fit, there is a question of where they
clip the `fill` too, we could have a `corner_radius` property that defines the border *outer* radius and compute radius for borders and inner
clip from there.

## Reference

CSS: https://www.w3schools.com/css/css_boxmodel.asp (has background-clip)
Illustrator *stroke* can be aligned Inner, Centered, Outer.

# Design

```rust
impl WidgetLayout {
    /// Current corner radius set by [`with_corner_radius`] and deflated by [`with_border`].
    ///
    /// [`with_corner_radius`]: Self::with_corner_radius
    /// [`with_border`]: Self::with_border
    pub fn corner_radius(&self) -> PxCornerRadius {
        self.corner_radius
    }

    /// Sets the corner radius that will affect the next inner borders.
    ///
    /// After each [`with_border`] the `corners` value will be deflated to fit inside the *outer* border.
    ///
    /// [`with_border`]: Self::with_border
    pub fn with_corner_radius(&mut self, corners: PxCornerRadius, f: impl FnOnce(&mut Self)) {
        let c = mem::replace(&mut self.corner_radius, corners);

        f(self);

        self.corner_radius = c;
    }

    /// Deflates the corner radius for the next inner border or content clip.
    pub fn with_border(&mut self, offsets: PxSideOffsets, f: impl FnOnce(&mut Self)) {
        let c = self.corner_radius;
        self.corner_radius = c.deflate(offsets);

        f(self);

        self.corner_radius = c;
    }
}
```
