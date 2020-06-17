//! Grid properties.

use crate::{
    core::{
        context::{StateKey, Vars},
        property,
        var::{BoxLocalVar, IntoVar},
        UiNode, Widget,
    },
    properties::set_widget_state,
};
use derive_more as dm;

macro_rules! grid_properties {
    ($(
        $(#[$value_meta:meta])*
        pub struct $Value:ident;
        struct $VarKey:ident;
        $(#[$property_meta:meta])*
        pub fn$property:ident;
        default: $default:expr;
    )*) => {
        $(
            $(#[$value_meta])*
            #[derive(
                Debug, dm::Display,
                Ord, PartialOrd,
                Eq, PartialEq,
                Clone, Copy,
                dm::From, dm::Into,
                dm::Add, dm::Sub,
                dm::AddAssign, dm::SubAssign,
            )]
            pub struct $Value(pub usize);
            impl Default for $Value {
                fn default() -> Self {
                    $Value($default)
                }
            }

            struct $VarKey;
            impl StateKey for $VarKey {
                type Type = BoxLocalVar<$Value>;
            }

            $(#[$property_meta])*
            #[property(context)]
            pub fn $property(child: impl UiNode, value: impl IntoVar<$Value>) -> impl UiNode {
                set_widget_state(child, $VarKey, Box::new(value.into_local()))
            }
        )*
    };
}

grid_properties! {
    /// Grid column index, `0` is the left-most column.
    pub struct Column;
    struct ColumnKey;
    /// Sets the grid column the widget aligns too.
    pub fn column;
    default: 0;

    /// Grid row index, `0` is the top-most row.
    pub struct Row;
    struct RowKey;
    // Sets the grid row the widget aligns too.
    pub fn row;
    default: 0;

    /// Grid column span.
    pub struct ColumnSpan;
    struct ColumnSpanKey;
    /// Sets the number of columns the widget occupies.
    pub fn column_span;
    default: 1;

    /// Grid row span.
    pub struct RowSpan;
    struct RowSpanKey;
    /// Sets the number of rows the widget occupies.
    pub fn row_span;
    default: 1;
}

/// Grid properties getter.
///
/// Grid container implementers can use this to get cell configuration from child widgets.
pub trait GridChildState {
    fn column_var(&self) -> Option<&BoxLocalVar<Column>>;
    fn row_var(&self) -> Option<&BoxLocalVar<Row>>;
    fn column_span_var(&self) -> Option<&BoxLocalVar<ColumnSpan>>;
    fn row_span_var(&self) -> Option<&BoxLocalVar<RowSpan>>;

    fn column_var_mut(&mut self) -> Option<&mut BoxLocalVar<Column>>;
    fn row_var_mut(&mut self) -> Option<&mut BoxLocalVar<Row>>;
    fn column_span_var_mut(&mut self) -> Option<&mut BoxLocalVar<ColumnSpan>>;
    fn row_span_var_mut(&mut self) -> Option<&mut BoxLocalVar<RowSpan>>;

    /// Initializes the local copy of all variables.
    fn init_local(&mut self, vars: &Vars) {
        if let Some(var) = self.column_var_mut() {
            var.init_local(vars);
        }
        if let Some(var) = self.row_var_mut() {
            var.init_local(vars);
        }
        if let Some(var) = self.column_span_var_mut() {
            var.init_local(vars);
        }
        if let Some(var) = self.row_span_var_mut() {
            var.init_local(vars);
        }
    }

    /// Updates the local copy of all variables, returns if any variable updated.
    fn update_local(&mut self, vars: &Vars) -> bool {
        let mut has_update = false;
        if let Some(var) = self.column_var_mut() {
            has_update = var.update_local(vars).is_some();
        }
        if let Some(var) = self.row_var_mut() {
            has_update = var.update_local(vars).is_some();
        }
        if let Some(var) = self.column_span_var_mut() {
            has_update = var.update_local(vars).is_some();
        }
        if let Some(var) = self.row_span_var_mut() {
            has_update = var.update_local(vars).is_some();
        }
        has_update
    }

    #[inline]
    fn column(&self) -> Column {
        self.column_var().map(|v| *v.get_local()).unwrap_or_default()
    }
    #[inline]
    fn row(&self) -> Row {
        self.row_var().map(|v| *v.get_local()).unwrap_or_default()
    }
    #[inline]
    fn column_span(&self) -> ColumnSpan {
        self.column_span_var().map(|v| *v.get_local()).unwrap_or_default()
    }
    #[inline]
    fn row_span(&self) -> RowSpan {
        self.row_span_var().map(|v| *v.get_local()).unwrap_or_default()
    }
}

impl<W: Widget> GridChildState for W {
    #[inline]
    fn column_var(&self) -> Option<&BoxLocalVar<Column>> {
        self.state().get(ColumnKey)
    }
    #[inline]
    fn row_var(&self) -> Option<&BoxLocalVar<Row>> {
        self.state().get(RowKey)
    }
    #[inline]
    fn column_span_var(&self) -> Option<&BoxLocalVar<ColumnSpan>> {
        self.state().get(ColumnSpanKey)
    }
    #[inline]
    fn row_span_var(&self) -> Option<&BoxLocalVar<RowSpan>> {
        self.state().get(RowSpanKey)
    }

    #[inline]
    fn column_var_mut(&mut self) -> Option<&mut BoxLocalVar<Column>> {
        self.state_mut().get_mut(ColumnKey)
    }
    #[inline]
    fn row_var_mut(&mut self) -> Option<&mut BoxLocalVar<Row>> {
        self.state_mut().get_mut(RowKey)
    }
    #[inline]
    fn column_span_var_mut(&mut self) -> Option<&mut BoxLocalVar<ColumnSpan>> {
        self.state_mut().get_mut(ColumnSpanKey)
    }
    #[inline]
    fn row_span_var_mut(&mut self) -> Option<&mut BoxLocalVar<RowSpan>> {
        self.state_mut().get_mut(RowSpanKey)
    }
}
