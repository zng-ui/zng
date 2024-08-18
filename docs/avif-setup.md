# Avif Setup

Compiling `zng-view` (or `zng` with `"view"` feature) and AVIF image format support is tricky, because it depends 
on the native `dav1d` library.

## Crate Setup

On your `Cargo.toml` file add an `avif` feature and an optional dependency to the `image` crate that 
matches the version used by `zng-view`:

```toml
[features]
avif = ["image/avif", "image/avif-native"]

[dependencies]
image = { version = "<same version as zng-view/Cargo.toml>", default-features = false, optional = true }
```

## Build

The general idea for building is to install all packages used by `dav1d`, clone and build `dav1d` and build the crate
with the `avif` feature and the `--cfg=zng_view_image_has_avif` Rust flag.

We recommend building on a container or GitHub workflow.

### On Ubuntu and macOS:

Install Python and Pip:

```console
sudo apt install python3 -y
sudo apt install python3-pip -y
```

Install david Python dependencies:

```console
pip install -U pip
pip install -U wheel setuptools
pip install -U meson ninja
```

Clone and build dav1d:

```console
export DAV1D_DIR=dav1d_dir
export LIB_PATH=lib/x86_64-linux-gnu

git clone --branch 1.3.0 --depth 1 https://github.com/videolan/dav1d.git
cd dav1d
meson build -Dprefix=$HOME/$DAV1D_DIR -Denable_tools=false -Denable_examples=false -Ddefault_library=static --buildtype release
ninja -C build
ninja -C build install

export PKG_CONFIG_PATH=$PKG_CONFIG_PATH:$HOME/$DAV1D_DIR/$LIB_PATH/pkgconfig
export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:$HOME/$DAV1D_DIR/$LIB_PATH
```

Build your crate:

```console
export SYSTEM_DEPS_LINK=static
export RUSTFLAGS=$RUSTFLAGS --cfg zng_view_image_has_avif

cargo build --features avif
```

### On Windows:

Install Python 3 using the latest installer at <https://www.python.org/downloads/>.

Install pip:

```console
py -m ensurepip --upgrade
```

Install david Python dependencies:

```console
pip install -U pip
pip install -U wheel setuptools
pip install -U meson ninja
```

Install NASM using the latest installer at <https://www.nasm.us/pub/nasm/releasebuilds/>.

Clone and build pkg-config and dav1d:

```console
set PKG_CONFIG=c:\build\bin\pkg-config.exe
set PKG_CONFIG_PATH=C:\build\lib\pkgconfig
set PATH=%PATH%;C:\build\bin

git clone --branch meson-glib-subproject --depth 1 https://gitlab.freedesktop.org/tpm/pkg-config.git
cd pkg-config
meson build -Dprefix=C:\build --buildtype release
ninja -C build
ninja -C build install

git clone --branch 1.3.0 --depth 1 https://github.com/videolan/dav1d.git
cd dav1d
meson build -Dprefix=C:\build -Denable_tools=false -Denable_examples=false -Ddefault_library=static --buildtype release
ninja -C build
ninja -C build install
```

Build your crate:

```console
set CC=clang-cl
set CXX=clang-cl
set SYSTEM_DEPS_LINK=static
set RUSTFLAGS=%RUSTFLAGS% --cfg zng_view_image_has_avif

cargo build --features avif
```
