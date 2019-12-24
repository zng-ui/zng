use crate::core::*;
use std::iter::FromIterator;

macro_rules! stack {
    ($Stack: ident, $stack_size: ident, $length_size: ident, $dimension: ident) => {
        pub struct $Stack<T> {
            children: Vec<StackEntry<T>>,
            hit_tag: HitTag,
        }
        #[impl_ui_crate(children)]
        impl<T: Ui> $Stack<T> {
            #[inline]
            pub fn new(children: Stack<T>) -> Self {
                $Stack {
                    children: children.stack,
                    hit_tag: HitTag::new_unique(),
                }
            }

            #[Ui]
            fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
                let mut total_size = LayoutSize::default();

                available_size.$stack_size = std::f32::INFINITY;
                for c in self.children.iter_mut() {
                    Ui::measure(c, available_size);
                    total_size.$length_size = total_size.$length_size.max(c.rect.size.$length_size);
                    total_size.$stack_size += c.rect.size.$stack_size;
                }

                total_size
            }

            #[Ui]
            fn arrange(&mut self, final_size: LayoutSize) {
                let mut $dimension = 0.0;
                for c in self.children.iter_mut() {
                    c.rect.origin.$dimension = $dimension;
                    c.rect.size.$length_size = c.rect.size.$length_size.min(final_size.$length_size);
                    $dimension += c.rect.size.$stack_size;
                    Ui::arrange(c, c.rect.size);
                }
            }

            #[Ui]
            fn render(&self, f: &mut NextFrame) {
                {
                    profile_scope!("{}_render", stringify!($Stack));
                    f.push_hit_test(self.hit_tag, LayoutRect::from_size(f.final_size()));
                }

                for c in self.children.iter() {
                    f.push_child(&c.child, &c.rect);
                }
            }

            #[Ui]
            fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
                let r = hits.point_over(self.hit_tag);
                if r.is_some() && self.children.iter().any(|c| Ui::point_over(c, hits).is_some()) {
                    return r;
                }
                None
            }
        }
    };
}

stack!(HStack, width, height, x);
stack!(VStack, height, width, y);

/// Stack the children in a line (X). The first child at the begining (0, 0) the last child
/// at the end (n, 0);
pub fn h_stack<T: Ui>(children: Stack<T>) -> HStack<T> {
    HStack::new(children)
}

/// Stacks the children in a column (Y). The first child at the top (0, 0) the last child at
/// the bottom (0, n).
pub fn v_stack<T: Ui>(children: Stack<T>) -> VStack<T> {
    VStack::new(children)
}

/// Stacks the children on top of each other. The first child at the bottom the last at the top.
pub struct ZStack<T> {
    children: Vec<StackEntry<T>>,
}

#[impl_ui_crate(children)]
impl<T: Ui> ZStack<T> {
    pub fn new(children: Stack<T>) -> Self {
        ZStack {
            children: children.stack,
        }
    }
}

/// Stacks the children on top of each other (Z-index). The first child at the bottom the last at the top.
pub fn z_stack<T: Ui>(children: Stack<T>) -> ZStack<T> {
    ZStack::new(children)
}

/// A child in a stack container.
struct StackEntry<T> {
    child: T,
    rect: LayoutRect,
}

#[impl_ui_crate(child)]
impl<T: Ui> StackEntry<T> {
    pub fn new(child: T) -> Self {
        StackEntry {
            child,
            rect: LayoutRect::default(),
        }
    }

    #[Ui]
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.rect.size = self.child.measure(available_size);
        self.rect.size
    }

    #[Ui]
    fn arrange(&mut self, final_size: LayoutSize) {
        self.rect.size = final_size;
        self.child.arrange(final_size);
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_child(&self.child, &self.rect);
    }
}

/// Stack children builder.
pub struct Stack<U: Ui> {
    stack: Vec<StackEntry<U>>,
}

impl<U: Ui> Default for Stack<U> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Ui> Stack<U> {
    /// Constructs a new empty `Stack<U>`.
    #[inline]
    pub fn new() -> Self {
        Stack { stack: Vec::new() }
    }

    /// Constructs a new, empty `Stack<T>` with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Stack {
            stack: Vec::with_capacity(capacity),
        }
    }

    /// Appends a `child` to the stack, takes and returns `self`
    /// for builder style initialization.
    #[inline]
    pub fn push(mut self, child: U) -> Self {
        self.stack.push(StackEntry::new(child));
        self
    }
}

/// Stack children builder that can take any type of children.
pub type BoxedStack = Stack<Box<dyn Ui>>;

impl Stack<Box<dyn Ui>> {
    /// Appends a `child` to the stack boxing it first.
    /// Takes and returns `self` for builder style initialization.
    #[inline]
    pub fn push_box(mut self, child: impl Ui) -> Self {
        self.stack.push(StackEntry::new(child.into_box()));
        self
    }
}

impl<U: Ui> FromIterator<U> for Stack<U> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        Stack {
            stack: iter.into_iter().map(StackEntry::new).collect(),
        }
    }
}

impl<U: Ui> From<Vec<U>> for Stack<U> {
    #[inline]
    fn from(vec: Vec<U>) -> Self {
        vec.into_iter().collect()
    }
}

macro_rules! impl_tuples {
    ($TH:ident, $TH2:ident, $($T:ident, )* ) => {
        impl<$TH, $TH2, $($T, )*> From<($TH, $TH2, $($T,)*)> for Stack<Box<dyn Ui>>
        where $TH: Ui, $TH2: Ui, $($T: Ui, )*
        {
            #[inline]
            #[allow(non_snake_case)]
            fn from(($TH, $TH2, $($T,)*): ($TH, $TH2, $($T,)*)) -> Stack<Box<dyn Ui>> {
                let stack = vec![StackEntry::new($TH.into_box()), StackEntry::new($TH2.into_box()),  $(StackEntry::new($T.into_box()), )*];
                Stack { stack }
            }
        }

        impl_tuples!($( $T, )*);
    };

    () => {};
}
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);
