* Text shaping needs "Language" and "Script".
* Colors don't match other apps, clear_color and background_color also does not match.
* WindowChangedEvent fired on first resize.
* Review `open_widget_display`, needs to be called with widget inner size to calculate default origin (center).

# Layer

* Focus does not recover to new root scope.
    - Disable window.
    - Show overlay with own focus scope.
    Ideal Behavior: focus on the overlay as if the window opened with it.

# Split `inner`

All the visual layers are in `inner`, this causes problems with order or `border` and `background`, the button `focus_highlight` changes
size depending on when you set for example.

Need to:

* Review box model of other frameworks.
* Rename `outer` to `layout` and split `inner` into `border`, `fill`.

## Our Model

0 - `context`
1 - `event`
2 - `layout`
3 - `size`
? - *background* = CSS by default extends the background *under* the borders.
4 - `border`
5 - `fill`
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

CSS: https://www.w3schools.com/css/css_boxmodel.asp (has background-clip to sort-of place the border)
Flutter: Like CSS?
WPF: Size like ours, no border placement, border is outset, stroke is half-way inset.