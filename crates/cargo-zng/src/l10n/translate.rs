use std::{borrow::Cow, path::Path, process::Command};

pub(crate) fn translate(translate: &str, translate_from: &str, translate_to: &[String], check: bool, verbose: bool) {
    let from = if translate_from.is_empty() {
        Path::new(translate).file_name().unwrap().to_str().unwrap().to_owned()
    } else {
        translate_from.to_owned()
    };
    for to in translate_to {
        crate::l10n::generate_util::generate(
            translate,
            &format!("{to}-machine"),
            "### Machine translated using cargo zng l10n",
            &|s| Cow::Owned(translate_text(&from, to, s, "")),
            check,
            verbose,
        );
    }
}

fn translate_text(from: &str, to: &str, text: &str, comments: &str) -> String {
    let mut cmd = CARGO_ZNG_TRANSLATE.with(|a| {
        let mut args = a.iter().map(|a| {
            a.replace("{text}", text)
                .replace("{from}", from)
                .replace("{to}", to)
                .replace("{comments}", comments)
        });
        let mut cmd = Command::new(args.next().unwrap_or_default());
        cmd.args(args);
        cmd
    });

    let output = cmd.output().unwrap_or_else(|e| fatal!("cannot spawn translate service, {e}"));

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        fatal!("translate failed, {}\n{err}", output.status);
    }

    let r = match String::from_utf8(output.stdout) {
        Ok(r) => r,
        Err(e) => fatal!("translate service output not valid UTF-8, {e}"),
    };

    if r.is_empty() {
        fatal!("translate service did not return translation in stdout");
    }

    r.to_owned()
}
thread_local! {
    static CARGO_ZNG_TRANSLATE: Vec<String> = {
        let s = match std::env::var("CARGO_ZNG_TRANSLATE") {
            Ok(v) => v,
            Err(e) => fatal!("cannot read CARGO_ZNG_TRANSLATE, {e}"),
        };
        let a = parse_args(&s);
        let mut from = false;
        let mut to = false;
        let mut text = false;
        for a in &a {
            from |= a.contains("{from}");
            to |= a.contains("{to}");
            text |= a.contains("{text}");
        }
        if !from {
            fatal!("CARGO_ZNG_TRANSLATE missing {{from}}");
        }
        if !to {
            fatal!("CARGO_ZNG_TRANSLATE missing {{to}}");
        }
        if !text {
            fatal!("CARGO_ZNG_TRANSLATE missing {{text}}");
        }
        a
    }
}
fn parse_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();

    let mut chars = input.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' | '"' => {
                if quote == Some(c) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(c);
                } else {
                    current.push(c);
                }
            }
            c if c.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
