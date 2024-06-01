
<!--do doc --readme-examples-->
### `animation`

<img alt='animation screenshot' src='./res/screenshots/animation.png' width='300'>

Source: [animation.rs](./animation.rs)

```console
cargo do run animation
```

Demonstrates animation, easing functions.

### `border`

<img alt='border screenshot' src='./res/screenshots/border.png' width='300'>

Source: [border.rs](./border.rs)

```console
cargo do run border
```

Demonstrates borders, corner radius, multiple borders per widget and clip-to-bounds.

### `button`

<img alt='button screenshot' src='./res/screenshots/button.png' width='300'>

Source: [button.rs](./button.rs)

```console
cargo do run button
```

Demonstrates the button and toggle widgets.

### `calculator`

<img alt='calculator screenshot' src='./res/screenshots/calculator.png' width='300'>

Source: [calculator.rs](./calculator.rs)

```console
cargo do run calculator
```

Simple calculator, demonstrates Grid layout, data context.

### `config`

<img alt='config screenshot' src='./res/screenshots/config.png' width='300'>

Source: [config.rs](./config.rs)

```console
cargo do run config
```

Demonstrates the CONFIG service, live updating config between processes.

### `countdown`

Source: [countdown.rs](./countdown.rs)

```console
cargo do run countdown
```

Demonstrates the TIMERS service, variable mapping.

### `cursor`

<img alt='cursor screenshot' src='./res/screenshots/cursor.png' width='300'>

Source: [cursor.rs](./cursor.rs)

```console
cargo do run cursor
```

Demonstrates each `CursorIcon`, tooltip anchored to cursor.

### `extend_view`

Source: [extend_view.rs](./extend_view.rs)

```console
cargo do run extend_view
```

Demonstrates the `zng-view` extension API and render extensions API.

### `focus`

<img alt='focus screenshot' src='./res/screenshots/focus.png' width='300'>

Source: [focus.rs](./focus.rs)

```console
cargo do run focus
```

Demonstrates the focus service, logical and directional navigation.

### `gradient`

<img alt='gradient screenshot' src='./res/screenshots/gradient.png' width='300'>

Source: [gradient.rs](./gradient.rs)

```console
cargo do run gradient
```

Demonstrates gradient rendering.

### `headless`

<img alt='headless screenshot' src='./res/screenshots/headless.png' width='300'>

Source: [headless.rs](./headless.rs)

```console
cargo do run headless
```

Demonstrates headless apps, image and video rendering.

### `hot_reload`

<img alt='hot_reload screenshot' src='./res/screenshots/hot_reload.png' width='300'>

Source: [hot_reload.rs](./hot_reload.rs)

```console
cargo do run hot_reload
```

Demonstrates the `"hot_reload"` feature.

### `icon`

<img alt='icon screenshot' src='./res/screenshots/icon.png' width='300'>

Source: [icon.rs](./icon.rs)

```console
cargo do run icon
```

Search and copy Material Icons constants.

### `image`

<img alt='image screenshot' src='./res/screenshots/image.png' width='300'>

Source: [image.rs](./image.rs)

```console
cargo do run image
```

Demonstrates image loading, displaying, animated sprites, rendering, pasting.

### `layer`

<img alt='layer screenshot' src='./res/screenshots/layer.png' width='300'>

Source: [layer.rs](./layer.rs)

```console
cargo do run layer
```

Demonstrates the LAYERS service.

### `localize`

<img alt='localize screenshot' src='./res/screenshots/localize.png' width='300'>

Source: [localize.rs](./localize.rs)

```console
cargo do run localize
```

Demonstrates localization.

### `markdown`

<img alt='markdown screenshot' src='./res/screenshots/markdown.png' width='300'>

Source: [markdown.rs](./markdown.rs)

```console
cargo do run markdown
```

Demonstrates the `Markdown!` widget.

### `respawn`

<img alt='respawn screenshot' src='./res/screenshots/respawn.png' width='300'>

Source: [respawn.rs](./respawn.rs)

```console
cargo do run respawn
```

Demonstrates app-process crash handler and view-process respawn.

### `scroll`

<img alt='scroll screenshot' src='./res/screenshots/scroll.png' width='300'>

Source: [scroll.rs](./scroll.rs)

```console
cargo do run scroll
```

Demonstrates the `Scroll!` widget and scroll commands.

### `shortcut`

<img alt='shortcut screenshot' src='./res/screenshots/shortcut.png' width='300'>

Source: [shortcut.rs](./shortcut.rs)

```console
cargo do run shortcut
```

Small utility that displays the pressed key gestures.

### `text`

<img alt='text screenshot' src='./res/screenshots/text.png' width='300'>

Source: [text.rs](./text.rs)

```console
cargo do run text
```

Demonstrates the `Text!` and `TextInput!` widgets. Text rendering, text editor.

### `transform`

<img alt='transform screenshot' src='./res/screenshots/transform.png' width='300'>

Source: [transform.rs](./transform.rs)

```console
cargo do run transform
```

Demonstrates 2D and 3D transforms, touch transforms.

### `window`

<img alt='window screenshot' src='./res/screenshots/window.png' width='300'>

Source: [window.rs](./window.rs)

```console
cargo do run window
```

Demonstrates the window widget, service, state and commands.

<!--do doc --readme #SECTION-END-->

## Adding an Example

Add the new example in `examples/<example-name>.rs`:

```rust
//! Demonstrates foo, bar.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::prelude::*;

use zng::view_process::prebuilt as view_process;

fn main() {
    examples_util::print_info();

    zng::env::init!();
    zng::app::crash_handler::init_debug();
    app_main();
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            title = "Foo Example";
            child = Text!("Bar");
        }
    })
}
```

Register it in `examples/Cargo.toml`:

```toml
[[example]]
name = "<example-name>"
path = "<example-name>.rs"
```

Run the example and test it.

```console
cargo do run <example-name>
```

Optionally, take a screenshot and save it to `examples/res/screenshots/<example-name>.png`. You can take a screenshot using
the inspector window, press `Ctrl+Shift+I` then click the "Save Screenshot" menu.

Run [`oxipng`](https://github.com/shssoichiro/oxipng) or another minifier on the screenshot before committing.

```console
oxipng -o max --strip safe --alpha "examples/res/screenshots/<example-name>.png"
```

Update the auto generated README:

```console
cargo do doc --readme-examples
```

Done.

## Local Example

You can create local examples for manual testing in `/examples/examples/<test>.rs`. These
files are git-ignored and can be run using `cargo do run <test>` without needing to register.
