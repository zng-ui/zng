use super::*;

/// Grid column definition.
///
/// This widget is layout to define the actual column width, it is not the parent
/// of the cells, only the `width` and `align` properties affect the cells.
///
/// See the [`Grid::columns`] property for more details.
///
/// # Shorthand
///
/// The `Column!` macro provides a shorthand init that sets the width, `grid::Column!(1.lft())` instantiates
/// a column with width of *1 leftover*.
#[widget($crate::Column { ($width:expr) => { width = $width; }; })]
pub struct Column(WidgetBase);
impl Column {
    widget_impl! {
        /// Column max width.
        pub max_width(max: impl IntoVar<Length>);

        /// Column min width.
        pub min_width(min: impl IntoVar<Length>);

        /// Column width.
        pub width(width: impl IntoVar<Length>);
    }

    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            access_role = AccessRole::Column;
        }
    }
}

static_id! {
    /// Column index, total in the parent widget set by the parent.
    pub(super) static ref INDEX_ID: StateId<(usize, usize)>;
}

/// If the column index is even.
///
/// Column index is zero-based, so the first column is even, the next [`is_odd`].
///
/// [`is_odd`]: fn@is_odd
#[property(CONTEXT, widget_impl(Column))]
pub fn is_even(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 == 0, |_| false, state)
}

/// If the column index is odd.
///
/// Column index is zero-based, so the first column [`is_even`], the next one is odd.
///
/// [`is_even`]: fn@is_even
#[property(CONTEXT, widget_impl(Column))]
pub fn is_odd(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 != 0, |_| false, state)
}

/// If the column is the first.
#[property(CONTEXT, widget_impl(Column))]
pub fn is_first(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    widget_state_is_state(
        child,
        |w| {
            let (i, l) = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
            i == 0 && l > 0
        },
        |_| false,
        state,
    )
}

/// If the column is the last.
#[property(CONTEXT, widget_impl(Column))]
pub fn is_last(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    widget_state_is_state(
        child,
        |w| {
            let (i, l) = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
            i < l && i == l - 1
        },
        |_| false,
        state,
    )
}

/// Get the column index.
///
/// The column index is zero-based.
#[property(CONTEXT, widget_impl(Column))]
pub fn get_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
    widget_state_get_state(
        child,
        |w, &i| {
            let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0;
            if a != i { Some(a) } else { None }
        },
        |_, &i| if i != 0 { Some(0) } else { None },
        state,
    )
}

/// Get the column index and number of columns.
#[property(CONTEXT, widget_impl(Column))]
pub fn get_index_len(child: impl IntoUiNode, state: impl IntoVar<(usize, usize)>) -> UiNode {
    widget_state_get_state(
        child,
        |w, &i| {
            let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
            if a != i { Some(a) } else { None }
        },
        |_, &i| if i != (0, 0) { Some((0, 0)) } else { None },
        state,
    )
}

/// Get the column index, starting from the last column at `0`.
#[property(CONTEXT, widget_impl(Column))]
pub fn get_rev_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
    widget_state_get_state(
        child,
        |w, &i| {
            let a = w.get(*INDEX_ID).copied().unwrap_or((0, 0));
            let a = a.1 - a.0;
            if a != i { Some(a) } else { None }
        },
        |_, &i| if i != 0 { Some(0) } else { None },
        state,
    )
}
