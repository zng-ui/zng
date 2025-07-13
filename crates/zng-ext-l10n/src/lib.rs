#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Localization service, [`l10n!`] and helpers.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use semver::Version;
use zng_app::{
    AppExtension,
    event::{Command, CommandMetaVar, EVENTS_L10N},
    update::EventUpdate,
    view_process::{VIEW_PROCESS_INITED_EVENT, raw_events::RAW_LOCALE_CONFIG_CHANGED_EVENT},
};
use zng_layout::context::LayoutDirection;
use zng_task as task;

use zng_txt::Txt;
use zng_var::{ArcEq, ArcVar, BoxedVar, ReadOnlyArcVar, Var, types::ArcCowVar};

#[doc(hidden)]
pub use zng_ext_l10n_proc_macros::lang as __lang;

#[doc(hidden)]
pub use zng_ext_l10n_proc_macros::l10n as __l10n;

#[doc(hidden)]
pub use unic_langid;

mod types;
pub use types::*;

mod service;
use service::L10N_SV;

mod sources;
pub use sources::*;

/// Localization service.
pub struct L10N;

/// Application extension that provides localization.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`L10N`]
#[derive(Default)]
#[non_exhaustive]
pub struct L10nManager {}
impl AppExtension for L10nManager {
    fn init(&mut self) {
        EVENTS_L10N.init_l10n(|file, cmd, attr, txt| {
            L10N.bind_command_meta(
                LangFilePath {
                    pkg_name: file[0].into(),
                    pkg_version: file[1].parse().unwrap_or_else(|e| {
                        tracing::error!("invalid package version from command localization, {e}");
                        Version::new(0, 0, 0)
                    }),
                    file: file[2].into(),
                },
                cmd,
                attr,
                txt,
            );
        });
    }

    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(u) = RAW_LOCALE_CONFIG_CHANGED_EVENT
            .on(update)
            .map(|args| &args.config)
            .or_else(|| VIEW_PROCESS_INITED_EVENT.on(update).map(|args| &args.locale_config))
        {
            L10N_SV.read().set_sys_langs(u);
        }
    }
}

