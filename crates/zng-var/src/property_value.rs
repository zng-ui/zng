use std::any::Any;

/// Represents metadata about a value type in a property args or value input.
///
/// Use the `#[impl_property_value]` attribute do generate an implementation.
#[diagnostic::on_unimplemented(
    message = "types must implement `PropertyValue` to support `_::item` syntax",
    note = "use `#[impl_property_value]` to derive implementation"
)]
pub trait PropertyValue: Any {
    /// Return type of [`assoc_items`].
    ///
    /// [`assoc_items`]: PropertyValue
    type AssocItems;

    /// Returns a proxy type that provides associated items from `Self` as methods.
    ///
    /// For example `Align::TOP` can be accessed as `value_sample.assoc_items().TOP()`. In property assigns
    /// the shorthand `_::TOP` or some other syntax expand !!: TODO (maybe take `self` as the unset value?)
    fn assoc_items(&self) -> Self::AssocItems;

    // this can be the mounting point for future editor features like getting a list of available values.
}

/// Implement [`PropertyValue`].
///
/// The attribute must be set in an `impl` block, it generates a proxy for every public associated constant or function that produces `Self`.
///
/// ```
/// # use zng_var::impl_property_value;
/// # struct ValueType;
/// #[impl_property_value]
/// impl ValueType {
///     /// Copy const value.
///     pub const ITEM: Self = ValueType;
///
///     /// Create func value
///     pub fn func() -> Self {
///         ValueType
///     }
///
///     /// Supports arguments and generics.
///     pub fn new(value: impl Into<bool>) -> Self {
///         VarType
///     }
/// }
/// ```
///
/// The attribute can also be set in an `enum` declaration, it generates a proxy for every unit and function like variants. Struct like
/// variants are ignored.
///
/// ```
/// # use zng_var::impl_property_value;
/// #[impl_property_value]
/// pub enum ValueType {
///     Variant1 = 0,
///     Variant2(bool),
///
///     NotSupported { arg: bool },
/// }
/// ```
///
/// The attribute can be set multiple times for the same type in the same module.
///
/// ```
/// # use zng_var::impl_property_value;
/// #[impl_property_value(Variants)]
/// pub enum ValueType {}
///
/// #[impl_property_value(Impl:Variants)]
/// impl ValueType {}
///
/// #[impl_property_value(:Impl)]
/// impl Default for ValueType {}
/// ```
///
/// The first call `impl_property_value(Variants)` generates only the proxy type, identifiable by `Variants`, this can be any ident.
///
/// The second call `impl_property_value(Impl:Variants)` also only generate a proxy type, this time identifiable by `Impl`. The generated proxy
/// type dereferences to `:Variants`, so the items of both will resolve.
///
/// The final call `impl_property_value(:Impl)` generates the final proxy type and the [`PropertyValue`] implementation. The generated proxy
/// type dereferences to `:Impl`, so all associated items across the three proxies will resolve.
pub use zng_var_proc_macros::impl_property_value;
