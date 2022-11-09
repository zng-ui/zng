* Test property generics `value::<bool> = true; when *#is_something { value::<u32> = 32; }`.
    - Panic on init.
```
use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            child = toggle! {
                value::<u32> = 0;
        
                when *#is_hovered {
                    value::<u64> = 1;
                }
            };
        }
    });
}
```

* Const errors don't show if  there is a compile error, so the when `!foo::ALLOWED_IN_WHEN_EXPR` does not show.
    - Generate a placeholder `__w_0__`?

* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Merge `Property.new/new_when` into a single constructor that uses some kind of provider interface.
    - Like the nightly Any provider.

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.
* Implement all `todo!` code.