///<span data-del-macro-root></span> Gets a variable that localizes and formats the text in a widget context.
///
/// # Syntax
///
/// Macro expects a message key string literal a *message template* string literal that is also used
/// as fallback, followed by optional named format arguments `arg = <arg>,..`.
///
/// The message string syntax is the [Fluent Project] syntax, interpolations in the form of `"{$var}"` are resolved to a local `$var`.
///
/// ```
/// # use zng_ext_l10n::*;
/// # use zng_var::*;
/// # let _scope = zng_app::APP.minimal();
/// let name = var("World");
/// let msg = l10n!("file/id.attribute", "Hello {$name}!");
/// ```
///
/// ## Key
///
/// This message key can be just a Fluent identifier, `"id"`, a Fluent attribute identifier can be added `"id.attr"`, and finally
/// a file name can be added `"file/id"`. The key syntax is validated at compile time.
///
/// ### Id
///
/// The only required part of a key is the ID, it must contain at least one character, it must start with an ASCII letter
/// and can be followed by any ASCII alphanumeric, _ and -, `[a-zA-Z][a-zA-Z0-9_-]*`.
///
/// ### Attribute
///
/// An attribute identifier can be suffixed on the id, separated by a `.` followed by an identifier of the same pattern as the
/// id, `.[a-zA-Z][a-zA-Z0-9_-]*`.
///
/// ### File
///
/// An optional file name can be prefixed on the id, separated by a `/`, it can be a single file name, no extension.
///
/// Using the default directory resolver the key `"file/id.attr"` will search the id and attribute in the file `{dir}/{lang}/file.ftl`:
///
/// ```ftl
/// id =
///     .attr = message
/// ```
///
/// And a key `"id.attr"` will be searched in the file `{dir}/{lang}/_.ftl`.
///
/// ### Package
///
/// The crate package name and version are also implicitly collected, when the message is requested from a different crate
/// it is searched in `{dir}/{lang}/{pkg-name}/{pkg-version}/{file}.ftl`. Version matches any other version, the nearest is selected.
///
/// # Scrap Template
///
/// The `cargo zng l10n` tool can be used to collect all localizable text of Rust code files, it is a text based search that
/// matches this macro name and the two first input literals, avoid renaming this macro to support scrapping, otherwise you will
/// have to declare the template file manually.
///
/// The scrapper can also scrap comments, if the previous code line from a [`l10n!`] call is a comment starting with
/// prefix `l10n-# ` the text the follows is collected, same for a comment in the same line of the [`l10n!`] call. Sections
/// can be declared using `l10n-## `, all entries after a section comment are added to that section.
///
/// Standalone notes can be added to the top of the template file from anywhere using `l10n-{file_pattern}-### `, file pattern can be omitted,
/// `l10n-### ` is equivalent to `l10n--### ` that matches the localization template used when no file is specified. Note that only standalone
/// notes can have file names, sections and comments (`##` and `#`) are copied to each file of keys associated with the comment or section.
///
/// ```
/// # use zng_ext_l10n::*;
/// # use zng_var::*;
/// # let _scope = zng_app::APP.minimal();
/// #
/// // l10n-### Standalone Note
///
/// // l10n-# Comment for `id`.
/// let msg = l10n!("id", "id message");
///
/// // l10n-# Comment for `id.attr`.
/// let msg = l10n!("id.attr", "attr message");
///
/// // l10n-## Section
///
/// let msg = l10n!("other", "other message"); // l10n-# Comment for `other`.
/// ```
///
/// The example above is scrapped to a `template.ftl` file:
///
/// ```ftl
/// ### Standalone Note
///
/// # Comment for `id`.
/// #
/// # attr:
/// #     Comment for `id.attr`.
/// id = id message
///     .attr = attr message
///
/// ## Section
///
/// # Commend for `other`.
/// other = other message
/// ```
///
/// You can install the scraper tool using cargo:
///
/// ```console
/// cargo install cargo-zng
/// ```
///
/// [Fluent Project]: https://projectfluent.org/fluent/guide/
#[macro_export]
macro_rules! l10n {
    ($message_id:tt, $message:tt $(,)?) => {
        $crate::__l10n! {
            l10n_path { $crate }
            message_id { $message_id }
            message { $message }
        }
    };
    ($message_id:tt, $message:tt, $($arg:ident = $arg_expr:expr),* $(,)?) => {
        {
            $(
                let $arg = $arg_expr;
            )*
            $crate::__l10n! {
                l10n_path { $crate }
                message_id { $message_id }
                message { $message }
            }
        }
    };
    ($($error:tt)*) => {
        std::compile_error!(r#"expected ("id", "message") or ("id", "msg {$arg}", arg=expr)"#)
    }
}

impl L10N {
    /// Change the localization resources to `source`.
    ///
    /// All active variables and handles will be updated to use the new source.
    pub fn load(&self, source: impl L10nSource) {
        L10N_SV.write().load(source);
    }

    /// Start watching the `dir` for `dir/{lang}/*.ftl` and `dir/{lang}/deps/*/*/*.ftl` files.
    ///
    /// The [`available_langs`] variable maintains an up-to-date list of locale files found, the files
    /// are only loaded when needed, and also are watched to update automatically.
    ///
    /// [`available_langs`]: Self::available_langs
    pub fn load_dir(&self, dir: impl Into<PathBuf>) {
        self.load(L10nDir::open(dir))
    }

    /// Load localization resources from a `.tar` or `.tar.gz` container.
    ///
    /// The expected container layout is `root_dir/{lang}/{file}.ftl` app files and `root_dir/{lang}/deps/{pkg-name}/{pkg-version}/{file}.ftl`
    /// for dependencies, same as [`load_dir`], `root_dir` can have any name.
    ///
    /// The data can be embedded using [`include_bytes!`] or loaded into a `Vec<u8>` and must be in the `.tar` or `.tar.gz` format.
    ///
    /// [`load_dir`]: L10N::load_dir
    #[cfg(feature = "tar")]
    pub fn load_tar(&self, data: impl Into<L10nTarData>) {
        self.load(L10nTar::load(data))
    }

    /// Available localization files.
    ///
    /// The value maps lang to one or more files, the files can be from the project `dir/{lang}/{file}.ftl` or from dependencies
    /// `dir/{lang}/deps/{pkg-name/{pkg-version}/{file}.ftl`.
    ///
    /// Note that this map will include any file in the source dir that has a name that is a valid [`lang!`],
    /// that includes the `template.ftl` file and test pseudo-locales such as `qps-ploc.ftl`.
    pub fn available_langs(&self) -> BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>> {
        L10N_SV.write().available_langs()
    }

    /// Status of the [`available_langs`] list.
    ///
    /// This will be `NotAvailable` before the first call to [`load_dir`], then it changes to `Loading`, then
    /// `Loaded` or `Error`.
    ///
    /// Note that this is the status of the resource list, not of each individual resource, you
    /// can use [`LangResource::status`] for that.
    ///
    /// [`available_langs`]: Self::available_langs
    /// [`load_dir`]: Self::load_dir
    pub fn available_langs_status(&self) -> BoxedVar<LangResourceStatus> {
        L10N_SV.write().available_langs_status()
    }

    /// Waits until [`available_langs_status`] is not `Loading`.
    ///
    /// [`available_langs_status`]: Self::available_langs_status
    pub async fn wait_available_langs(&self) {
        // wait potential `load_dir` start.
        task::yield_now().await;

        let status = self.available_langs_status();
        while matches!(status.get(), LangResourceStatus::Loading) {
            status.wait_update().await;
        }
    }

    /// Gets a read-write variable that sets the preferred languages for the app.
    /// Lang not available are ignored until they become available, the first language in the
    /// vec is the most preferred.
    ///
    /// The value is the same as [`sys_lang`], if set the variable disconnects from system lang.
    ///
    /// Note that the [`LANG_VAR`] is used in message requests, the default value of that
    /// context variable is this one.
    ///
    /// [`sys_lang`]: Self::sys_lang
    pub fn app_lang(&self) -> ArcCowVar<Langs, ArcVar<Langs>> {
        L10N_SV.read().app_lang()
    }

    /// Gets a read-only variable that is the current system language.
    ///
    /// The variable will update when the view-process notifies that the config has changed. Is
    /// empty if the system locale cannot be retrieved.
    pub fn sys_lang(&self) -> ReadOnlyArcVar<Langs> {
        L10N_SV.read().sys_lang()
    }

    /// Gets a read-only variable that is a localized message in the localization context
    /// where the variable is first used. The variable will update when the contextual language changes.
    ///
    /// If the message has variable arguments they must be provided using [`L10nMessageBuilder::arg`], the
    /// returned variable will also update when the arg variables update.
    ///
    /// Prefer using the [`l10n!`] macro instead of this method, the macro does compile time validation.
    ///
    /// # Params
    ///
    /// * `file`: Name of the resource file, in the default directory layout the file is searched at `dir/{lang}/{file}.ftl`, if
    ///   empty the file is searched at `dir/{lang}/_.ftl`. Only a single file name is valid, no other path components allowed.
    ///   Note that the file can also be a full [`LangFilePath`] that includes dependency package info. Those files are searched in
    ///   `dir/{lang}/deps/{pkg-name}/{pkg-version}/{file}.ftl`.
    /// * `id`: Message identifier inside the resource file.
    /// * `attribute`: Attribute of the identifier, leave empty to not use an attribute.
    /// * `fallback`: Message to use when a localized message cannot be found.
    ///
    /// The `id` and `attribute` is only valid if it starts with letter `[a-zA-Z]`, followed by any letters, digits, _ or - `[a-zA-Z0-9_-]*`.
    ///
    /// Panics if any parameter is invalid.
    pub fn message(
        &self,
        file: impl Into<LangFilePath>,
        id: impl Into<Txt>,
        attribute: impl Into<Txt>,
        fallback: impl Into<Txt>,
    ) -> L10nMessageBuilder {
        L10nMessageBuilder {
            file: file.into(),
            id: id.into(),
            attribute: attribute.into(),
            fallback: fallback.into(),
            args: vec![],
        }
    }

    /// Function called by `l10n!`.
    #[doc(hidden)]
    pub fn l10n_message(
        &self,
        pkg_name: &'static str,
        pkg_version: &'static str,
        file: &'static str,
        id: &'static str,
        attribute: &'static str,
        fallback: &'static str,
    ) -> L10nMessageBuilder {
        self.message(
            LangFilePath {
                pkg_name: Txt::from_static(pkg_name),
                pkg_version: pkg_version.parse().unwrap(),
                file: Txt::from_static(file),
            },
            Txt::from_static(id),
            Txt::from_static(attribute),
            Txt::from_static(fallback),
        )
    }

    /// Gets a handle to the lang file resource.
    ///
    /// The resource will be loaded and stay in memory until all clones of the handle are dropped, this
    /// can be used to pre-load resources so that localized messages find it immediately avoiding flashing
    /// the fallback text in the UI.
    ///
    /// If the resource directory or file changes it is auto-reloaded, just like when a message variable
    /// held on the resource does.
    ///
    /// # Params
    ///
    /// * `lang`: Language identifier.
    /// * `file`: Name of the resource file, in the default directory layout the file is searched at `dir/{lang}/{file}.ftl`, if
    ///   empty the file is searched at `dir/{lang}/_.ftl`. Only a single file name is valid, no other path components allowed.
    ///   Note that the file can also be a full [`LangFilePath`] that includes dependency package info. Those files are searched in
    ///   `dir/{lang}/deps/{pkg-name}/{pkg-version}/{file}.ftl`.
    ///
    /// Panics if the file is invalid.
    pub fn lang_resource(&self, lang: impl Into<Lang>, file: impl Into<LangFilePath>) -> LangResource {
        L10N_SV.write().lang_resource(lang.into(), file.into())
    }

    /// Gets a handle to all resource files for the `lang` after they load.
    ///
    /// This awaits for the available langs to load, then collect an awaits for all lang files.
    pub async fn wait_lang(&self, lang: impl Into<Lang>) -> LangResources {
        let lang = lang.into();
        let mut r = vec![];
        for (file, _) in self.available_langs().get().get(&lang).into_iter().flatten() {
            r.push(self.lang_resource(lang.clone(), file.clone()));
        }
        for h in &r {
            h.wait().await;
        }
        LangResources(r)
    }

    /// Gets a handle to all resource files of the first lang in `langs` that is available and loaded.
    ///
    /// This awaits for the available langs to load, then collect an awaits for all lang files.
    pub async fn wait_first(&self, langs: impl Into<Langs>) -> (Option<Lang>, LangResources) {
        let langs = langs.into();

        L10N.wait_available_langs().await;

        let available = L10N.available_langs().get();
        for lang in langs.0 {
            if let Some(files) = available.get_exact(&lang) {
                let mut r = Vec::with_capacity(files.len());
                for file in files.keys() {
                    r.push(self.lang_resource(lang.clone(), file.clone()));
                }
                let handle = LangResources(r);
                handle.wait().await;

                return (Some(lang), handle);
            }
        }

        (None, LangResources(vec![]))
    }

    /// Bind the command metadata to a message.
    ///
    /// This is automatically called by [`command!`] instances that set the metadata `l10n!: true` or `l10n!: "file"`.
    ///
    /// [`command!`]: zng_app::event::command!
    pub fn bind_command_meta(
        &self,
        file: impl Into<LangFilePath>,
        cmd: Command,
        meta_name: impl Into<Txt>,
        meta_value: CommandMetaVar<Txt>,
    ) {
        let msg = self.message(file, cmd.event().as_any().name(), meta_name, meta_value.get()).build();
        meta_value.set_from(&msg).unwrap();

        // bind only holds a weak ref to `meta_value`` in `msg`
        msg.bind(&meta_value).perm();
        meta_value
            .hook(move |_| {
                // keep `msg` alive to it continues updating `meta_value`
                let _keep = &msg;
                true
            })
            .perm();
    }
}

/// <span data-del-macro-root></span> Compile-time validated [`Lang`] value.
///
/// The language is parsed during compile and any errors are emitted as compile time errors.
///
/// # Syntax
///
/// The input can be a single a single string literal with `-` separators, or a single ident with `_` as the separators.
///
/// # Examples
///
/// ```
/// # use zng_ext_l10n::lang;
/// let en_us = lang!(en_US);
/// let en = lang!(en);
///
/// assert!(en.matches(&en_us, true, false));
/// assert_eq!(en_us, lang!("en-US"));
/// ```
#[macro_export]
macro_rules! lang {
    ($($tt:tt)+) => {
        {
            let lang: $crate::unic_langid::LanguageIdentifier = $crate::__lang! {
                unic_langid { $crate::unic_langid }
                lang { $($tt)+ }
            };
            $crate::Lang(lang)
        }
    }
}

/// Represents a localization data source.
///
/// See [`L10N.load`] for more details.
///
/// [`L10N.load`]: L10N::load
pub trait L10nSource: Send + 'static {
    /// Gets a read-only variable with all lang files that the source can provide.
    fn available_langs(&mut self) -> BoxedVar<Arc<LangMap<HashMap<LangFilePath, PathBuf>>>>;
    /// Gets a read-only variable that is the status of the [`available_langs`] value.
    ///
    /// [`available_langs`]: Self::available_langs
    fn available_langs_status(&mut self) -> BoxedVar<LangResourceStatus>;

    /// Gets a read-only variable that provides the fluent resource for the `lang` and `file` if available.
    fn lang_resource(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<Option<ArcEq<fluent::FluentResource>>>;
    /// Gets a read-only variable that is the status of the [`lang_resource`] value.
    ///
    /// [`lang_resource`]: Self::lang_resource
    fn lang_resource_status(&mut self, lang: Lang, file: LangFilePath) -> BoxedVar<LangResourceStatus>;
}

fn from_unic_char_direction(d: unic_langid::CharacterDirection) -> LayoutDirection {
    match d {
        unic_langid::CharacterDirection::LTR => LayoutDirection::LTR,
        unic_langid::CharacterDirection::RTL => LayoutDirection::RTL,
        d => {
            tracing::warn!("converted {d:?} to LTR");
            LayoutDirection::LTR
        }
    }
}
