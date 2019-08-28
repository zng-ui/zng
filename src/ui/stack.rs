use super::{LayoutRect, LayoutSize, NextFrame, Ui, UiContainer, UiMultiContainer};
use std::iter::FromIterator;

macro_rules! stack {
    ($Stack: ident, $stack_size: ident, $length_size: ident, $dimension: ident) => {
        pub struct $Stack<T> {
            children: Vec<StackSlot<T>>,
        }
        impl<T: Ui> $Stack<T> {
            pub fn new<B: IntoStackSlots<Child = T>>(children: B) -> Self {
                $Stack {
                    children: children.into(),
                }
            }
        }
        impl<'a, T: Ui + 'static> UiMultiContainer<'a> for $Stack<T> {
            delegate_children!(children, StackSlot<T>);

            fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
                let mut total_size = LayoutSize::default();

                available_size.$stack_size = std::f32::INFINITY;
                for c in self.children.iter_mut() {
                    c.rect.size = c.child.measure(available_size);
                    total_size.$length_size = total_size.$length_size.max(c.rect.size.$length_size);
                    total_size.$stack_size += c.rect.size.$stack_size;
                }

                total_size
            }
            fn arrange(&mut self, final_size: LayoutSize) {
                let mut $dimension = 0.0;
                for c in self.children.iter_mut() {
                    c.rect.origin.$dimension = $dimension;
                    c.rect.size.$length_size = c.rect.size.$length_size.min(final_size.$length_size);
                    $dimension += c.rect.size.$stack_size;
                    c.child.arrange(c.rect.size);
                }
            }
            fn render(&self, f: &mut NextFrame) {
                for c in self.children.iter() {
                    f.push_child(&c.child, &c.rect);
                }
            }
        }
        delegate_ui!(UiMultiContainer, $Stack<T>, T);
    };
}

stack!(HStack, width, height, x);
stack!(VStack, height, width, y);

pub fn h_stack<B: IntoStackSlots>(children: B) -> HStack<B::Child> {
    HStack::new(children)
}

pub fn v_stack<B: IntoStackSlots>(children: B) -> VStack<B::Child> {
    VStack::new(children)
}

/// A child in a stack munti-container.
pub struct StackSlot<T> {
    child: T,
    rect: LayoutRect,
}

impl<T> StackSlot<T> {
    pub fn new(child: T) -> Self {
        StackSlot {
            child,
            rect: LayoutRect::default(),
        }
    }
}

pub struct ZStack<T> {
    children: Vec<StackSlot<T>>,
}

impl<T: Ui> UiContainer for StackSlot<T> {
    delegate_child!(child, T);

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.rect.size = self.child.measure(available_size);
        self.rect.size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.rect.size = final_size;
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_child(&self.child, &self.rect);
    }
}
delegate_ui!(UiContainer, StackSlot<T>, T);

impl<'a, T: Ui + 'static> UiMultiContainer<'a> for ZStack<T> {
    delegate_children!(children, StackSlot<T>);
}
delegate_ui!(UiMultiContainer, ZStack<T>, T);

pub trait IntoStackSlots {
    type Child: Ui;
    fn into(self) -> Vec<StackSlot<Self::Child>>;
}

impl<T: Ui + 'static> IntoStackSlots for Vec<T> {
    type Child = T;
    fn into(self) -> Vec<StackSlot<T>> {
        self.into_iter().map(StackSlot::new).collect()
    }
}

macro_rules! impl_tuples {
    ($TH:ident, $TH2:ident, $($T:ident, )* ) => {
        impl<$TH, $TH2, $($T, )*> IntoStackSlots for ($TH, $TH2, $($T,)*)
        where $TH: Ui + 'static, $TH2: Ui + 'static, $($T: Ui + 'static, )*
        {
            type Child = Box<dyn Ui>;

            #[allow(non_snake_case)]
            fn into(self) -> Vec<StackSlot<Box<dyn Ui>>> {
                let ($TH, $TH2, $($T,)*) = self;
                vec![StackSlot::new($TH.into_box()), StackSlot::new($TH2.into_box()),  $(StackSlot::new($T.into_box()), )*]
            }
        }
        impl_tuples!($( $T, )*);
    };

    () => {};
}
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

impl<T: Ui> ZStack<T> {
    pub fn new<B: IntoStackSlots<Child = T>>(children: B) -> Self {
        ZStack {
            children: children.into(),
        }
    }
}

pub fn z_stack<B: IntoStackSlots>(children: B) -> ZStack<B::Child> {
    ZStack::new(children)
}
