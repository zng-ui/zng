# Optimize Release Builds

This document is a guide for how to setup your release process to optimize release builds for speed, binary size and memory use.

## Features

Most projects don't use all the features the `zng` crate provides, the default features and `"view_prebuilt"` are convenient
during development as you can just start implementing your app and just use whatever feature you need, but once you compile a
release build you might find that the final executable is oversized.

The `Cargo.toml` example below setups two features `"dev"` and `"release"`.  The `"dev"` feature is enabled by default
and just enables all the default `zng` features and selects the prebuilt view-process, the `"release"` only selects the features
that are actually used by the release app.

```toml
[dependencies]
zng = { version = "*", default-features = false }

[features]
default = ["dev"]

# features in debug builds
dev = ["zng/default", "zng/view_prebuilt"]

# features in release builds
release = [
    "zng-ipc", # to support running the view-process as a separate process
    "zng/view", # build the view-process to avoid embedding the prebuilt binary
    "zng/view_hardware", # enable GPU renderer, better performance, but uses more RAM
    "zng/view_software", # enable CPU renderer, good to have as fallback
    "zng/crash_handler", # use the crash handler to recover from fatal crashes
    "zng/window", # enable the windows service and `Window!` widget
    "zng/button", # enable the `Button!` widget
]
```

To run with debug features just use `cargo run`.

To build a release use the command `cargo build --release --no-default-features --features release`. 

Note that the [`zng-template`] already setups something like this, if your project was created using `cargo zng new` you can just call `cargo do build-r`.

Note that you may need some external dependencies to build the `"view"` feature. See [the instructions](https://github.com/zng-ui/zng?tab=readme-ov-file#requirements) on the main README for more details.

To quickly find what features your app is actually using you can temporary set `default = ["release"]` and `cargo run`, the rustc error messages
usually show missing features.

## LTO and Codegen Units

The compiler has some parallelism by default to speedup compilation, you can configure the release profile to compile as a single unit
and use more aggressive optimization to reduce size and speedup the release app.

```toml
[profile.release]
lto = "fat"
codegen-units = 1
```

The `Cargo.toml` fragment above shows an override to the `release` profile that does not split the build into separate code units and enables link time optimization. This configuration will give you the best performance possible in release builds with the stable compiler, at the cost of compilation time.

## Binary Size

To optimize specifically for binary size you can use optimization level `"z"`.

The `Cargo.toml` fragment below show changes you can add to the previous examples to optimize for size.

```toml
[profile.release]
opt-level = "z" # instruct the compiler to optimize for size
```

Note that the [`zng-template`] already setups something like this, you just need to fill in the release features on each crate and call `cargo do build-r -z`.

### Nightly

If your project safety constraints allows the nightly compiler and an unstable feature you can enable `-Zshare-generics` to reduce size even more:

**Windows**
```prompt
set RUSTFLAGS=-Zshare-generics -C link-args=-znostart-stop-gc
cargo +nightly build --release
```

**Unix**
```prompt
RUSTFLAGS="-Zshare-generics -C link-args=-znostart-stop-gc" cargo +nightly build --release
```

In the command line example above the `RUSTFLAGS` is set to enable the `share-generics` feature that reduces many monomorphised copies of generic functions and the `cargo +nightly` compiler is used because share-generics is unstable.

The example also sets the `-C link-args=-znostart-stop-gc`, this is to workaround a nightly only linker issue that affects a dependency of `zng`. Note that every nightly release can cause all kinds of issues and the `zng` project is only officially supported on the latest stable Rust release. This optimization was tested on the `2025-01-08` nightly release.

The [`zng-template`] already setups something like this, call `cargo do build -r -z --bleed` to compile with nightly optimizations.

[`zng-template`]: https://github.com/zng-ui/zng-template

## Memory Usage

A typical Zng app runs three processes, `crash-handler-process`, `app-process` and `view-process`. Usually on startup the view-process
will use the most memory, followed by the app-process.

### View Process

The view-process uses the most memory when running on a GPU, small apps like small tools with simple UIs should consider building
with only the `"view_software"` and running the window with `RenderMode::Software`, the memory gains are significant, specially on Windows.

**Windows**

On Windows you can also try running on ANGLE, this is an adapter mode that runs on the GPU using DirectX instead of OpenGL.
DirectX drivers are more optimized on Windows and use less RAM. See `zng-view-angle` for setup details.

Note that ANGLE requires deploying two DLLs, these will add an extra 5MB to the binary payload.

### App Process

The app-process typically does not allocate much on startup, if you already compiled with only the features required by your app.

More optimization will depend on the app, you can build with `"memory_profiler"` to record a DHAT heap allocation trace, see `zng::app::memory_profiler`
for details.

Note that resource services like `IMAGES` cache by default, the services can purge cache, you can also disable caching per resource, `Image! { img_cache = false; }` will not cache the images it loads.

### Crash Handler Process

The crash-handler-process is already optimized and should use very little memory, running it is recommended if you don't have
an alternative crash handling setup. To run without crash handler simply don't compile with `"crash_handler"` feature.
