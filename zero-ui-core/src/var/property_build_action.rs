use std::{sync::Arc, time::Duration};

use crate::{
    units::*,
    widget_builder::{AnyPropertyBuildAction, PropertyBuildAction},
};

use super::{BoxedVar, Var, VarValue};

#[doc(hidden)]
pub fn easing_property_build_action(
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
) -> Vec<Box<dyn AnyPropertyBuildAction>> {
    todo!()
}

/// Represents a *type visitor* that visits the input types of a property.
pub trait PropertyInputTypeVisitor {
    /// Input is a [`BoxedVar<T>`].
    fn visit_var<T: VarValue>(&mut self, index: usize) {
        self.visit_input::<BoxedVar<T>>(index)
    }

    /// Visit any input type, other visit methods calls this method in their default implementation.
    fn visit_input<I: Send + 'static>(&mut self, index: usize) {
        let _ = index;
    }
}

pub fn __visit_types__(visitor: &mut impl PropertyInputTypeVisitor) {
    visitor.visit_var::<bool>(0);
}

pub struct EasingPropertyBuildActionVisitor {
    duration: Duration,
    easing: Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync + 'static>,

    actions: Vec<Box<dyn AnyPropertyBuildAction>>,
    any_var: bool,
}

impl PropertyInputTypeVisitor for EasingPropertyBuildActionVisitor {
    fn visit_var<T: VarValue>(&mut self, _: usize) {
        let dur = self.duration;
        let easing = self.easing.clone();
        let action = PropertyBuildAction::<BoxedVar<T>>::new(move |var| {
            let easing = easing.clone();
            var.easing(dur, move |t| easing(t)).boxed()
        });
        self.actions.push(Box::new(action));
        self.any_var = true;
    }

    fn visit_input<I: Send + 'static>(&mut self, index: usize) {
        self.actions.push(Box::new(PropertyBuildAction::<I>::no_op()));
    }
}
