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

fn new_child(child: impl UiNode) -> impl UiNode { }
fn new_child_size(child: impl UiNode) -> impl UiNode { }
fn new_child_outer(child: impl UiNode) -> impl UiNode { }
fn new_child_event(child: impl UiNode) -> impl UiNode { }
fn new_child_context(child: impl UiNode) -> impl UiNode { }

fn new_inner(child: impl UiNode) -> impl UiNode { }
fn new_size(child: impl UiNode) -> impl UiNode { }
fn new_outer(child: impl UiNode) -> impl UiNode { }
fn new_event(child: impl UiNode) -> impl UiNode { }
fn new(child: impl UiNode) -> W { }

// ### rename, new_child -> new_child_inner and new -> new_context?
// 
// #### For:
//
// * Gives users a hint of how it works.
// * Lets users define their own `pub fn new`.

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
// * They cannot be made from more then one property.
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