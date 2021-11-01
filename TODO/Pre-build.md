# Pre-Build TOOD

Using the C compatible `staticlib` does not make the pre-build compatible across different compiler releases.

Pre-build dependencies are an [open issue](https://github.com/rust-lang/cargo/issues/1139).

* Embedded DLL extracted every time it opens?
   Tensorflow does this, but there are problems with cleanup: https://github.com/tensorflow/tensorflow/issues/18397
        