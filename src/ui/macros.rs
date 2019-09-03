/// Delegate the methods in an [Ui] implementation for `$T` to methods with the same
/// signature in a trait `$Del` that is also implemented by `$T`.
///
/// # Example
/// ```rust
/// pub struct Foo<T, F> {
///     child: T,
///     handler: F
/// }
///
/// impl<T: Ui, F> UiContainer for Foo<T, F> {
///    type Child = T;
///
///    fn child(&self) -> &Self::Child {
///        &self.child
///    }
///
///    fn child_mut(&mut self) -> &mut Self::Child {
///        &mut self.child
///    }
///
///    fn into_child(self) -> Self::Child {
///        self.child
///    }
/// }
///
/// impl<T: Ui + 'static, F> Ui for Foo<T, F> {
///     delegate_ui_methods!(UiContainer, Foo<T, F>);
/// }
/// ```
///
/// # Note
///
/// Try [delegate_ui] first before using this.
#[macro_export]
macro_rules! delegate_ui_methods {
    ($Del:ident) => {
        fn measure(&mut self, available_size: $crate::ui::LayoutSize) -> $crate::ui::LayoutSize {
            $Del::measure(self, available_size)
        }

        fn arrange(&mut self, final_size: $crate::ui::LayoutSize) {
            $Del::arrange(self, final_size)
        }

        fn render(&self, rc: &mut $crate::ui::NextFrame) {
            $Del::render(self, rc)
        }

        fn keyboard_input(&mut self, input: &$crate::ui::KeyboardInput, update: &mut $crate::ui::NextUpdate) {
            $Del::keyboard_input(self, input, update)
        }

        fn focused(&mut self, focused: bool, update: &mut $crate::ui::NextUpdate) {
            $Del::focused(self, focused, update)
        }

        fn mouse_input(&mut self, input: &$crate::ui::MouseInput, hits: &$crate::ui::Hits, update: &mut $crate::ui::NextUpdate) {
            $Del::mouse_input(self, input, hits, update)
        }

        fn mouse_move(&mut self, input: &$crate::ui::MouseMove, hits: &$crate::ui::Hits, update: &mut $crate::ui::NextUpdate) {
            $Del::mouse_move(self, input, hits, update)
        }

        fn mouse_entered(&mut self, update: &mut $crate::ui::NextUpdate) {
            $Del::mouse_entered(self, update);
        }

        fn mouse_left(&mut self, update: &mut $crate::ui::NextUpdate) {
            $Del::mouse_left(self, update);
        }

        fn close_request(&mut self, update: &mut $crate::ui::NextUpdate) {
            $Del::close_request(self, update)
        }

        fn point_over(&self, hits: &$crate::ui::Hits) -> Option<$crate::ui::LayoutPoint> {
            $Del::point_over(self, hits)
        }
    };
}

/// Implements [Ui] for `$T` by delegating Ui methods to methods with the same
/// signature in a trait `$Del` that is also implemented by `$T`.
/// # Example
/// ```rust
/// pub struct Foo {}
///
/// impl UiLeaf for Foo {
///     fn render(&self, _: &mut NextFrame) {}
/// }
/// delegate_ui!(UiLeaf, Foo);
/// ```
///
/// You can also have a generic child type `TChild: Ui + 'static`.
///
/// ```rust
/// pub struct Bar<T> {
///     child: T,
/// }
///
/// impl<T: Ui> UiContainer for Bar<T> {
///     type Child = T;
///
///     fn child(&self) -> &Self::Child {
///         &self.child
///     }
///
///     fn child_mut(&mut self) -> &mut Self::Child {
///         &mut self.child
///     }
///
///     fn into_child(self) -> Self::Child {
///         self.child
///     }
/// }
/// delegate_ui!(UiContainer, Bar<T>, T);
/// ```
/// # Note
/// To use more complex generic signatures see [delegate_ui_methods].
#[macro_export]
macro_rules! delegate_ui {
    ($Del:ident, $T:ty) => {
        impl $crate::ui::Ui for $T {
            delegate_ui_methods!($Del);
        }
    };

    ($Del:ident, $T:ty, $TChild:ident) => {
        impl<$TChild: Ui + 'static> $crate::ui::Ui for $T {
            delegate_ui_methods!($Del);
        }
    };
}

/// Generates boilerplate code in an `UiContainer` implementation.
///
/// # Example
/// ```rust
/// pub struct Bar<T> {
///     child: T,
/// }
///
/// impl<T: Ui> UiContainer for Bar<T> {
///     delegate_child!(child, T);
/// }
/// ```
/// Expands to:
/// ```rust
/// pub struct Bar<T> {
///     child: T,
/// }
///
/// impl<T: Ui> UiContainer for Bar<T> {
///     type Child = T;
///
///     fn child(&self) -> &Self::Child {
///         &self.child
///     }
///
///     fn child_mut(&mut self) -> &mut Self::Child {
///         &mut self.child
///     }
///
///     fn into_child(self) -> Self::Child {
///         self.child
///     }
/// }
/// ```
#[macro_export]
macro_rules! delegate_child {
    ($child: ident, $TChild: ty) => {
        type Child = $TChild;

        fn child(&self) -> &Self::Child {
            &self.$child
        }

        fn child_mut(&mut self) -> &mut Self::Child {
            &mut self.$child
        }

        fn into_child(self) -> Self::Child {
            self.$child
        }
    };
}

/// Generates boilerplate code in an `UiMultiContainer` implementation.
///
/// # Example
/// ```rust
/// pub struct Bar<T> {
///     child: T,
/// }
///
/// impl<T: Ui> UiContainer for Bar<T> {
///     delegate_child!(child, T);
/// }
/// ```
/// Expands to:
/// ```rust
/// pub struct Bar<T> {
///     child: T,
/// }
///
/// impl<T: Ui> UiContainer for Bar<T> {
///     type Child = T;
///
///     fn child(&self) -> &Self::Child {
///         &self.child
///     }
///
///     fn child_mut(&mut self) -> &mut Self::Child {
///         &mut self.child
///     }
///
///     fn into_child(self) -> Self::Child {
///         self.child
///     }
/// }
/// ```
#[macro_export]
macro_rules! delegate_children {
    ($children: ident, $TChild: ty) => {
        type Child = $TChild;
        type Children = std::slice::Iter<'a, Self::Child>;
        type ChildrenMut = std::slice::IterMut<'a, Self::Child>;

        fn children(&'a self) -> Self::Children {
            self.$children.iter()
        }

        fn children_mut(&'a mut self) -> Self::ChildrenMut {
            self.$children.iter_mut()
        }

        fn collect_children<B: FromIterator<Self::Child>>(self) -> B {
            self.$children.into_iter().collect()
        }
    };
}
