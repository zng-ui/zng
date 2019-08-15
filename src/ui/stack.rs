use super::{LayoutRect, LayoutSize, RenderContext, Ui};

pub struct StackChild {
    child: Box<dyn Ui>,
    rect: LayoutRect,
}

impl StackChild {
    pub fn new(child: impl Ui + 'static) -> Self {
        StackChild {
            child: child.into_box(),
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
    ($($T: ident),* | $($n:tt),*) => {
        impl<$($T: Ui + 'static, )*> IntoStackChildren for ($($T,)*) {
            fn into(self) -> Vec<StackChild> {
                vec![$(StackChild::new(self.$n), )*]
            }
        }
    };
}

// see!: https://stackoverflow.com/questions/55553281/is-it-possible-to-automatically-implement-a-trait-for-any-tuple-that-is-made-up
//
// C# codegen
//> r = "";
//. for (int i = 2; i <= 32; i++)
//. {
//.     r += "\nimpl_tuples!(";
//.     for (int t = 1; t <= i; t++)
//.     {
//.         r += $"T{t}, ";
//.     }
//.     r = r.TrimEnd(',', ' ');
//.     r += " | ";
//.     for (int n = 0; n < i; n++)
//.     {
//.         r += $"{n}, ";
//.     }
//.     r = r.TrimEnd(',', ' ');
//.     r += ");";
//. }
//. WriteLine(r)
impl_tuples!(T1, T2 | 0, 1);
impl_tuples!(T1, T2, T3 | 0, 1, 2);
impl_tuples!(T1, T2, T3, T4 | 0, 1, 2, 3);
impl_tuples!(T1, T2, T3, T4, T5 | 0, 1, 2, 3, 4);
impl_tuples!(T1, T2, T3, T4, T5, T6 | 0, 1, 2, 3, 4, 5);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7 | 0, 1, 2, 3, 4, 5, 6);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8 | 0, 1, 2, 3, 4, 5, 6, 7);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9 | 0, 1, 2, 3, 4, 5, 6, 7, 8);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31);



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
            fn render(&self, mut r: RenderContext) {
                for c in self.children.iter() {
                    r.push_child(&c.child, &c.rect);
                }
            }
        }
    };
}

stack!(HStack, width, height, x);
stack!(VStack, height, width, y);

pub fn h_list(children: impl IntoStackChildren) -> HStack {
    HStack::new(children)
}

pub fn v_list(children: impl IntoStackChildren) -> VStack {
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

    fn render(&self, mut r: RenderContext) {
        for c in self.children.iter() {
            r.push_child(&c.child, &c.rect);
        }
    }
}
pub fn z_list(children: impl IntoStackChildren) -> ZStack {
    ZStack::new(children)
}
