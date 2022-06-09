# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Need to make every thing sync.
    * Vars are already *locked* for updates due to their delayed assign, is only reading from `Arc` slower then from `Rc`?
    * Event `propagation` becomes indeterministic.
    * Services must become sync.
    * State must become sync!
* Maybe can always have an AppContext for each UI thread, with a copy of services and such, after each update they merge into
  the main AppContext.

# Update Mask

* Sub-divide UiNodeList masks.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.

# Layout Passes

* Right now we need multiple passes to do things like define the size of cells in the uniform grid depending on
  the constrains on the panel, maybe we can reduce these to one pass somehow?
   - We need to know the size a widget would be like given a constrain, and them call layout and give the widget
     our preferred constrain.
   - We can't have the widget respond to multiple constrains at the same time, because widgets may generate different
     content depending on their size.

* Multiple passes cancels out layout reuse of everything that depends on the constrains, maybe we can at least
  signal the children that a layout pass is "experimental".
    - This still causes an "experimental" pass for each panel that contains a single child that requested layout.

* We can cache multiple layout sizes in widgets, then reuse if any matches?
  - No, reused widget will still try to render the last size observed.
  - We can cache the "experimental" marked result and the actual result.

* What are the "experimental" results that panels need:
  - `uniform_grid`:
    * Min size (not fill) to select the minimal uniform cell size when the panel is not fill or fixed size.
  - `v_stack`:
    * Min width (not fill) to select the panel width when it is not fill-width or fixed-width.
    * Min height, to determinate the extra fill size when items fill-height.
  - `grid`:
    * Similar to uniform, but cells only affect their row and column.
  - Any future takers on the max_size, or is it just the min?
    * Can widgets know their min_size before layout?
      - No, they need to compute lengths.

* Can we know what child has requested layout and only do two passes in it, unless it defines the cell size?
  - Needs to insert widget info in `Updates` but it is possible.

* We can let users define the "mold" widget that is used to compute the others, this needs only one pass then.
  - Lets do this if we fail to optimize to one pass.
  - Needs a better name.

* If we make an `experimental` flag in LayoutContext:
  - Can cache two different results.
  - Widgets can avoid requesting render for experimental.
  - Downside, all widgets need to be layout twice, because they will not record changes for experimental.
  - Downside, widgets **must not** save anything in the experimental pass, because we want to reuse in the actual after experimental.
  - Review leaf widgets like `image` and `text` first, can they actually handle this without recording state?
    - `image`, no problem, can even cut some of the computation.
    - `text`, need rewrite, the experimental pass can easily cause a reshape, and that gets saved as we compute right now.

* Add an `UiNode::layout_query(&self, ctx: &mut LayoutQueryContext, wl: &mut WidgetLayoutQuery) -> PxSize`?
  - It cannot write to `self` or request any kind of updates, is like an `InfoContext` with metrics.
  - If only computes sizes, not positioning of anything.
  - Cannot be auto-implemented if `UiNode::layout` is custom.
  - Can call it `UiNode::measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize`?
    - Is different from the older `measure/arrange`, but we can only confuse ourselves..
  - Is any parent going to need the align of children too?
    - They can still do two full layout passes.
  - Can we reduce the `LayoutMetrics`?
    - No, font sizes can depend on the viewport for example.
  - How does the cache works?
    - Cache last layout and last measure.
    - Measure also hits cache if matches the layout (and is not pending a layout).
  - What if the widget is measured after it requests layout?
    - It will have to potentially create temporary stuff, like a shaped text just for measure?
  - Is the `WidgetMeasure` only for symmetry?
    - We can do the use/reuse thing in the `MeasureContext`.
    - Only very advanced users (writing fully custom widgets) will ever need to thing about this.
  - Final design `UiNode::measure(&self, ctx: &mut MeasureContext) -> PxSize`.
    - And the context is just an `InfoContext` with extra `LayoutMetrics`.

# Better render reuse

* Already started implementing this, see `ReuseGroup`.
  - Webrender reuse depends on space/clip ids and these invalidate all items after an insert remove.
  - Worst, we can't cache creation of space/clips because it will just mess-up the id count for all subsequent items.
  - Maybe we should stop using the webrender display list.
    - If we had more access to the display list internals we could save by ranges for each widget, then send range refs to
      the previous display list bytes that is retained in the view-process.

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