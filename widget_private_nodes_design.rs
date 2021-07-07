// # New functions for each stage:
// 
// ## Advantages:
// 
// * They can capture properties.
// * They are light-weight.
//
// ## Disadvantages:
//
// * The user needs to lookup existing property priorities when using then in widgets.
// * Only one method for each priority.

fn new_child_inner(child: impl UiNode) -> impl UiNode {
    child
}

fn new_inner(child: impl UiNode) -> impl UiNode {
    child
}

fn new_outer_inner(child: impl UiNode) -> impl UiNode {
    child
}

fn new_event(child: impl UiNode) -> impl UiNode {
    child
}

// # Private Properties
//
// ## Advantages:
//
// * They can be any property.
//
// ## Disadvantages:
//
// * They cannot be made from more then one property.
// * Most of the time the code is private for the widget, 
//   so a property must be declared just for the widget.
// * You need to set it to a value.

properties! {
    #[private]
    property = value;
}

// OR

private! {
    property = value;
}