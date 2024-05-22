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

pub mod l10n;

use clap::*;

/// Cli
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Localization text scraper
    ///
    /// See the docs for `zng::l10n`
    L10n(l10n::L10nArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::L10n(args) => l10n::run(args),
    }
}
