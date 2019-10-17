use super::{ElementState, LayoutPoint, ModifiersState, MouseButton};

#[derive(Debug, Clone, Copy)]
pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
}

#[derive(Debug, Clone, Copy)]
pub struct UiMouseMove {
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
}
