//! Localization service [`L10N`] and helpers.
//!

/// Localization service.
pub struct L10N;
impl L10N {}

///<span data-del-macro-root></span> Gets a variable that localizes and formats the text in a widget context.
///
/// # Syntax
///
/// Macro expects a resource ID string literal a *template* string literal that is also used
/// as fallback, followed by optional named format arguments `arg = <arg>,..`.
///
/// The *template* string syntax is the [Fluent Project] syntax, interpolations in the form of `"{$var}"` are resolved to a local `$var`.
///
/// ```
/// # use zero_ui_core::l10n::*;
/// # macro_rules! demo
/// ```
///
/// # Scrapper
///
/// The `zero-ui-l10n-scrapper` tool can be used to collect all localizable text of Rust code files, it is a text based search that
/// matches this macro name and the two first input literals, avoid renaming this macro to support scrapping, otherwise you will
/// have to declare the template file manually.
///
/// The scrapper also has some support for comments, if the previous code line from a [`l10n!`] call is a comment starting with
/// prefix `l10n: #comment` the `#comment` is collected, same for a suffix comment in the same line of the [`l10n!`] call.
///
/// [Fluent Project]: https://projectfluent.org/fluent/guide/
#[macro_export]
macro_rules! l10n {
    ($resource_id:tt, $template:tt $(,)?) => {
        $crate::l10n_impl! {
            resource_id { $resource_id }
            template { $template }
        }
    };
    ($resource_id:tt, $template:tt, $($arg:ident = $arg_expr:expr),* $(,)?) => {
        {
            $(
                let $arg = $arg_expr;
            )*
            $crate::l10n_impl! {
                resource_id { $resource_id }
                template { $template }
            }
        }
    };
    ($($error:tt)*) => {
        std::compile_error!(r#"expected ("resource-id") or ("id", "template") or ("id", "t", arg=expr)"#)
    }
}
#[doc(inline)]
pub use l10n;
