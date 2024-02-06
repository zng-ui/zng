# Pre-Build TOOD

Pre-build dependencies are an [open issue](https://github.com/rust-lang/cargo/issues/1139).

# Current Status

* We extract an embedded `dylib` to the data-dir, unique name by hash.
* Do we need to care about cleanup?
   - Yes, at least during development (Windows never clears %TMP%).
* Tensorflow does this, but tried to delete the dll every time, did not work on windows: https://github.com/tensorflow/tensorflow/issues/18397

# Distribute

* How to distribute? Binary file in the crate git is not cool, download in build.rs?