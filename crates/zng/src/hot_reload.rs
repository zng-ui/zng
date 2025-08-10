//! Hot reloading instrumentation macros and service.
//!
//! Hot reloading rebuilds an instrumented library and automatically re-inits widgets that
//! are using marked nodes, properties, all without needing to restart the application.
//!
//! This feature is very useful when developing something that requires interactive feedback adjustments, but
//! is does require some setup.
//!
//! # Setup
//!
//! First your project must be split into two crates, a binary and a library. The binary crate runs the app like normal
//! it depends on the library crate and `zng` with `"hot_reload"` feature. The library crate is the one that will be
//! instrumented for hot reloading.
//!
//! First in the `Cargo.toml` for the library crate add:
//!
//! ```toml
//! [lib]
//! crate-type = ["lib", "cdylib"]
//! ```
//!
//! Then in the library crate `src/lib.rs` root add a call to the [`zng_hot_entry!`] item macro:
//!
//! ```
//! zng::hot_reload::zng_hot_entry!();
//! ```
//!
//! Then set the [`hot_node`] attribute in node or property functions that you are developing:
//!
//! ```
//! use zng::{prelude::*, prelude_wgt::*};
//! # zng::hot_reload::zng_hot_entry!();
//!
//! #[hot_node]
//! pub fn hello_text(input: impl IntoVar<bool>) -> UiNode {
//!     let input = input.into_var();
//!     Text! {
//!         txt = greeting_text();
//!         widget::background_color = rgb(0, 100, 0);
//!         when *#{input} {
//!             font_weight = FontWeight::BOLD;
//!         }
//!     }
//! }
//!
//! fn greeting_text() -> Txt {
//!     "Hello!".into()
//! }
//!
//! fn other_ui() -> UiNode {
//!     Container! {
//!         child = hello_text(true);
//!         text::font_size = 2.em();
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! In the example above the `hello_text` function is marked for hot reload, any change in the library crate
//! will trigger a rebuild and widget reinit.
//!
//! In the example you can change anything except the signature of `hello_text`, changes inside the function or
//! inside any other item used by the function will hot reload, you can add or remove properties, replace
//! the `Text!` widget with some other node type, even add Cargo dependencies and use their items.
//!
//! Changes in other *cold nodes* that only contextually affect the hot node will trigger a hot reload,
//! **but will not affect** the hot node, in the example the `font_size` set in `other_ui` affects the
//! hot node even after reload, but the value is fixed at `2.em()`, if you change it the changes are ignored.
//!
//! # How It Works
//!
//! On app init, if at least one `#[hot_node]` is set, all the library crate files are monitored for changes, any change triggers a
//! background rebuild, when the rebuild is finished all `#[hot_node]` functions or properties reinit the related widget,
//! on reinit the new compiled code will run.
//!
//! ## Limitations
//!
//! There are some limitations to the current implementation:
//!
//! ##### Only Node Functions
//!
//! Currently this is only implemented for node functions, this covers all property nodes, intrinsic nodes and functions like
//! in the example above that instantiate widgets, but the widget type must implement `IntoUiNode`, widgets that build different types
//! cannot be hot reloaded, because of this the `Window!` widget cannot be hot reloaded.
//!
//! ##### Limited Function Signature
//!
//! Some input types are not supported for the hot node function. Only the `impl` generics supported by [`property`] and
//! types that are `Clone + Any + Send` are supported. A compile time error is generated if you attempt to use an invalid function
//! signature. Only the output type `UiNode` is supported. Generic properties (named generic params) are also not supported.
//!
//! ##### Rebuild Speed
//!
//! The rebuild speed is only as fast as Rust incremental compilation, it should be pretty fast for small changes,
//! but if your library crate grows large you might want to create a separate *design library* where you place
//! only the nodes under current interactive development.
//!
//! You need to make sure that the same Cargo feature set is used to rebuild, by default the command is `cargo build`, interrupted
//! as soon as the hot library rebuilds. The build command can be overridden using [`HOT_RELOAD.rebuilder`], see the method
//! documentation for more details.
//!
//! [`HOT_RELOAD.rebuilder`]: HOT_RELOAD::rebuilder
//!
//! ##### Rebuild Races IDE for Target Lock
//!  
//! The rebuild uses the same target directory used by `cargo check/clippy`, this means that if your IDE (Rust Analyzer) runs
//! these checks it will race the hot reload rebuild process to acquire the exclusive lock to the target dir. If you are seeing
//! this interference try pausing your IDE analyzer before running.
//!
//! ##### Any Change Reloads All Hot Nodes
//!
//! Any change on the crate triggers a rebuild and all hot nodes reinit because of it. You can set `#[hot_node]` on multiple functions
//! at a time, but this will cause large parts of the screen to reload. It is recommenced that you only set it on functions
//! under iterative development.
//!
//! Hot node reinit reloads the entire tree branch, so descendants of hot nodes are reinited too. This may cause some state to be lost,
//! in particular all state inited inside the hot node will be reinited.
//!
//! ##### Hot Libraries Don't Unload
//!
//! Every hot reload represents a new version of the library reloading and the previous one cannot be unloaded because static references
//! to it might persist in the program, usually `&'static str` texts captured during info rebuild.
//!
//! [`property`]: crate::widget::property#input-types
//!
//! # Full API
//!
//! See [`zng_ext_hot_reload`] for the full hot reload API.

/// Expands an UI node function into a hot reloading one.
///
/// See the [module] level documentation for more details about hot reloading.
///
/// [module]: crate::hot_reload
///
/// # Attribute
///
/// This attribute has one optional argument, a string literal that uniquely identifies the function among all other
/// hot node functions. The default name is only the function name, so you can use this argument to resolve name conflicts.
///
/// # Limitations
///
/// This attribute only accepts inputs with a single name, no destructuring, and of type that is `Clone + Any + Send` or
/// the `impl` generics supported by [`property`]. Unlike property this function does not support named generic parameters.
///
/// The function output type must be `UiNode`, the attribute will change the internal node type.
///
/// [`property`]: crate::widget::property#input-types
#[cfg(hot_reload)]
pub use zng_ext_hot_reload::hot_node;

/// Declare the dynamic library hot reload entry.
///
/// This must be called at the root (`src/lib.rs`) of the library that will hot reload. See the [module] level
/// documentation for more details.
///
/// [module]: crate::hot_reload
#[cfg(hot_reload)]
pub use zng_ext_hot_reload::zng_hot_entry;

#[cfg(hot_reload)]
pub use zng_ext_hot_reload::{BuildArgs, BuildError, HOT_RELOAD};

pub use zng_unique_id::{hot_static, hot_static_ref, lazy_static, lazy_static_init};
