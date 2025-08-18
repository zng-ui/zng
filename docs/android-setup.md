# Android Setup

Cross compilation for Android requires some setup. The project [default template] provides most of this setup, 
you only need to install some packages:

* Build dependencies:
    - Install Android Studio or the Android Command-Line Tools, use the Studio UI or the `sdkmanager` tool to install:
        - Android SDK Build Tools.
        - NDK.
        - Android SDK Platform Tools.
    - Set the `ANDROID_HOME` and `ANDROID_NDK_HOME` environment variables.
    - Install [cargo-ndk].
    - Install one or more Rust targets for Android, we test using `aarch64-linux-android`.
    - If you are using the [default template] or just want to build the example you are done.

* Crate setup:
    - Enable the `"view"` feature as prebuilt is not supported for Android.
    - Enable the `"android_native_activity"` feature.
    - Set `crate-type = ["cdylib"]`, add a `lib.rs` file with the [`android_main`] function.
    - Call `init_android_app`, [`android_install_res`] and [`run_same_process`] to run, multi-process is not supported.
    - Program the [Build Script](#build-script) to copy and link the `libc++_shared.so` library.

* Build Setup:
    - Append `RUSTFLAGS` with `-Clink-arg=-z -Clink-arg=nostart-stop-gc`.
    - Use `cargo ndk` to wrap your build call.
    - If you just want the built the binary you are done.

* Build APK Setup:
    - Create a staging folder, `app.apk/`.
    - Add a `AndroidManifest.xml` file.
        - Declare the activity: `android:name="android.app.NativeActivity"`.
        - Declare events support: `android:configChanges="orientation|screenSize|screenLayout|keyboardHidden"`.
        - See [test example].
    - Copy binary to `app.apk/lib/<platform>/<binary>.so`.
    - Copy resources to `app.apk/assets/res/`.
    - Create a `app.apk/build.zr-apk` file and call use `cargo zng res --pack ..` to build.

Android cross compilation is tested for macOS, Ubuntu and Windows, see the "check-android*" jobs in [ci.yml],
the [multi](examples/multi/) example and the [build-apk] task in the `do` tool. Also see `cargo zng res --tool apk`
for help on how to sign the APK.

[ci.yml]: ../.github/workflows/ci.yml
[multi]: ../examples/multi/
[build-apk]: https://github.com/search?q=repo%3Azng-ui%2Fzng%20fn%20build_apk(&type=code
[default template]: https://github.com/zng-ui/zng-template
[cargo-ndk]: https://crates.io/crates/cargo-ndk
[test example]: https://github.com/zng-ui/zng/blob/main/tools/cargo-do/src/build-apk-manifest.xml
[`android_main`]: https://zng-ui.github.io/doc/zng/env/macro.init.html#android-start
[`android_install_res`]: https://zng-ui.github.io/doc/zng_env/fn.android_install_res.html
[`run_same_process`]: https://zng-ui.github.io/doc/zng/view_process/default/fn.run_same_process.html

## Build Script

Building requires linking to Android's `c++_shared` library. You can configure the crate's `build.rs` script
to copy and link using this code:

```rust
use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        android();
    }
}

// copy and link "c++_shared"
fn android() {
    println!("cargo:rustc-link-lib=c++_shared");

    if let Ok(output_path) = env::var("CARGO_NDK_OUTPUT_PATH") {
        let output_path = PathBuf::from(output_path);

        let sysroot_libs_path = PathBuf::from(env::var_os("CARGO_NDK_SYSROOT_LIBS_PATH").unwrap());
        let lib_path = sysroot_libs_path.join("libc++_shared.so");

        let output_path = output_path.join(env::var("CARGO_NDK_ANDROID_TARGET").unwrap());
        let _ = fs::create_dir(&output_path);

        let output_path = output_path.join("libc++_shared.so");
        std::fs::copy(lib_path, &output_path).unwrap();
        println!("cargo:rerun-if-changed={}", output_path.display());
    }
    println!("cargo:rerun-if-env-changed=CARGO_NDK_OUTPUT_PATH");
    println!("cargo:rerun-if-env-changed=CARGO_NDK_ANDROID_TARGET");
}
```

## Backtraces

To log backtraces you must ensure:

* `android:extractNativeLibs="true"` is set on the Android manifest XML (in the `application` element).
* Symbols must not be stripped, use `--no-strip` with  `cargo do build-apk` or with `cargo ndk`.
* Use the [`backtrace`] crate to collect the backtraces, there is a bug on Rust's `std` backtrace (see [rust#121033]).
  If you are debugging a panic you have to add a `println!("{:?}", backtrace::Backtrace::new())` just before the panic code.

[`backtrace`]: https://crates.io/crates/backtrace
[rust#121033]: https://github.com/rust-lang/rust/issues/121033
