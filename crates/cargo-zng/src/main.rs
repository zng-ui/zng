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

m#[macro_use]
mod util;
mod l10n;
mod new;

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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::New(args) => new::run(args),
        Command::L10n(args) => l10n::run(args),
    }

    crate::util::exit();
}
