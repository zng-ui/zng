use super::{AnyUi, LayoutRect, LayoutSize, RenderContext, Ui};

pub struct StackChild {
    child: AnyUi,
    rect: LayoutRect,
}

impl StackChild {
    pub fn new(child: impl Ui + 'static) -> Self {
        StackChild {
            child: child.as_any(),
            rect: LayoutRect::default(),
        }
    }
}

pub trait IntoStackChildren {
    fn into(self) -> Vec<StackChild>;
}

impl<T: Ui + 'static> IntoStackChildren for Vec<T> {
    fn into(self) -> Vec<StackChild> {
        self.into_iter().map(StackChild::new).collect()
    }
}

macro_rules! impl_tuples {
    ($TH:ident, $TH2:ident, $($T:ident, )* ) => {
        impl<$TH, $TH2, $($T, )*> IntoStackChildren for ($TH, $TH2, $($T,)*)
        where $TH: Ui + 'static, $TH2: Ui + 'static, $($T: Ui + 'static, )*
        {
            #[allow(non_snake_case)]
            fn into(self) -> Vec<StackChild> {
                let ($TH, $TH2, $($T,)*) = self;
                vec![StackChild::new($TH), StackChild::new($TH2),  $(StackChild::new($T), )*]
            }
        }
        impl_tuples!($( $T, )*);
    };

    () => {};
}
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

macro_rules! stack {
    ($Stack: ident, $stack_size: ident, $length_size: ident, $dimension: ident) => {
        pub struct $Stack {
            children: Vec<StackChild>,
        }
        impl $Stack {
            pub fn new(children: impl IntoStackChildren) -> Self {
                $Stack {
                    children: children.into(),
                }
            }
        }
        impl Ui for $Stack {
            type Child = AnyUi;

            fn for_each_child(&mut self, mut action: impl FnMut(&mut Self::Child)) {
                for c in self.children.iter_mut() {
                    action(&mut c.child)
                }
            }
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
            fn render(&mut self, rc: &mut RenderContext) {
                for c in self.children.iter_mut() {
                    rc.push_child(&mut c.child, &c.rect);
                }
            }
        }
    };
}

stack!(HStack, width, height, x);
stack!(VStack, height, width, y);

pub fn h_stack(children: impl IntoStackChildren) -> HStack {
    HStack::new(children)
}

pub fn v_stack(children: impl IntoStackChildren) -> VStack {
    VStack::new(children)
}

///
pub struct ZStack {
    children: Vec<StackChild>,
}
impl ZStack {
    pub fn new(children: impl IntoStackChildren) -> Self {
        ZStack {
            children: children.into(),
        }
    }
}
impl Ui for ZStack {
    type Child = AnyUi;

    fn for_each_child(&mut self, mut action: impl FnMut(&mut Self::Child)) {
        for c in self.children.iter_mut() {
            action(&mut c.child)
        }
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut desired_size = LayoutSize::default();

        for c in self.children.iter_mut() {
            c.rect.size = c.child.measure(available_size);
            desired_size = desired_size.max(c.rect.size);
        }

        desired_size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        for c in self.children.iter_mut() {
            c.rect.size = c.rect.size.min(final_size);
            c.child.arrange(c.rect.size);
        }
    }

    fn render(&mut self, rc: &mut RenderContext) {
        for c in self.children.iter_mut() {
            rc.push_child(&mut c.child, &c.rect);
        }
    }
}
pub fn z_stack(children: impl IntoStackChildren) -> ZStack {
    ZStack::new(children)
}
