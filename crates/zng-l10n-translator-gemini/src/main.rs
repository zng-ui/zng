#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Gemini plugin for `cargo zng l10n --translate`.
//!
//! Define `GEMINI_API_KEY` environment variable.
//!
//! Optionally define `GEMINI_TRANSLATOR_MODEL`, is "gemini-3.1-flash-lite-preview" by default.
//!
//! Call `cargo zng l10n --translate gemini|en->ja "l10n/path/"`
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::io::{Read, Write};

use clap::*;
use zng_ext_l10n::Lang;
mod gemini;

/// Gemini plugin for `cargo zng l10n --translate`
///
/// Define `GEMINI_API_KEY` environment variable
///
/// Optionally define `GEMINI_TRANSLATOR_MODEL`, is "gemini-3.1-flash-lite-preview" by default
///
/// Optionally define `GEMINI_TRANSLATOR_RPM`, is 15 by default
///
/// Call `cargo zng l10n --translate gemini --from-lang en --to-lang ja "l10n/path/"`
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
        let mut rpm = 15u64;
        if let Ok(r) = std::env::var("GEMINI_TRANSLATOR_RPM")
            && let Ok(r) = r.parse::<u64>()
        {
            rpm = r;
        }
        let limits_json = r#"{ "requests-per-minute": <<RPM>> }"#.replace("<<RPM>>", &rpm.to_string());
        println!("{limits_json}");
        return;
    }

    let key = match std::env::var("GEMINI_API_KEY") {
        Ok(k) => {
            if k.is_empty() {
                fatal!("missing `GEMINI_API_KEY` env var value");
            } else {
                k
            }
        }
        Err(e) => match e {
            std::env::VarError::NotPresent => fatal!("missing `GEMINI_API_KEY` env var"),
            std::env::VarError::NotUnicode(_) => fatal!("invalid `GEMINI_API_KEY`"),
        },
    };
    let model = match std::env::var("GEMINI_TRANSLATOR_MODEL") {
        Ok(m) => {
            if m.is_empty() {
                fatal!("missing `GEMINI_TRANSLATOR_MODEL` env var value");
            } else {
                m
            }
        }
        Err(e) => match e {
            std::env::VarError::NotPresent => "gemini-3.1-flash-lite-preview".to_owned(),
            std::env::VarError::NotUnicode(_) => fatal!("invalid `GEMINI_TRANSLATOR_MODEL`"),
        },
    };

    let args = Cli::parse();

    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        fatal!("cannot read input, {e}");
    }

    let task = gemini::translate(key, model, args.from_lang, args.to_lang, input);

    match zng_task::block_on(task) {
        Ok(o) => std::io::stdout().write_all(o.as_bytes()).unwrap(),
        Err(e) => fatal!("cannot translate, {e}"),
    }
}
