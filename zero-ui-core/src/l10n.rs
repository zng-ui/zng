//! Localization service [`L10N`] and helpers.
//!

/// Localization service.
pub struct L10N;
impl L10N {}

///<span data-del-macro-root></span> Gets a variable that localizes and formats the text in a widget context.
///
/// # Syntax
///
/// Macro expects a message ID string literal a *message template* string literal that is also used
/// as fallback, followed by optional named format arguments `arg = <arg>,..`.
///
/// The message string syntax is the [Fluent Project] syntax, interpolations in the form of `"{$var}"` are resolved to a local `$var`.
///
/// ```
/// # use zero_ui_core::{l10n::*, var::*};
/// let name = var("World");
/// let msg = l10n!("msg-id", "Hello {$name}!");
/// ```
///
/// # Scrapper
///
/// The `zero-ui-l10n-scrapper` tool can be used to collect all localizable text of Rust code files, it is a text based search that
/// matches this macro name and the two first input literals, avoid renaming this macro to support scrapping, otherwise you will
/// have to declare the message file manually.
///
/// The scrapper also has some support for comments, if the previous code line from a [`l10n!`] call is a comment starting with
/// prefix `l10n: #comment` the `#comment` is collected, same for a suffix comment in the same line of the [`l10n!`] call.
///
/// [Fluent Project]: https://projectfluent.org/fluent/guide/
#[macro_export]
macro_rules! l10n {
    ($message_id:tt, $message:tt $(,)?) => {
        $crate::l10n::__l10n! {
            l10n_path { $crate::l10n }
            message_id { $message_id }
            message { $message }
        }
    };
    ($message_id:tt, $message:tt, $($arg:ident = $arg_expr:expr),* $(,)?) => {
        {
            $(
                let $arg = $arg_expr;
            )*
            $crate::l10n::__l10n! {
                l10n_path { $crate::l10n }
                message_id { $message_id }
                message { $message }
            }
        }
    };
    ($($error:tt)*) => {
        std::compile_error!(r#"expected ("id", "message") or ("id", "msg {$arg}", arg=expr)"#)
    }
}
use fluent::types::FluentNumber;
#[doc(inline)]
pub use l10n;

#[doc(hidden)]
pub use zero_ui_proc_macros::l10n as __l10n;

use crate::text::{Lang, Txt, LANG_VAR};
use crate::var::{self, *};

impl L10N {
    /// Gets a variable that is a localized message identified by `id` in the localization context
    /// where the variable is first used. The variable will update when the contextual language changes.
    ///
    /// If the message has variable arguments they must be provided using [`L10nMessageBuilder::arg`], the
    /// returned variable will also update when the arg variables update.
    ///
    /// The `id` can be compound with an attribute `"msg-id.attribute"`, the `fallback` is used
    /// when the message is not found in the localization context.
    ///
    /// Prefer using the [`l10n!`] macro instead of this method, the macro does compile time validation.
    pub fn message(&self, id: Txt, fallback: Txt) -> L10nMessageBuilder {
        L10nMessageBuilder {
            id,
            fallback,
            args: vec![],
        }
    }

    /// Function called by `l10n!`.
    #[doc(hidden)]
    pub fn l10n_message(&self, id: &'static str, fallback: &'static str) -> L10nMessageBuilder {
        self.message(Txt::from_static(id), Txt::from_static(fallback))
    }
}

/// Represents lazy loaded localization data retrieved from an specific data source.
#[derive(Clone, Debug)]
pub struct L10nResource {}
impl L10nResource {
    /// Empty resource, never finds any message.
    pub fn empty() -> Self {
        Self {}
    }

    /// Search for the message in the resources.
    pub fn raw_message(&self, lang: &Lang, id: &str) -> Option<Txt> {
        // !!: TODO
        let _ = (lang, id);
        None
    }
}

context_var! {
    /// Represents the contextual [`L10nResource`], together with the [`LANG_VAR`]
    /// a localized message can be retrieved.
    static L10N_RESOURCE_VAR: L10nResource = L10nResource::empty();
}

/// Localized message variable builder.
///
/// See [`L10N.message`] for more details.
pub struct L10nMessageBuilder {
    id: Txt,
    fallback: Txt,
    args: Vec<(Txt, BoxedVar<L10nArgument>)>,
}
impl L10nMessageBuilder {
    /// Add a format arg variable.
    pub fn arg(mut self, name: Txt, value: impl IntoVar<L10nArgument>) -> Self {
        self.args.push((name, value.into_var().boxed()));
        self
    }
    #[doc(hidden)]
    pub fn l10n_arg(self, name: &'static str, value: impl Var<L10nArgument>) -> Self {
        self.arg(Txt::from_static(name), value)
    }

    /// Build the variable.
    pub fn build(self) -> impl Var<Txt> {
        let Self { id, fallback, args } = self;
        merge_var!(L10N_RESOURCE_VAR, LANG_VAR, move |res, lang| {
            // !!: TODO
            let _ = args;
            match res.raw_message(lang, &id) {
                Some(f) => f,
                None => fallback.clone(),
            }
        })
    }
}

/// Represents an argument value for a localization message.
///
/// See [`L10nMessageBuilder::arg`] for more details.
#[derive(Clone, Debug)]
pub enum L10nArgument {
    /// String.
    Txt(Txt),
    /// Number, with optional style details.
    Number(FluentNumber),
} // !!: TODO, see https://docs.rs/fluent/0.16.0/fluent/enum.FluentValue.html

impl_from_and_into_var! {
    fn from(txt: Txt) -> L10nArgument {
        L10nArgument::Txt(txt)
    }
    fn from(txt: &'static str) -> L10nArgument {
        L10nArgument::Txt(Txt::from_static(txt))
    }
    fn from(txt: String) -> L10nArgument {
        L10nArgument::Txt(Txt::from(txt))
    }
    fn from(t: char) -> L10nArgument {
        L10nArgument::Txt(Txt::from_char(t))
    }
    fn from(number: FluentNumber) -> L10nArgument {
        L10nArgument::Number(number)
    }
}

#[doc(hidden)]
pub struct L10nSpecialize<T>(pub T);
#[doc(hidden)]
pub trait IntoL10nVar {
    type Var: Var<L10nArgument>;
    fn into_l10n_var(self) -> Self::Var;
}

impl<T: Into<L10nArgument>> IntoL10nVar for L10nSpecialize<T> {
    type Var = var::LocalVar<L10nArgument>;

    fn into_l10n_var(self) -> Self::Var {
        var::LocalVar(self.0.into())
    }
}
impl<T: VarValue + Into<L10nArgument>> IntoL10nVar for &L10nSpecialize<ArcVar<T>> {
    type Var = var::types::ContextualizedVar<L10nArgument, var::ReadOnlyArcVar<L10nArgument>>;

    fn into_l10n_var(self) -> Self::Var {
        self.0.map_into()
    }
}
impl<V: Var<L10nArgument>> IntoL10nVar for &&L10nSpecialize<V> {
    type Var = V;

    fn into_l10n_var(self) -> Self::Var {
        self.0.clone()
    }
}
