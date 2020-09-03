use super::{BoxVar, MapVar, MapVarBiDi};
use crate::core::context::{Updates, Vars};
use std::fmt::Debug;

/// A type that can be a [`Var`](crate::core::var::Var) value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait VarValue: Debug + Clone + 'static {}
impl<T: Debug + Clone + 'static> VarValue for T {}

/// A variable value that is set by the ancestors of an UiNode.
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: VarValue;

    /// Default value, used when the variable is not set in the context.
    fn default_value() -> &'static Self::Type;
}

/// Part of vars that is not public.
pub(crate) mod protected {
    use super::{VarValue, Vars};
    use std::any::TypeId;

    /// Info for context var binding.
    pub enum BindInfo<'a, T: VarValue> {
        /// Owned or SharedVar.
        ///
        /// * `&'a T` is a reference to the value borrowed in the context.
        /// * `bool` is the is_new flag.
        Var(&'a T, bool, u32),
        /// ContextVar.
        ///
        /// * `TypeId` of self.
        /// * `&'static T` is the ContextVar::default value of self.
        /// * `Option<(bool, u32)>` optional is_new and version override.
        ContextVar(TypeId, &'static T, Option<(bool, u32)>),
    }

    /// pub(crate) part of `ObjVar`.
    pub trait Var<T: VarValue>: 'static {
        fn bind_info<'a>(&'a self, vars: &'a Vars) -> BindInfo<'a, T>;

        fn is_context_var(&self) -> bool {
            false
        }

        fn read_only_prev_version(&self) -> u32 {
            0
        }
    }
}

/// Error when trying to set or modify a read-only variable.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct VarIsReadOnly;

impl std::fmt::Display for VarIsReadOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "cannot set or modify read-only variable")
    }
}

impl std::error::Error for VarIsReadOnly {}

/// Part of [`Var`] that can be boxed (object safe).
pub trait ObjVar<T: VarValue>: protected::Var<T> {
    /// The current value.
    ///
    /// If animating it is the animation final value, use [`get_step`](ObjVar::get_step) to get the current
    /// animation intermediary value.
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T;

    /// The intermediary animating value.
    ///
    /// If not animating it is the same as [`get`](ObjVar::get).
    fn get_step<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.get(vars)
    }

    /// [`get`](ObjVar::get) value if [`is_new`](ObjVar::is_new) otherwise returns `None`.
    ///
    /// If animating only one update happens immediately with the animation final value.
    /// Use [`update_step`](ObjVar::update_step) to get intermediary animation values updates.
    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;

    /// [`get_step`](ObjVar::get_step) value if [`is_animating`](ObjVar::is_animating) otherwise returns `None`.
    ///
    /// Animation updates happen in the [high-pressure channel](crate::core::UiNode::update_hp).
    fn update_step<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        let _ = vars;
        assert!(!self.is_animating(vars));
        None
    }

    /// If the value changed this update.
    fn is_new(&self, vars: &Vars) -> bool;

    /// Current value version. Version changes every time the value changes.
    fn version(&self, vars: &Vars) -> u32;

    /// If the value can be change.
    fn can_update(&self) -> bool;

    /// If the var is animating.
    fn is_animating(&self, vars: &Vars) -> bool {
        let _ = vars;
        false
    }

    /// If the variable cannot be set.
    ///
    /// Note the variable can still change from another endpoint, see [`can_update`](Self::can_update) to
    /// check if the variable is always a single value.
    fn read_only(&self, vars: &Vars) -> bool {
        let _ = vars;
        true
    }

    /// If [`read_only`](Self::read_only) is always `true`.
    fn always_read_only(&self, vars: &Vars) -> bool {
        let _ = vars;
        true
    }

    /// Schedules a variable change for the next update if the variable is not [`read_only`](ObjVar::read_only).
    fn push_set(&self, new_value: T, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let _ = new_value;
        let _ = vars;
        let _ = updates;
        assert!(self.read_only(vars));
        Err(VarIsReadOnly)
    }

    /// Schedules a variable modification for the next update using a boxed closure.
    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut T) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        let _ = modify;
        let _ = vars;
        let _ = updates;
        assert!(self.read_only(vars));
        Err(VarIsReadOnly)
    }

    /// Box the variable. This disables mapping.
    fn boxed(self) -> BoxVar<T>
    where
        Self: std::marker::Sized,
    {
        Box::new(self)
    }
}

/// A value that can change. Can [own the value](crate::core::var::OwnedVar) or be a [reference](crate::core::var::SharedVar).
///
/// This is the complete generic trait, the non-generic methods are defined in [ObjVar]
/// to support boxing.
///
/// Cannot be implemented outside of zero-ui crate. Use this together with [`IntoVar`] to
/// support dynamic values in property definitions.
pub trait Var<T: VarValue>: ObjVar<T> + Clone + IntoVar<T, Var = Self> {
    /// Return type of [`as_read_only`](Var::as_read_only).
    type AsReadOnly: Var<T>;
    /// Return type of [`as_local`](Var::as_local).
    type AsLocal: LocalVar<T>;

    /// Schedules a variable modification for the next update.
    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let _ = modify;
        let _ = vars;
        let _ = updates;
        assert!(self.read_only(vars));
        Err(VarIsReadOnly)
    }

    /// Returns a read-only `Var<O>` that uses a closure to generate its value from this `Var<T>` every time it changes.
    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        Self: Sized,
        M: FnMut(&T) -> O + 'static,
        O: VarValue;

    /// Returns a read-only `Var<O>` that uses a closure to generate its value from this `Var<T>` every time it changes.
    fn into_map<O, M>(self, map: M) -> MapVar<T, Self, O, M>
    where
        Self: Sized,
        M: FnMut(&T) -> O + 'static,
        O: VarValue;

    /// Bidirectional map. Returns a `Var<O>` that uses two closures to convert to and from this `Var<T>`.
    ///
    /// Unlike [`map`](Var::map) the returned variable is read-write when this variable is read-write.
    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        Self: Sized,
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static;

    /// Ensures this variable is [`always_read_only`](ObjVar::always_read_only).
    fn as_read_only(self) -> Self::AsReadOnly;

    /// Returns a [variable](LocalVar) that keeps the current value locally so
    /// it can be read without a [context](Vars).
    fn as_local(self) -> Self::AsLocal;
}

/// A value-to-[var](Var) conversion that consumes the value.
pub trait IntoVar<T: VarValue>: Clone {
    type Var: Var<T> + 'static;

    /// Converts the source value into a var.
    fn into_var(self) -> Self::Var;

    /// Shortcut call `self.into_var().as_local()`.
    #[inline]
    fn into_local(self) -> <<Self as IntoVar<T>>::Var as Var<T>>::AsLocal
    where
        Self: Sized,
    {
        self.into_var().as_local()
    }
}

/// A variable that can be read without [context](Vars).
pub trait LocalVar<T: VarValue>: ObjVar<T> {
    /// Gets the local copy of the value.
    fn get_local(&self) -> &T;

    /// Gets the local copy of the animation intermediary value if is animating
    /// or [`get_local`](LocalVar::get_local) if not.
    fn get_local_step(&self) -> &T {
        self.get_local()
    }

    /// Initializes the local copy of the value and local animating value if is animating.
    /// Must be called on [`init`](crate::core::UiNode::init).
    fn init_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> &'a T;

    /// Update the local copy of the value. Must be called every [`update`](crate::core::UiNode::update).
    fn update_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T>;

    /// Update the local copy of the animation intermediary value. Must be called every
    /// [`update_hp`](crate::core::UiNode::update_hp) to support animation.
    fn update_local_step<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T>;
}
