use crate::core::{
    render::FrameBuilder,
    types::{LayoutRect, LayoutSize},
    UiNode,
};
use crate::impl_ui_node;
use std::iter::FromIterator;

macro_rules! stack {
    ($Stack: ident, $stack_size: ident, $length_size: ident, $dimension: ident) => {
        struct $Stack<T> {
            children: Vec<StackEntry<T>>,
            //TODO - interspacing - space between entries
        }
        #[impl_ui_node(children)]
        impl<T: UiNode> UiNode for $Stack<T> {
            fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
                let mut total_size = LayoutSize::default();

                available_size.$stack_size = std::f32::INFINITY;
                for c in self.children.iter_mut() {
                    c.measure(available_size);
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
                    c.arrange(c.rect.size);
                }
            }

            fn render(&self, frame: &mut FrameBuilder) {
                for c in self.children.iter() {
                    c.render(frame)
                }
            }
        }
    };
}

stack!(HStack, width, height, x);
stack!(VStack, height, width, y);

/// Stack the children in a line (X). The first child at the beginning (0, 0) the last child
/// at the end (n, 0);
pub fn h_stack<T: UiNode>(children: Stack<T>) -> impl UiNode {
    HStack { children: children.stack }
}

/// Stacks the children in a column (Y). The first child at the top (0, 0) the last child at
/// the bottom (0, n).
pub fn v_stack<T: UiNode>(children: Stack<T>) -> impl UiNode {
    VStack { children: children.stack }
}

/// Stacks the children on top of each other. The first child at the bottom the last at the top.
struct ZStack<T> {
    children: Vec<StackEntry<T>>,
}

#[impl_ui_node(children)]
impl<T: UiNode> ZStack<T> {}

/// Stacks the children on top of each other (Z-index). The first child at the bottom the last at the top.
pub fn z_stack<T: UiNode>(children: Stack<T>) -> impl UiNode {
    ZStack { children: children.stack }
}

/// A child in a stack container.
struct StackEntry<T> {
    child: T,
    rect: LayoutRect,
}

impl<T: UiNode> StackEntry<T> {
    pub fn new(child: T) -> Self {
        StackEntry {
            child,
            rect: LayoutRect::default(),
        }
    }
}

#[impl_ui_node(child)]
impl<T: UiNode> UiNode for StackEntry<T> {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.rect.size = self.child.measure(available_size);
        self.rect.size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.rect.size = final_size;
        self.child.arrange(final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.rect.origin, |frame| self.child.render(frame));
    }
}

/// Stack children builder.
pub struct Stack<U: UiNode> {
    stack: Vec<StackEntry<U>>,
}

impl<U: UiNode> Default for Stack<U> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<U: UiNode> Stack<U> {
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
pub type BoxedStack = Stack<Box<dyn UiNode>>;

impl Stack<Box<dyn UiNode>> {
    /// Appends a `child` to the stack boxing it first.
    /// Takes and returns `self` for builder style initialization.
    #[inline]
    pub fn push_box(mut self, child: impl UiNode) -> Self {
        self.stack.push(StackEntry::new(child.boxed()));
        self
    }
}

impl<U: UiNode> FromIterator<U> for Stack<U> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        Stack {
            stack: iter.into_iter().map(StackEntry::new).collect(),
        }
    }
}

impl<U: UiNode> From<Vec<U>> for Stack<U> {
    #[inline]
    fn from(vec: Vec<U>) -> Self {
        vec.into_iter().collect()
    }
}

macro_rules! impl_tuples {
    ($TH:ident, $TH2:ident, $($T:ident, )* ) => {
        impl<$TH, $TH2, $($T, )*> From<($TH, $TH2, $($T,)*)> for Stack<Box<dyn UiNode>>
        where $TH: UiNode, $TH2: UiNode, $($T: UiNode, )*
        {
            #[inline]
            #[allow(non_snake_case)]
            fn from(($TH, $TH2, $($T,)*): ($TH, $TH2, $($T,)*)) -> Stack<Box<dyn UiNode>> {
                let stack = vec![StackEntry::new($TH.boxed()), StackEntry::new($TH2.boxed()),  $(StackEntry::new($T.boxed()), )*];
                Stack { stack }
            }
        }

        impl_tuples!($( $T, )*);
    };

    () => {};
}
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);
