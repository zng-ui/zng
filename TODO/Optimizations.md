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

* Widget as a parent defines its children position.
* The global transform of an widget outer and inner regions must be available in info, before render.

* The border "padding" and corner radius mut be available in info.
* The border info of parent must be available for children, they adjust their own corner-radius to fit.
```rust
impl UiNode for BorderNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let border = self.border.layout(ctx);
    wl.with_border(border, |wl| {
      self.child.layout(ctx, wl)
    })
  }
}
impl UiNode for CornerRadiusNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let corners = self.corners.layout(ctx);
    wl.with_corner_radius(corners, |wl| {
      self.child.layout(ctx, wl)
    })
  }
}
```

* Render wants to accumulate transforms to apply in one display item.
* Widgets want to cache size, only invalidating once the contextual metrics it uses changes.
* Widget size cache should survive transform updates.
```rust
impl UiNode for WidgetNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    if self.requested_layout || self.cached_metrics.affected_by(ctx.metrics) {
      wl.with_widget(&self.outer_bounds, &self.inner_bounds, |wl| {
        self.size = self.child.layout(ctx, wl);
      });
    } else {
      // if all bounds are relative we are done
    }
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
// Currently layer link only works if layout after the content, so the same, but now they need to query the info tree.

// How do transforms get updated?
//
// Nodes can affect their own bounds, the widget outer-most node sets the references with `with_widget`, that gets
// targeted by `set_inner_offset` while inside the widget.
//
// Panels can use the `Widget::outer_bounds` maybe? No this lets anyone move the widgets.//
```

* Panels want to position multiple widgets depending on their size.
* Panels may want to change the available size of widgets after measuring then, causing an immediate second layout.
```rust
impl UiNode for VStackNode {
  fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
    let mut available_size = wl.available_size();
    available_size.height = AvailablePx::Inf;
    wl.with_available_size(avaiable_size, |wl| {
      let mut width = 0;
      let mut offset_y = 0;
      self.children.layout_all(ctx, wl, |ctx, wl, size| {
        wl.add_child_offset(PxSize::new(0, offset_y));
        width = width.max(size.width);
        offset_y += size.height;
      });

      width = available_size.width.min(width);
      PxSize::new(width, offset_y)
    })
  }
}

// How does `add_child_offset` works?
// - We could retain the last set `outer_bounds`?
// - Used in conjunction with `WidgetList::layout_all` there is no confusion.
// - Maybe we can have a `with_child` method in WidgetLayout, and only in it we have the child_outer_bounds, also only
//   after the first call of `with_widget` inside it.

wl.with_child(|wl| {
  // wl.set_child_offset(..); // panic here
  self.child.layout(ctx, wl);

  wl.set_child_offset(..);
  // WidgetList::layout_all closure gets called here.
});
// 
// Problems?
// Panels need to handle bad Widget implementations, or just panic really.
```

## Other Layout Changes

* Have metrics value accessible only by methods, on usage of method update a `LayoutMask` as a widget layout cache key.
  - Clones of metrics should still be linked to the same flags.
* Remove `leaf_transform`, it does not give any performance benedict over creating an widget.
  - Make creating a widget easy, `UiNode::to_widget`?
* Consolidate render request for bounds transforms, only request in the node that applies it.

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