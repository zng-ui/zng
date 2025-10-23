#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Zng project management.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

#[macro_use]
mod util;
mod fmt;
mod l10n;
mod new;
mod res;
mod trace;

/// Utilities for implementing `cargo-zng-res-{tool}` executables.
///
/// Note don't depend on `cargo-zng`, just copy the source code of the utilities you need.
pub mod res_tool_util {
    pub use crate::res::built_in::{
        ZR_APP, ZR_CACHE_DIR, ZR_CRATE_NAME, ZR_DESCRIPTION, ZR_FINAL, ZR_HELP, ZR_HOMEPAGE, ZR_LICENSE, ZR_ORG, ZR_PKG_AUTHORS,
        ZR_PKG_NAME, ZR_QUALIFIER, ZR_REQUEST, ZR_REQUEST_DD, ZR_SOURCE_DIR, ZR_TARGET, ZR_TARGET_DD, ZR_TARGET_DIR, ZR_VERSION,
        ZR_WORKSPACE_DIR, display_path, path,
    };
}

use clap::*;

#[derive(Parser)] // requires `derive` feature
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Zng(Zng),
}

#[derive(Args, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Zng {
    /// Command.
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Format code and macros
    ///
    /// Runs cargo fmt and formats Zng macros
    Fmt(fmt::FmtArgs),
    /// New project from a Zng template repository.
    New(new::NewArgs),
    /// Localization text scraper
    ///
    /// See the docs for `l10n!` for more details about the expected format.
    L10n(l10n::L10nArgs),

    /// Build resources
    ///
    /// Builds resources SOURCE to TARGET, delegates `.zr-{tool}` files to `cargo-zng-res-{tool}`
    /// executables and crates.
    Res(res::ResArgs),

    /// Run an app with trace recording enabled.
    ///
    /// The app must be built with `"trace_recorder"` feature enabled.
    Trace(trace::TraceArgs),
}

fn main() {
    res::built_in::run();

    let CargoCli::Zng(cli) = CargoCli::parse();

    match cli.command {
        Command::Fmt(args) => fmt::run(args),
        Command::New(args) => new::run(args),
        Command::L10n(args) => l10n::run(args),
        Command::Res(args) => res::run(args),
        Command::Trace(args) => trace::run(args),
    }

    crate::util::exit();
}
