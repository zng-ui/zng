* Text shaping needs "Language" and "Script".
    - Replace `unicode-script` with `unic-langid`.
    - Enable macros for `unic-langid`, re-export it?
        - Their macros panic for errors, maybe re-implement it on our own?
    - Don't implement IntoVar for str? Encourage macro usage, validation at compile time.

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

CSS: https://www.w3schools.com/css/css_boxmodel.asp (has background-clip to sort-of place the border)
Flutter: Like CSS?
WPF: Size like ours, no border placement, border is outset, stroke is half-way inset.