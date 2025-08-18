#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Data view widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use parking_lot::Mutex;
use std::sync::Arc;

use zng_wgt::prelude::*;

/// Arguments for the [`DataView!`] widget.
///
/// [`DataView!`]: struct@DataView
#[derive(Clone)]
pub struct DataViewArgs<D: VarValue> {
    data: Var<D>,
    replace: Arc<Mutex<(bool, UiNode)>>,
    is_nil: bool,
}
impl<D: VarValue> DataViewArgs<D> {
    /// Reference the data variable.
    ///
    /// Can be cloned and used in the [`set_view`] to avoid rebuilding the info tree for every update.
    ///
    /// [`set_view`]: Self::set_view
    pub fn data(&self) -> &Var<D> {
        &self.data
    }

    /// Get the current data value if [`view_is_nil`] or [`data`] is new.
    ///
    /// [`view_is_nil`]: Self::view_is_nil
    /// [`data`]: Self::data
    pub fn get_new(&self) -> Option<D> {
        if self.is_nil { Some(self.data.get()) } else { self.data.get_new() }
    }

    /// If the current child is nil node.
    pub fn view_is_nil(&self) -> bool {
        self.is_nil
    }

    /// Replace the child node.
    ///
    /// If set the current child node will be deinited and dropped.
    pub fn set_view(&self, new_child: impl IntoUiNode) {
        *self.replace.lock() = (true, new_child.into_node());
    }

    /// Set the view to [`UiNode::nil`].
    pub fn unset_view(&self) {
        self.set_view(UiNode::nil())
    }
}

/// Dynamically presents a data variable.
///
/// # Shorthand
///
/// The `DataView!` macro provides a shorthand init that sets `view` property directly.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt_data_view::*;
/// # use zng_wgt::prelude::*;
/// # fn main() { }
/// # fn shorthand_demo<T: VarValue>(data: impl IntoVar<T>, update: impl WidgetHandler<DataViewArgs<T>>) -> UiNode {
/// DataView!(::<T>, data, update)
/// # }
/// ```
///
/// Note that the first argument is a *turbo-fish* that defines the data type and is required.
///
/// The shorthand is above expands to:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt_data_view::*;
/// # use zng_wgt::prelude::*;
/// # fn main() { }
/// # fn shorthand_demo<T: VarValue>(data: impl IntoVar<T>, update: impl WidgetHandler<DataViewArgs<T>>) -> UiNode {
/// DataView! {
///     view::<T> = {
///         data: data,
///         update: update,
///     };
/// }
/// # }
/// ```
#[widget($crate::DataView {
    (::<$T:ty>, $data:expr, $update:expr $(,)?) => {
        view::<$T> = {
            data: $data,
            update: $update,
        };
    }
})]
pub struct DataView(WidgetBase);
impl DataView {
    widget_impl! {
        /// Spacing around content, inside the border.
        pub zng_wgt_container::padding(padding: impl IntoVar<SideOffsets>);

        /// Content alignment.
        pub zng_wgt_container::child_align(align: impl IntoVar<Align>);

        /// Content overflow clipping.
        pub zng_wgt::clip_to_bounds(clip: impl IntoVar<bool>);
    }
}

/// The view generator.
///
/// The `update` widget handler is used to generate the view from the `data`, it is called on init and
/// every time `data` or `update` are new. The view is set by calling [`DataViewArgs::set_view`] in the widget function
/// args, note that the data variable is available in [`DataViewArgs::data`], a good view will bind to the variable
/// to support some changes, only replacing the view for major changes.
///
/// [`DataView!`]: struct@DataView
#[property(CHILD, widget_impl(DataView))]
pub fn view<D: VarValue>(child: impl IntoUiNode, data: impl IntoVar<D>, update: impl WidgetHandler<DataViewArgs<D>>) -> UiNode {
    let data = data.into_var();
    let mut update = update.cfg_boxed();
    let replace = Arc::new(Mutex::new((false, UiNode::nil())));

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data);
            update.event(&DataViewArgs {
                data: data.clone(),
                replace: replace.clone(),
                is_nil: true,
            });
            let child = std::mem::replace(&mut *replace.lock(), (false, UiNode::nil()));
            if child.0 {
                // replaced
                *c.node() = child.1;
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() {
                update.event(&DataViewArgs {
                    data: data.clone(),
                    replace: replace.clone(),
                    is_nil: c.node().is_nil(),
                });
            }

            update.update();

            let child = std::mem::replace(&mut *replace.lock(), (false, UiNode::nil()));
            if child.0 {
                // replaced
                // skip update if nil -> nil, otherwise updates
                if !c.node().is_nil() || !child.1.is_nil() {
                    c.node().deinit();
                    *c.node() = child.1;
                    c.node().init();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                }
            }
        }
        _ => {}
    })
}
