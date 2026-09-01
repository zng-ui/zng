#![cfg(feature = "setup")]

//! Widgets for setup UI and a service for implementing it.
//!
//! The [`SETUP`] service manages running setup operations. Operations are
//! composed of [`task::SetupTask`] that are the building blocks of installing, updating
//! and uninstalling an app or resource on the operating system. To start, a list of
//! tasks is grouped into an [`InstallConfig`] that runs in [`SETUP.install`].
//!
//! The [`SetupWizard!`] widget can be used together with [`page`] builders to create the
//! classic Windows setup experience. Usually the wizard starts with a set of config pages,
//! once these pages are finished an [`InstallConfig`] is built and started, the set of pages
//! is replaced with a progress page, once the operation is done the pages are replaced again
//! with a a results page.
//!
//! Note that the wizard widget is not required, a completely custom UI could also build an [`InstallConfig`],
//! the [`SETUP`] service has no connection with what UI is using it, it can even be operated from a command
//! line interface (CLI) in a headless app.
//!
//! Note that this module is only tested on Windows, it should work in other operating systems, some
//! tasks make a minimal effort to be cross platform, but Windows is the only OS without a standard way
//! of installing apps, so it is the focus of this API.
//!
//! # SFX Setup
//!
//! The `cargo zng res --tool sfx` tool can be used to build a self-extracting executable
//! that extracts and runs a setup program implemented using this module. The examples that follow
//! show how to implement a setup app in the same executable as the app that will be installed.
//!
//! The reasons for implementing on the same executable are:
//!
//! * Reduced payload size. All the heavy dependencies like the renderer are shared with the setup app.
//! * Consistent visual identity. Any themes applied to the app are naturally used in the setup app.
//! * Shared config. Settings like language preference selected during setup naturally apply to the installed app.
//! * Shared metadata. The [`zng::env::about`] metadata is used by default in [`SetupWizard!`] pages.
//!
//! ## Pack
//!
//! Before implementing the setup app lets overview how the `cargo zng res` command will be used to package
//! it and the rest of the app data in a single setup executable.
//!
//! The `pack/windows/my-app-setup.exe.zr-sfxf` file:
//!
//! ```toml
//! [sfx]
//! run = "target/release/my-app.exe"
//!
//! [[data]]
//! name = "data"
//! compress = "zstd"
//! file = "target/pack/windows/setup/data.tar"
//! ```
//!
//! The command `cargo zng res --pack "pack/windows" "target/pack/windows"` will generate a `target/pack/windows/my-app-setup.exe`
//! file that contains the executable and data, both compressed. Note that the data is pre-packed into a `tar`, this can be
//! done using the `.zr-tar` tool, outside of the scope of this example.
//!
//! When the `my-app-setup.exe` runs it extracts the `my-app.exe` and runs it with the `SFX_ARGS` environment variable set.
//! Inside the `my-app.exe` this variable causes the setup app to run, instead of the normal app.
//!
//! The `my-app.exe` will run from a temp dir, without any accompanying files. It must use the `SFX_ARGS` to call
//! the `my-app-setup.exe` in server mode to read any extra file it requires to run, and also to extract the install
//! payload.
//!
//! Call `cargo zng res --tool sfx` to read detailed docs about this tool.
//!
//! ## Setup App
//!
//! In this example the setup app is implemented on the same crate as the installed app.
//!
//! ```
//! # mod demo_no_run {
//! use zng::prelude::*;
//!
//! fn main() {
//!     zng::env::init!();
//!
//!     // if "SFX_ARGS" is set intercept and run setup app
//!     #[cfg(windows)]
//!     windows_setup::setup_main();
//!     // if not, run normal app
//!     app_main();
//! }
//!
//! #[cfg(windows)]
//! mod windows_setup {
//!     use zng::prelude::*;
//!     use zng::setup::{task as setup_task, *};
//!
//!     pub fn setup_main() {
//!         // connect if a valid SFX_ARGS is set.
//!         let sfx = match SfxClient::connect_blocking() {
//!             Ok(c) => c,
//!             Err(e) => return assert!(e.is_no_sfx(), "{e}"),
//!         };
//!
//!         // basic CLI parsing
//!         let destination = match sfx.sfx_args().get(1) {
//!             Some(d) => d.clone(),
//!             // print CLI help, SFX replicates all stdout/err
//!             None => return println!("{} DESTINATION", sfx.sfx_args()[0]),
//!         };
//!
//!         let mut app = APP.defaults().run_headless(false);
//!         app.run_task(async move {
//!             // define install operation
//!             let mut install_cfg = InstallConfig::new();
//!
//!             // first task, extract TAR read from SFX
//!             let data = sfx.read("data").await.unwrap().into_blocking().await;
//!             let extract_cfg = setup_task::ExtractTarConfig::from_sfx(data, destination.into());
//!             install_cfg.push::<setup_task::ExtractTar>("extract", extract_cfg);
//!
//!             // install
//!             let uninstall_cfg = SETUP.install(install_cfg).wait_rsp().await.unwrap();
//!         });
//!
//!         // interrupt normal app
//!         zng::env::exit(0);
//!     }
//! }
//! # fn app_main() { }
//! # }
//! ```
//!
//! In the example above a headless app is used to run a very simple install operation configured directly
//! from CLI. In a full setup app both CLI and UI modes should be provided, with a CLI parsed by something
//! more robust like [`clap::Parser::parse_from`].
//!
//! The following example shows a [`SetupWizard!`] that does the same simple install operation, configured
//! by the user using a GUI.
//!
//! ```
//! # use zng::prelude::*;
//! # use zng::setup::{task as setup_task, *};
//! # fn demo(sfx: SfxClient) -> UiNode {
//! let destination_page = page::InstallDirPage::new();
//! let destination = destination_page.install_dir.clone();
//! SetupWizard! {
//!     // Set window title from page
//!     get_title = WINDOW.vars().title();
//!
//!     pages = vec![page::WelcomePage::new("").build(), destination_page.build()];
//!     setup_op = SetupOp::Install;
//!
//!     on_finish = async_hn!(destination, sfx, |args| {
//!         args.propagation.stop();
//!
//!         let mut cfg = InstallConfig::new();
//!
//!         let data = sfx.read("data").await.unwrap().into_blocking().await;
//!         cfg.push::<setup_task::ExtractTar>("extract", setup_task::ExtractTarConfig::from_sfx(data, destination.get()));
//!
//!         let r = SETUP.install(cfg, None);
//!
//!         let uninstall = r.wait_rsp().await.unwrap();
//!     });
//! }
//! # }
//! ```
//!
//! [`SETUP.install`]: SETUP::install
//! [`SetupWizard!`]: struct@SetupWizard
//! [`clap::Parser::parse_from`]: https://docs.rs/clap/latest/clap/trait.Parser.html#method.parse_from
//!
//! # Full API
//!
//! See [`zng_ext_setup`] and [`zng_wgt_setup`] for the full API.

