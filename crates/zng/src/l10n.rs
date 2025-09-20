//! Localization service, sources and other types.
//!
//! Localized text is declared using the [`l10n!`] macro, it provides a read-only text variable that automatically
//! updates to be best localized text available given the current loaded localization and the app language.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! let click_count = var(0u32);
//! # let _ =
//! Window! {
//!     title = l10n!("window-title", "Window Title");
//!     child = Button! {
//!         on_click = hn!(click_count, |_| click_count.set(click_count.get() + 1));
//!         child = Text!(l10n!("click-count", "Clicked {$n} times", n = click_count.clone()));
//!     };
//! }
//! # ; }
//! ```
//!
//! In the example above declares two localization messages, "window.title" and "btn.click_count", if
//! these messages are localized for the current language the localized text is used, otherwise the provided
//! fallback is used.
//!
//! The [`L10N`] service can be used to set the app language and load localization resources. The example below
//! sets the language to en-US and loads localization from a directory using [`L10N.load_dir`].
//!
//! ```no_run
//! use zng::prelude::*;
//!
//! APP.defaults().run_window(async {
//!     // start loading localization resources
//!     L10N.load_dir(zng::env::res("l10n"));
//!     // set the app language, by default is the system language
//!     L10N.app_lang().set(lang!("en-US"));
//!     // preload the localization resources for a language
//!     L10N.wait_first(lang!("en-US")).await;
//!
//!     Window! {
//!         // ..
//!     }
//! });
//! ```
//!
//! The service also supports embedded localization resources in the `.tar` and `.tar.gz` formats using
//! [`L10N.load_tar`], see the [localize example] for more details. You can also implement more container formats using [`L10N.load`].
//!
//! [`L10N.load_dir`]: crate::l10n::L10N::load_dir
//! [`L10N.load_tar`]: crate::l10n::L10N::load_tar
//! [`L10N.load`]: crate::l10n::L10N::load
//! [localize example]: https://github.com/zng-ui/zng/blob/main/examples/localize/build.rs
//!
//! # Fluent
//!
//! The localization files are in the [Fluent](https://projectfluent.org/) format. Fluent empowers translators to
//! script things like plural forms, for this reason a localization file should be provided even for the same
//! language the `l10n!` fallback text is written in.
//!
//! ```ftl
//! click-count = {$n ->
//!     [one] Clicked {$n} time
//!     *[other] Clicked {$n} times
//! }
//! ```
//!
//! The example above demonstrates a localized message that provides plural alternatives for the English language.
//!
//! # Scraper
//!
//! The `cargo zng l10n` tool can be used to generate a Fluent file from source code, the Fluent file can be
//! used as a template for translators, it will include the fallback text and comments written close the key
//! declaration.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! // l10n-### This standalone comment is added to the scraped template file.
//!
//! let click_count = var(0u32);
//! # let _ =
//! Window! {
//!     title = l10n!("window-title", "Window Title");
//!     child = Button! {
//!         on_click = hn!(click_count, |_| click_count.set(click_count.get() + 1));
//!         // l10n-# This comment is added to the `"click-count"` entry.
//!         child = Text!(l10n!("click-count", "Clicked {$n} times", n = click_count.clone()));
//!     };
//! }
//! # ; }
//! ```
//!
//! When the example above is scrapped it generates:
//!
//! ```ftl
//! ### This standalone comment is added to all scraped template files.
//!
//! # This comment is added to the `"click-count"` entry.
//! click-count = Clicked {$n} times
//! ```
//!
//! See the [`l10n!`] documentation for a full explanation of how the Scraper converts comments and the
//! `l10n!` calls into Fluent files.
//!
//! [`l10n!`]: crate::l10n::l10n
//!
//! # Commands
//!
//! Commands metadata can be localized and scrapped, to enable this set `l10n!:` on the [`command!`](zng::event::command) declarations.
//!
//! If the first metadata is `l10n!:` the command init will attempt to localize the other string metadata. The `cargo zng l10n`
//! command line tool scraps commands that set this special metadata.
//!
//! ```
//! # use zng_app::{event::{command, CommandNameExt, CommandInfoExt}, shortcut::{CommandShortcutExt, shortcut}};
//! command! {
//!     pub static FOO_CMD = {
//!         l10n!: true,
//!         name: "Foo!",
//!         info: "Does the foo thing",
//!     };
//! }
//! ```
//!
//! The example above will be scrapped as:
//!
//! ```ftl
//! FOO_CMD =
//!     .name = Foo!
//!     .info = Does the foo thing.
//! ```
//!
//! The `l10n!:` meta can also be set to a localization file name:
//!
//! ```
//! # use zng_app::{event::{command, CommandNameExt, CommandInfoExt}, shortcut::{CommandShortcutExt, shortcut}};
//! command! {
//!     pub static FOO_CMD = {
//!         l10n!: "file",
//!         name: "Foo!",
//!     };
//! }
//! ```
//!
//! The example above is scrapped to `{l10n-dir}/{lang}/file.ftl` files.
//!
//! ## Limitations
//!
//! Interpolation is not supported in command localization strings.
//!
//! The `l10n!:` value must be a *textual* literal, that is, it can be only a string literal or a `bool` literal, and it cannot be
//! inside a macro expansion.
//! # Full API
//!
//! See [`zng_ext_l10n`] for the full localization API.

pub use zng_ext_l10n::{
    IntoL10nVar, L10N, L10nArgument, L10nDir, L10nMessageBuilder, L10nSource, L10nTar, LANG_VAR, Lang, LangFilePath, LangMap, LangResource,
    LangResourceStatus, LangResources, Langs, NilL10nSource, SwapL10nSource, l10n, lang,
};
