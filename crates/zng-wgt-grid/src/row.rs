use super::*;

/// Grid row definition.
///
/// This widget is layout to define the actual row height, it is not the parent
/// of the cells, only the `height` property affect the cells.
///
/// See the [`Grid::rows`] property for more details.
///
/// # Shorthand
///
/// The `Row!` macro provides a shorthand init that sets the height, `grid::Row!(1.lft())` instantiates
/// a row with height of *1 leftover*.
#[widget($crate::Row { ($height:expr) => { height = $height; }; })]
pub struct Row(WidgetBase);
impl Row {
    widget_impl! {
        /// Row max height.
        pub max_height(max: impl IntoVar<Length>);

        /// Row min height.
        pub min_height(max: impl IntoVar<Length>);

        /// Row height.
        pub height(max: impl IntoVar<Length>);
    }

    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            access_role = AccessRole::Row;
        }
    }
}

static_id! {
    /// Row index, total in the parent widget set by the parent.
    pub(super) static ref INDEX_ID: StateId<(usize, usize)>;
}

/// If the row index is even.
///
/// Row index is zero-based, so the first row is even, the next [`is_odd`].
///
/// [`is_odd`]: fn@is_odd
#[property(CONTEXT, widget_impl(Row))]
pub fn is_even(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 == 0, |_| false, state)
}

/// If the row index is odd.
///
/// Row index is zero-based, so the first row [`is_even`], the next one is odd.
///
/// [`is_even`]: fn@is_even
#[property(CONTEXT, widget_impl(Row))]
pub fn is_odd(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    widget_state_is_state(child, |w| w.get(*INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 != 0, |_| false, state)
}

/// If the row is the first.
#[property(CONTEXT, widget_impl(Row))]
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

/// If the row is the last.
#[property(CONTEXT, widget_impl(Row))]
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

/// Get the row index.
///
/// The row index is zero-based.
#[property(CONTEXT, widget_impl(Row))]
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

/// Get the row index and number of rows.
#[property(CONTEXT, widget_impl(Row))]
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

/// Get the row index, starting from the last row at `0`.
#[property(CONTEXT, widget_impl(Row))]
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
