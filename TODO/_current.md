# All Dyn Rewrite

* Finish implementing new dynamic widget.
    - Implement pre-bind for when expressions.
    - When assigns need to "import" private properties in the same properties! block.
    - Use the term `child` and `children` in widgets, the rename to `content` and `items` does add value.
    - Make `style_mixin` now that the API supports it.
        - Remove `element` widget (already deleted need to stop using).
        - Use the new `NestPosition` to insert the intrinsic at the outermost node should work?
            - How to include the widget stuff? We need to full builder to be able to restore the widget.
            - The full builder needs to be cloned on build not on intrinsic.
            - More than one "service" may want the builder?
                - If so we can have a flag on the builder to insert an "outer-context" node that provides the builder.
                - What other services and how do they interact?
                    - Inspector, we can reduce it to a single node per widget?
                        - What about tracing?
                - This causes a useless build call? -> Build normally, then Style node init builds again.
            - We could have a new item type that is a "build_action: FnMut(&mut WidgetBuilder)"?
                - Better yet, a "build_capture" that replaces the default behavior.
                - They get chained and sorted by an index?

* Refactor all widgets to use the new API.

* Reimplement inspector.
    - Implement widget instance info, type etc.
        - Use info in `widget_base::mixin` error log.
* Remove custom docs stuff.
    - Refactor to minimal docs generation that does not require custom post-processing.
* Update docs of new macros.
* Test all.

* Merge.

# Other

* Update webrender to fx-106
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.
