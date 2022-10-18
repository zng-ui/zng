* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement all `todo!` code.

# All Rc Rewrite

* Refactor API to remove all return references.
    - Like `UiNode::try_state` or `UiNodeList`.
    - Implement a visitor `UiNode::with_info(f: FnOnce(&WidgetInfo) -> R) -> Option<R>` that groups every thing.
        - This lets we add stuff a lot quickly and without breaking changes too, right now we have multiple info related methods.

# All Dyn Rewrite

* Replace "takeout" with `RcNode<BoxedUiNode>`, `RcWidget`, `RcNodeList`, `RcWidgetHandler`.
* Finish implementing new dynamic widget.
    - We have dynamic at the info level, need dynamic at the instantiated level?
    - Implement helper methods for doing things like moving a property to the top of the pile of its own priority.
* Implement new base widget.
* Test some widgets using the new API.
* Refactor all widgets to use the new API.
* Remove all previous proc-macros.
* Remove custom docs stuff.
* Update docs of new macros.

* Refactor `UiNodeList` and `WidgetList` to be actually boxable.

* Merge.

```rust
use zero_ui::core::{property::*, var::*, *};

fn main() {
    let _wgt = bar! {
        when *#is_state {
            basic_prop = false;
        }
    };
}

/// Property docs.
#[property2(context, default(true, None))]
pub fn boo<T: VarValue>(child: impl UiNode, boo: impl IntoVar<bool>, too: impl IntoVar<Option<T>>) -> impl UiNode {
    let _ = (boo, too);
    tracing::error!("boo must be captured by the widget");
    child
}

///
#[property2(context, default(true))]
pub fn basic_prop(child: impl UiNode, boo: impl IntoVar<bool>) -> impl UiNode {
    let _ = boo;
    child
}

///
#[property2(context)]
pub fn is_state(child: impl UiNode, s: StateVar) -> impl UiNode {
    let _ = s;
    child
}

/// Widget docs.
#[widget2($crate::bar)]
pub mod bar {
    use super::*;

    pub use super::boo as other;

    properties! {
        other = true, Some(32);

        when *#is_state {
            basic_prop = true;
        }
    }

    fn build(_: WidgetBuilder) -> NilUiNode {
        NilUiNode
    }
}

/// Widget docs.
#[widget2($crate::foo)]
pub mod foo {
    use super::*;

    properties! {
        boo = true, Some(32);
    }

    fn build(_: WidgetBuilder) -> NilUiNode {
        NilUiNode
    }
}

/// Widget docs.
#[widget2($crate::zap)]
pub mod zap {
    use super::*;

    inherit!(foo); // not expanded in correct order
    inherit!(bar);

    properties! {
        other = true, Some(33);
    }

    fn build(_: WidgetBuilder) -> NilUiNode {
        NilUiNode
    }
}

```