# Unix TODO

Things TODO or FIX for Ubuntu builds.

* Implement Wayland backend blit.

* Implement zero_ui_view::config functions if possible.
* No fonts found, caused fallback code to run and fails too, start fixing from fallback.
* Font-kit is producing a lot of warn and error logs.

# Ubuntu Build Log

Things needed to build Ubuntu:

git
rustup
build-essential
--"cargo do install" works--
pkg-config
libssl-dev
cmake
libfreetype6-dev
libfontconfig1-dev
libx11-dev
--"cargo do build" works--
