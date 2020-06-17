//! Grid properties.

use crate::{
    core::{
        context::{StateKey, WidgetContext},
        impl_ui_node, property,
        var::{BoxVar, IntoVar, ObjVar},
        LastWidgetId, UiNode,
    },
    prelude::WidgetId,
};
use derive_more as dm;

macro_rules! grid_properties {
    ($(
        $(#[$value_meta:meta])*
        pub struct $Value:ident;
        struct $VarKey:ident;
        struct $Node:ident;
        $(#[$property_meta:meta])*
        pub fn$property:ident;
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

            struct $VarKey;
            impl StateKey for $VarKey {
                type Type = Option<(WidgetId, BoxVar<$Value>)>;
            }

            struct $Node<C: UiNode> {
                child: C,
                var: Option<BoxVar<$Value>>
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for $Node<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    self.child.init(ctx);
                    ctx.event_state.set($VarKey, Some((ctx.widget_id, self.var.take().unwrap())));
                }
            }

            $(#[$property_meta])*
            #[property(context)]
            pub fn $property(child: impl UiNode, value: impl IntoVar<$Value>) -> impl UiNode {
                $Node {
                    child,
                    var: Some(value.into_var().boxed())
                }
            }
        )*
    };
}

grid_properties! {
    /// Grid column index, `0` is the left-most column.
    pub struct Column;
    struct ColumnInit;
    struct ColumnNode;
    /// Sets the grid column the widget aligns too.
    pub fn column;

    /// Grid row index, `0` is the top-most row.
    pub struct Row;
    struct RowInit;
    struct RowNode;
    // Sets the grid row the widget aligns too.
    pub fn row;

    /// Grid column span.
    pub struct ColumnSpan;
    struct ColumnSpanInit;
    struct ColumnSpanNode;
    /// Sets the number of columns the widget occupies.
    pub fn column_span;

    /// Grid row span.
    pub struct RowSpan;
    struct RowSpanInit;
    struct RowSpanNode;
    /// Sets the number of rows the widget occupies.
    pub fn row_span;
}

/// Variables associated with a child widget.
///
/// This type can be used for implementing grid layout containers that support the [`grid`](crate::properties::grid) properties.
///
/// # Grid Implementers
///
/// During `UiNode::init` call `GridVars::new(ctx)` after calling init in a child widget to get its grid related vars.
pub struct GridVars {
    pub column: BoxVar<Column>,
    pub row: BoxVar<Row>,
    pub column_span: BoxVar<ColumnSpan>,
    pub row_span: BoxVar<RowSpan>,
}

impl Default for GridVars {
    fn default() -> Self {
        GridVars {
            column: Column(0).into_var().boxed(),
            row: Row(0).into_var().boxed(),
            column_span: ColumnSpan(1).into_var().boxed(),
            row_span: RowSpan(1).into_var().boxed(),
        }
    }
}

impl GridVars {
    pub fn new(ctx: &mut WidgetContext) -> Self {
        if let Some(&child_widget_id) = ctx.event_state.get(LastWidgetId) {
            macro_rules! get {
                ($Key:ident or $Default:ident($default:expr)) => {
                    ctx.event_state
                        .get_mut($Key)
                        .and_then(|o| o.take())
                        .and_then(|(wid, var)| if child_widget_id == wid { Some(var) } else { None })
                        .unwrap_or_else(|| $Default($default).into_var().boxed())
                };
            }
            GridVars {
                column: get!(ColumnInit or Column(0)),
                row: get!(RowInit or Row(0)),
                column_span: get!(ColumnSpanInit or ColumnSpan(1)),
                row_span: get!(RowSpanInit or RowSpan(1)),
            }
        } else {
            GridVars::default()
        }
    }
}
