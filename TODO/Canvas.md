# Canvas TODO

- Implement canvas API in `zero-ui-view-api`, it should communicate in real time, each command send over IPC without blocking
the main loop.

- Use `pathfinder_canvas`, it should work well with `webrender` as it is a servo project.
    - Check if we can use `swgl` with `pathfinder` or any software renderer.

- Canvas output needs to integrate with the normal image API.

- Need to integrate all types with normal units.

# Widget

- Canvas widget accepts render function as a variable that can update.
- Provide API that takes pre-layout units and layout then using the canvas layout context.
    - This could be a wrapper for the canvas API with different units but same methods.

## Async

- Async, can we use normal `async_hn`?
- When does `webrender` update, use monitor frame rate?
- JavaScript needs to call `requestAnimationFrame` in long running render operations, can do the same with `yield_one`.