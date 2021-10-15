use zero_ui::prelude::*;

pub trait ViewGenerator<T>: 'static {
    type View: UiNode;
    
    /// Generates an [`UiNode`] from data.
    fn generate(&mut self, data: T) -> Self::View;

    fn boxed(self) -> BoxedViewGenerator<T> where Self: Sized {
        Box::new(self)
    }
}

pub type BoxedViewGenerator<T> = Box<dyn ViewGeneratorBoxed<T>>;

#[doc(hidden)]
pub trait ViewGeneratorBoxed<T> {
    fn generate_boxed(&mut self, data: T) -> zero_ui_core::BoxedUiNode;
}
impl<T, V: UiNode, U: Template<T, View=V>> ViewGeneratorBoxed<T> for U {
    fn generate_boxed(&mut self, data: T) -> zero_ui_core::BoxedUiNode {
        self.generate(data).boxed()
    }
}

fn main() {
    
}

pub type ViewGenerator<T> = Rc<Box<dyn Fn(T) -> BoxedUiNode>>;

mod data_stack {
    properties! {
        items_source: impl IntoVar<Vec<T>>;
        item_generator: impl ViewGenerator<T> + Clone + Debug;
    }
}