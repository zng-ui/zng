
<!--do doc --readme-examples-->
### `animation`

<img alt='animation screenshot' src='./animation/res/screenshot.png' width='300'>

Source: [animation/src](./animation/src)

```console
cargo do run animation
```

Demonstrates animation, easing functions.

### `border`

<img alt='border screenshot' src='./border/res/screenshot.png' width='300'>

Source: [border/src](./border/src)

```console
cargo do run border
```

Demonstrates borders, corner radius, multiple borders per widget and clip-to-bounds.

### `button`

<img alt='button screenshot' src='./button/res/screenshot.png' width='300'>

Source: [button/src](./button/src)

```console
cargo do run button
```

Demonstrates the button and toggle widgets.

### `calculator`

<img alt='calculator screenshot' src='./calculator/res/screenshot.png' width='300'>

Source: [calculator/src](./calculator/src)

```console
cargo do run calculator
```

Simple calculator, demonstrates Grid layout, data context.

### `config`

<img alt='config screenshot' src='./config/res/screenshot.png' width='300'>

Source: [config/src](./config/src)

```console
cargo do run config
```

Demonstrates the CONFIG service, live updating config between processes.

### `countdown`

<img alt='countdown screenshot' src='./countdown/res/screenshot.png' width='300'>

Source: [countdown/src](./countdown/src)

```console
cargo do run countdown
```

Demonstrates the TIMERS service, variable mapping.

### `cursor`

<img alt='cursor screenshot' src='./cursor/res/screenshot.png' width='300'>

Source: [cursor/src](./cursor/src)

```console
cargo do run cursor
```

Demonstrates each `CursorIcon`, tooltip anchored to cursor.

### `extend-view`

<img alt='extend-view screenshot' src='./extend-view/res/screenshot.png' width='300'>

Source: [extend-view/src](./extend-view/src)

```console
cargo do run extend-view
```

Demonstrates the `zng-view` extension API and render extensions API.

### `focus`

<img alt='focus screenshot' src='./focus/res/screenshot.png' width='300'>

Source: [focus/src](./focus/src)

```console
cargo do run focus
```

Demonstrates the focus service, logical and directional navigation.

### `gradient`

<img alt='gradient screenshot' src='./gradient/res/screenshot.png' width='300'>

Source: [gradient/src](./gradient/src)

```console
cargo do run gradient
```

Demonstrates gradient rendering.

### `headless`

<img alt='headless screenshot' src='./headless/res/screenshot.png' width='300'>

Source: [headless/src](./headless/src)

```console
cargo do run headless
```

Demonstrates headless apps, image and video rendering.

### `hot-reload`

<img alt='hot-reload screenshot' src='./hot-reload/res/screenshot.png' width='300'>

Source: [hot-reload/src](./hot-reload/src)

```console
cargo do run hot-reload
```

Demonstrates the `"hot_reload"` feature.

### `icon`

<img alt='icon screenshot' src='./icon/res/screenshot.png' width='300'>

Source: [icon/src](./icon/src)

```console
cargo do run icon
```

Search and copy Material Icons constants.

### `image`

<img alt='image screenshot' src='./image/res/screenshot.png' width='300'>

Source: [image/src](./image/src)

```console
cargo do run image
```

Demonstrates image loading, displaying, animated sprites, rendering, pasting.

### `layer`

<img alt='layer screenshot' src='./layer/res/screenshot.png' width='300'>

Source: [layer/src](./layer/src)

```console
cargo do run layer
```

Demonstrates the LAYERS service.

### `localize`

<img alt='localize screenshot' src='./localize/res/screenshot.png' width='300'>

Source: [localize/src](./localize/src)

```console
cargo do run localize
```

Demonstrates localization.

### `markdown`

<img alt='markdown screenshot' src='./markdown/res/screenshot.png' width='300'>

Source: [markdown/src](./markdown/src)

```console
cargo do run markdown
```

Demonstrates the `Markdown!` widget.

### `respawn`

<img alt='respawn screenshot' src='./respawn/res/screenshot.png' width='300'>

Source: [respawn/src](./respawn/src)

```console
cargo do run respawn
```

Demonstrates app-process crash handler and view-process respawn.

### `scroll`

<img alt='scroll screenshot' src='./scroll/res/screenshot.png' width='300'>

Source: [scroll/src](./scroll/src)

```console
cargo do run scroll
```

Demonstrates the `Scroll!` widget and scroll commands.

### `shortcut`

<img alt='shortcut screenshot' src='./shortcut/res/screenshot.png' width='300'>

Source: [shortcut/src](./shortcut/src)

```console
cargo do run shortcut
```

Small utility that displays the pressed key gestures.

### `text`

<img alt='text screenshot' src='./text/res/screenshot.png' width='300'>

Source: [text/src](./text/src)

```console
cargo do run text
```

Demonstrates the `Text!` and `TextInput!` widgets. Text rendering, text editor.

### `transform`

<img alt='transform screenshot' src='./transform/res/screenshot.png' width='300'>

Source: [transform/src](./transform/src)

```console
cargo do run transform
```

Demonstrates 2D and 3D transforms, touch transforms.

### `window`

<img alt='window screenshot' src='./window/res/screenshot.png' width='300'>

Source: [window/src](./window/src)

```console
cargo do run window
```

Demonstrates the window widget, service, state and commands.

<!--do doc --readme #SECTION-END-->

## Adding an Example

Add the new example crate in `examples/<example-name>`:

`examples/<example-name>/Cargo.toml`

```
[package]
name = "<example-name>"
version = "0.0.0"
publish = false
edition = "2021"

[dependencies]
zng = { path = "../../crates/zng", features = ["view_prebuilt"] }
```

`examples/<example-name>/src/main.rs`: 

```rust
//! Demonstrates foo, bar.
use zng::prelude::*;

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

Run the example and test it.

```console
cargo do run <example-name>
```

Optionally, take a screenshot and save it to `examples/<example-name>/res/screenshot.png`. You can take a screenshot using
the inspector window, press `Ctrl+Shift+I` then click the "Save Screenshot" menu.

Run [`oxipng`](https://github.com/shssoichiro/oxipng) or another minifier on the screenshot before committing.

```console
oxipng -o max --strip safe --alpha "examples/<example-name>/res/screenshot.png"
```

Update the auto generated README:

```console
cargo do doc --readme-examples
```

Done.

## Local Example

You can create a local "example" for manual testing in `/examples/test/`. This dir is gitignored.