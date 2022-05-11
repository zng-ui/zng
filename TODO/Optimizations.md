# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Need to make every thing sync.
    * Vars are already *locked* for updates due to their delayed assign, is only reading from `Arc` slower then from `Rc`?
    * Event `stop_propagation` becomes indeterministic.
    * Services must become sync.
    * State must become sync!
* Maybe can always have an AppContext for each UI thread, with a copy of services and such, after each update they merge into
  the main AppContext.

# Mouse Move Interest

* Let widgets define what sort of mouse event they want, use the hit-test tag, filter events in the view-process?

# Update Mask

* Sub-divide UiNodeList masks.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.

# Single Pass Layout

* Can we merge measure, arrange?
  - `UiNode::layout(LayoutContext, WidgetLayout)`
  - How do we do `available_size` -> `desired_size` -> `final_size` in one pass?
  - What do we loose by making the `desired_size` be the `final_size`?
   - A layout that equally divides the extra space after measuring children for each child?
   - There is nothing stopping us from doing two passes in this case.

```rust
trait UiNode {
  /// Available-size is in ctx.metrics.
  fn layout(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout) -> PxSize;
}

// usage:

impl UiNode for Center {
  fn layout(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout) -> PxSize {
    let desired_size = self.child.layout(ctx, widget_layout);
    let offset = self.center(ctx.metrics.available_size, desired_size);
    // how does the transform gets comunicated to child?
  }
}
```

## Layout Requirements

* Widget as a child defines its own size.
```rust
impl UiNode for WidthNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let mut size = self.child.layout(ctx, wl);
    size.width = self.width.to_layout(ctx); // Metrics dependencies recorded here.
    size
  }
}
``` 

* Widget as a parent defines its children position.
* The global transform of an widget outer and inner regions must be available in info, before render.
```rust
impl UiNode for CenterNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let size = self.child.layout(ctx, wl);

    let available_size = ctx.metrics.available_size;
    if available_size.is_inf() {
      size
    } else {
      let offset = self.center(ctx, size);
      // TODO how to add offset to child transform?
      // wl.offset_child(offset); // this causes each parent to visit all node transforms again.
      //                          // of if we make the outer/inner info be relative, forces a big matrix merge for decorators.
      //                          // right now we place the parent transform in the stack and acumulate as we go.
      available_size
    }
  }
  
}
```

* The border "padding" and corner radius mut be available in info.
* The border info of parent must be available for children, they adjust their own corner-radius to fit.
```rust
impl UiNode for BorderNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let border = self.border.to_layout(ctx);
    wl.with_border(border, |wl| {
      self.child.layout(ctx, wl)
    })
  }
}
impl UiNode for CornerRadiusNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let corners = self.corners.to_layout(ctx);
    wl.with_corner_radius(corners, |wl| {
      self.child.layout(ctx, wl)
    })
  }
}
```

* Render wants to accumulate transforms to apply in one display item.
```rust
impl UiNode for InnerBoundaryNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let size = self.child.layout(ctx, wl);
    wl.set_bounds(self.bounds.clone(), size); // parent `offset_child` affects these bounds.
    size
  }
}
```

* Widgets want to cache size, only invalidating once the contextual metrics it uses changes.
* Widget size cache should survive transform updates.
```rust
impl UiNode for WidgetNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    if self.requested_layout || self.cached_metrics.affected_by(ctx.metrics) {
      self.size = self.child.layout(ctx, wl);
    } else {
      // if all bounds are relative we are done
    }
    wl.set_bounds(self.bounds.clone(), self.size); // parent `offset_child` affects these bounds.
    self.size
  }
}

// reading global bounds
// if relative no convenient direct reference to computed transform
fn global_bounds(wgt: &WidgetInfo) -> Transform {
  let mut t = wgt.outer_bounds().transform();
  for wgt in wgt.ancestors() {
    t = t.then(wgt.inner_bounds().transform());
    t = t.then(wgt.outer_bounds().transform());
  }
  t
}
// How does layer link operates in this case?
// Currently layer link only works if layout after the content, so the same, but now they need to query the info tree?
```

## Other Layout Changes

* Place metrics in `WidgetLayout`.
* Metrics contains AvailableSize, have sub-selection of metrics for each dimension.
* Rename Length types `to_layout` to just `layout`. 
* Have metrics value accessible only by methods, on usage of method update a `LayoutMask` as a widget layout cache key.

# Cache Everything

General idea, reuse computed data for `UiNode` info, layout and render at
widget boundaries if the widget or inner widgets did not request an update of these types.

## `UiNode::measure` and `UiNode::arrange`

* Already started implementing this, see `LayoutMask`.
* Await single pass layout rewrite.

## `UiNode::render`

Webrender needs to support this? Can we implement our own display list? If so, we can record the inserted range of display list,
keep the old display list around, then copy from it to the new display list. Maybe even have the ranges expand in the view-process?

* See `DisplayListBuilder::start_item_group`, `finish_item_group` and `push_reuse_items`.
* `set_cache_size` is a part of this too? Yes needs to be set to the count of item groups.
* Does not allow nesting, can we auto-split nested items?

If each widget here is a "reuse item", we can auto generate WR keys like:
  - widget_0 = start key0
    -child_1 = end key0, start key1
      -leaf3 = end key1, start key2
      -leaf4 = end key2, start key3
     child_1 = end key4, start key5 // child_1 added more items after content.
    -child_2 = end key6, start key7
      -leaf5 = end key8, start key9

We can store the key range for each widget, if it did not invalidate render it can generate all keys and push reuse:
- widget_0 = key0..=key9
-  child_1 = key0..=key5
-  child_2 = key6..=key9
-    leaf3 = key1..=key2

* Keys are `u16` are we generating to many keys?
  - If we hit max `65535` we cause a full frame rebuild?
  - If we hit max in a single frame, just stop caching for the rest, the window is probably exploding anyway.
* If keys are just ranges, how to update unchanged items after a remove?
  - If we insert a new leaf after lead3 what key will it get?

# Image Render

* Try reusing renderer.

# View Open

* Try to detect unsupported render mode without glutin.
* Try to implement async context creation in default view crate.
    - Problem, glutin needs the event-loop window target to build a context (it is not send and must be in main).
    - Can use `build_raw_context` that only requires a window handle, so we create the winit window blocking then offload
      everything to a thread.
    - gleam uses a `Rc<dyn Gl>` for the OpenGL functions.
    - There are obscure bugs with sending OpenGL contexts across threads, maybe review using `surfman` again.