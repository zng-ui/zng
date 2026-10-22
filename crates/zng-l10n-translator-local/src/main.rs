#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Local LLM plugin for `cargo zng l10n`.
//!
//! Optionally define `LOCAL_TRANSLATOR_LLM_URL` is "http://localhost:9931/v1/chat/completions" by default.
//!
//! Optionally define `LOCAL_TRANSLATOR_LLM_MODEL`, is "llama" by default.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::io::{Read, Write};

use clap::*;
use zng_ext_l10n::Lang;

mod local;

/// Local LLVM plugin for `cargo zng l10n`
#[derive(Parser, Debug)]
struct Cli {
    #[arg(long, required_unless_present = "limits")]
    from_lang: Option<Lang>,
    #[arg(long, required_unless_present = "limits")]
    to_lang: Option<Lang>,

    #[arg(
        long,
        default_value = "http://localhost:9931/v1/chat/completions",
        env = "LOCAL_TRANSLATOR_LLM_URL"
    )]
    url: String,
    #[arg(long, default_value = "llama", env = "LOCAL_TRANSLATOR_LLM_MODEL")]
    model: String,

    #[arg(long, action)]
    limits: bool,
}
macro_rules! fatal {
    ($($tt:tt)*) => {
        {
            eprintln!($($tt)*);
            std::process::exit(101);
        }
    };
}

fn main() {
    let args = Cli::parse();

    if args.limits {
        let limits_json = r#"{ "requests-per-minute": 9999 }"#;
        println!("{limits_json}");
        return;
    }

    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        fatal!("cannot read input, {e}");
    }

    let task = local::translate(args.url, args.model, args.from_lang.unwrap(), args.to_lang.unwrap(), input);

    match zng_task::block_on(task) {
        Ok(o) => std::io::stdout().write_all(o.as_bytes()).unwrap(),
        Err(e) => fatal!("cannot translate, {e}"),
    }
}
