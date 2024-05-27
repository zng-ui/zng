#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
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
mod l10n;
mod new;
mod res;

/// Utilities for implementing `cargo-zng-res-{tool}` executables.
///
/// Note don't depend on `cargo-zng`, just copy the source code of the utilities you need.
pub mod res_tool_util {
    pub use crate::res::built_in::{ToolCli, ToolRequest, CACHE_DIR};
}

use clap::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Command.
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a new repository from a Zng template repository.
    New(new::NewArgs),
    /// Localization text scraper
    ///
    /// See the docs for `l10n!` for more details about the expected format.
    L10n(l10n::L10nArgs),

    /// Build resources
    ///
    /// Walks SOURCE and delegates `.zr-{tool}` files to `cargo-zng-res-{tool}`
    /// executables and crates.
    Res(res::ResArgs),
}

fn main() {
    res::built_in::run();

    let cli = Cli::parse();

    match cli.command {
        Command::New(args) => new::run(args),
        Command::L10n(args) => l10n::run(args),
        Command::Res(args) => res::run(args),
    }

    crate::util::exit();
}
