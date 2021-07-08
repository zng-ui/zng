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

// Input: optional captures, no property.
fn new_child() -> impl UiNode { }
// Input: child node wrapped in child inner properties.
fn new_child_inner(child: impl UiNode) -> impl UiNode { }
// Input: new_child_inner output wrapped in child size properties.
fn new_child_size(child: impl UiNode) -> impl UiNode { }
// Input: same thing.
fn new_child_outer(child: impl UiNode) -> impl UiNode { }
fn new_child_event(child: impl UiNode) -> impl UiNode { }
fn new_child_context(child: impl UiNode) -> impl UiNode { }

// Input: new_child_context output wrapped in inner properties.
fn new_inner(child: impl UiNode) -> impl UiNode { }
// Input: new_inner output wrapped in size properties.
fn new_size(child: impl UiNode) -> impl UiNode { }
// Input: same
fn new_outer(child: impl UiNode) -> impl UiNode { }
fn new_event(child: impl UiNode) -> impl UiNode { }
// Input: new_event output wrapped in context properties.
// Output: the widget type, can add private context nodes here before creating the final widget instance.
fn new(child: impl UiNode) -> W { }

// # Private Properties
//
// ## Advantages:
//
// * They can be any property.
// * Easy to use with multiple properties of the same priority.
// * Easier to learn?
//
// ## Disadvantages:
//
// * They cannot be made from more than one property.
// * Most of the time the code is private for the widget, 
//   so a property must be declared just for the widget.
// * You need to set it to a value that will probably not change.

properties! {
    #[private]
    property = value;
}

// OR

private! {
    property = value;
}