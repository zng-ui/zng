* Have a `last_property_mut` and `last_when_mut` for property action attributes?
    - Useful in general, for example, an attribute that sets the importance.
    - Can't be done, because properties may be ignored if their importance is less.
    - Need to change widget expansion to let build action be pushed before properties.
        - And WhenInfo.
    - Lets focus, we need to modify the when before push.
* When easing attribute.    
    - Right now can't instantly lit a button then ease-out on mouse leave.
    - In CSS we can set the "mouse-out" transition in the default selector and the "mouse-in" transition in the hover selector.
    - Need an `EasingWhenVar` that knows that it owns a `WhenVar`, accessing the conditions directly?
        - The `EasingWhenVar` creation is inside the property_build_action.
            - So the build action can't just be `I -> I`, it needs the when metadata.
            - Maybe we can have "when-build-action" be just a `Box<dyn Any + Send + Sync>`.
                - Also needs the default action for when without default?
            - And the build action, `(I, &[WhenIdx?, Box<dyn Any + Send + Sync>>])`.
                - In our case the Any is `(Duration, Box<dyn EasingFn>)`.
            - How do we find what when in the WhenVar corresponds to each metadata?
                - Can just use `usize`?
            - How does the build action retrieves the WhenVar?
                - Use `AnyVar::as_any().downcast_ref::<types::ContextualizedVar<T, ArcWhenVar<T>>>()`.
                - The `EasingWhenVar` is contextualized too?
                    - Yes, we need to get the init value to ease between.

* Sort property build actions by importance?
    - Right now we just have one, `easing` but can be many.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?