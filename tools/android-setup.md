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