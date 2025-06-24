# Crates

This directory contains all the crates that are published and developed by the Zng project.

The main crate is `zng`, it re-exports the main API of the other crates in well documented and organized modules.

All other crates have the name prefix `zng-`. 

Proc-macro crates are named after their single dependant crate, with the `-proc-macros` suffix. The dependant
crate provides the documentation, it re-exports or wraps the proc-macros in declarative macros.

### Foundation

Foundation crates define primitive types and utilities, they are used by all other crates.

- `zng-unique-id`
- `zng-clone-move`
- `zng-handle`
- `zng-txt`
- `zng-unit`
- `zng-state-map`
- `zng-app-context`
- `zng-env`
    - `zng-env-proc-macros`
- `zng-layout`
- `zng-var`
    - `zng-var-proc-macros`
- `zng-task`
    - `zng-task-proc-macros`
- `zng-color`
    - `zng-color-proc-macros`
- `zng-time`
- `zng-tp-licenses`

### View

View-process API and implementation crates. They are named with prefix `zng-view`.

The "view-process" is a platform adapter plus a renderer, it usually runs in a separate process, communicating
over IPC  with the app-process.

Currently all platforms (Linux, Windows and MacOS) are implemented in the `zng-view` crate. If a
future platform cannot integrate with this crate it will be named `zng-view-#platform`. 

- `zng-view-api`
- `zng-view`
    - `zng-view-prebuilt`

### App

Core app-process implementation. Defines the main loop, events, app-extensions API. Connects with 
any external view-process and converts IPC events into RAW_*_EVENT events.

- `zng-app`
    - `zng-app-proc-macros`

### App Extensions

Most of the app services and events are implemented as app extensions. Extension crates are named 
with prefix `zng-ext-`.

- `zng-ext-input`
- `zng-ext-font`
- `zng-ext-fs-watcher`
- `zng-ext-config`
- `zng-ext-l10n`
    - `zng-ext-l10n-proc-macros`
- `zng-ext-hot-reload`
    - `zng-ext-hot-reload-proc-macros`
- `zng-ext-image`
- `zng-ext-clipboard`
- `zng-ext-window`
- `zng-ext-undo`
- `zng-ext-single-instance`

### Widget

Widget and property declarations. Widget crates are named with prefix `zng-wgt-`. These crates
are not fully re-exported by the main crate, this difference between the main and full API provide
a kind of *internal* visibility, providing extra nodes and context values that can be useful for derived widgets
or for custom properties that deeply integrate with a widget.

- `zng-wgt`
- `zng-wgt-style`
- `zng-wgt-input`
- `zng-wgt-access`
- `zng-wgt-transform`
- `zng-wgt-window`
- `zng-wgt-data`
- `zng-wgt-filter`
- `zng-wgt-inspector`
- `zng-wgt-size-offset`
- `zng-wgt-container`
- `zng-wgt-undo`
- `zng-wgt-data-view`
- `zng-wgt-fill`
- `zng-wgt-checkerboard`
- `zng-wgt-layer`
- `zng-wgt-dialog`
- `zng-wgt-undo-history`
- `zng-wgt-image`
- `zng-wgt-text`
- `zng-wgt-text-input`
- `zng-wgt-button`
- `zng-wgt-stack`
- `zng-wgt-panel`
- `zng-wgt-grid`
- `zng-wgt-wrap`
- `zng-wgt-rule-line`
- `zng-wgt-toggle`
- `zng-wgt-menu`
- `zng-wgt-scroll`
- `zng-wgt-settings`
- `zng-wgt-ansi-text`
- `zng-wgt-tooltip`
- `zng-wgt-markdown`
- `zng-wgt-progress`
- `zng-wgt-slider`
- `zng-wgt-material-icons`
- `zng-wgt-webrender-debug`

### Tools

Tools that can be installed by cargo for use in Zng apps.

- `cargo-zng`

### Webrender

The Zng project publishes a fork of [`servo/webrender`] called [`zng-ui/zng-webrender`]. The fork code has minimal
modification, the crates uses by the Zng project are renamed with the `zng-` prefix and some Mozilla specific
dependencies are removed.

[`servo/webrender`]: https://github.com/servo/webrender
[`zng-ui/zng-webrender`]: https://github.com/zng-ui/zng-webrender