#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Local LLM plugin for `cargo zng l10n`.
//!
//! Optionally define `LOCAL_TRANSLATOR_LLM_URL` is "http://localhost:9931/v1/chat/completions" by default.
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
///
/// Optionally define `LOCAL_TRANSLATOR_LLM_URL` is "http://localhost:9931/v1/chat/completions" by default.
#[derive(Parser, Debug)]
struct Cli {
    #[arg(long)]
    from_lang: Lang,
    #[arg(long)]
    to_lang: Lang,
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
    if std::env::args().any(|a| a == "--limits") {
        let limits_json = r#"{ "requests-per-minute": 9999 }"#;
        println!("{limits_json}");
        return;
    }

    let url = match std::env::var("LOCAL_TRANSLATOR_LLM_URL") {
        Ok(m) => {
            if m.len() > 4 && m[..4].eq_ignore_ascii_case("http") {
                m
            } else {
                fatal!("invalid LOCAL_TRANSLATOR_LLM_URL")
            }
        }
        Err(e) => match e {
            std::env::VarError::NotPresent => "http://localhost:9931/v1/chat/completions".to_owned(),
            std::env::VarError::NotUnicode(_) => fatal!("invalid `LOCAL_TRANSLATOR_LLM_URL`"),
        },
    };

    let args = Cli::parse();

    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        fatal!("cannot read input, {e}");
    }

    let task = local::translate(url, args.from_lang, args.to_lang, input);

    match zng_task::block_on(task) {
        Ok(o) => std::io::stdout().write_all(o.as_bytes()).unwrap(),
        Err(e) => fatal!("cannot translate, {e}"),
    }
}