pub use zng_wgt_setup::{APP_ID_VAR, APP_NAME_VAR, APP_ORG_VAR, APP_VERSION_VAR, SETUP_OP_VAR, SetupOp, SetupWizard};

pub use zng_ext_setup::{
    InstallConfig, PreparedInstallConfig, SETUP, SetupError, SetupErrorState, SetupOpStatus, SfxClient, SfxDataInfo, SfxError,
    UninstallConfig,
};

/// Common setup tasks and custom task API.
///
/// Setup tasks can be instantiated with [`InstallConfig::push`], usually after configuration
/// is collected using a [`SetupWizard!`] or CLI.
///
/// [`SetupWizard!`]: struct@SetupWizard
///
/// # Full API
///
/// See [`zng_ext_setup::task`] for the full API.
pub mod task {
    pub use zng_ext_setup::task::{ExtractTar, ExtractTarConfig, SetupTask, SetupTaskError};

    #[cfg(windows)]
    pub use zng_ext_setup::task::{RegisterUninstaller, RegisterUninstallerConfig};

    #[cfg(any(windows, target_os = "linux"))]
    pub use zng_ext_setup::task::{CreateShortcut, CreateShortcutConfig};
}

/// Common setup wizard pages.
///
/// The types in this module are builders for [`zng::wizard::Page`] instances that
/// can be used with [`SetupWizard!`].
///
/// [`SetupWizard!`]: struct@SetupWizard
///
/// # Full API
///
/// See [`zng_wgt_setup::page`] for the full API.
pub mod page {
    pub use zng_wgt_setup::page::{InstallDirPage, WelcomePage};
}
