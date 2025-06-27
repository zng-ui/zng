use std::{
    borrow::Cow,
    fs,
    io::{self, Read, Write},
    path::Path,
    process::Stdio,
};

use clap::*;
use once_cell::sync::Lazy;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use rayon::prelude::*;
use regex::Regex;

use crate::util;

#[derive(Args, Debug, Default)]
pub struct FmtArgs {
    /// Only check if files are formatted
    #[arg(long, action)]
    check: bool,

    /// Format the crate identified by Cargo.toml
    #[arg(long)]
    manifest_path: Option<String>,

    /// Format the workspace crate identified by package name
    #[arg(short, long)]
    package: Option<String>,

    /// Format all files matched by glob
    #[arg(short, long)]
    files: Option<String>,

    /// Format the stdin to the stdout.
    #[arg(short, long, action)]
    stdin: bool,
}

pub fn run(mut args: FmtArgs) {
    let (check, action) = if args.check { ("--check", "checking") } else { ("", "formatting") };

    let mut custom_fmt_files = vec![];

    if args.stdin {
        if args.manifest_path.is_some() || args.package.is_some() || args.files.is_some() {
            fatal!("stdin can only be used standalone or with --check");
        }

        let mut code = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut code) {
            fatal!("stdin read error, {e}");
        }

        if code.is_empty() {
            return;
        }

        if let Some(code) = rustfmt_stdin(&code) {
            let stream: TokenStream = code.parse().unwrap_or_else(|e| fatal!("cannot parse stdin, {e}"));

            let formatted = fmt_code(&code, stream);
            if let Err(e) = std::io::stdout().write_all(formatted.as_bytes()) {
                fatal!("stdout write error, {e}");
            }
        }
    } else if let Some(glob) = args.files {
        if args.manifest_path.is_some() || args.package.is_some() {
            fatal!("--files must not be set when crate is set");
        }

        for file in glob::glob(&glob).unwrap_or_else(|e| fatal!("{e}")) {
            let file = file.unwrap_or_else(|e| fatal!("{e}"));
            if let Err(e) = util::cmd("rustfmt", &["--edition", "2021", check, &file.as_os_str().to_string_lossy()], &[]) {
                fatal!("{e}");
            }
            custom_fmt_files.push(file);
        }
    } else {
        if let Some(pkg) = args.package {
            if args.manifest_path.is_some() {
                fatal!("expected only one of --package, --manifest-path");
            }
            match util::manifest_path_from_package(&pkg) {
                Some(m) => args.manifest_path = Some(m),
                None => fatal!("package `{pkg}` not found in workspace"),
            }
        }
        if let Some(path) = args.manifest_path {
            if let Err(e) = util::cmd("cargo fmt --manifest-path", &[&path, check], &[]) {
                fatal!("{e}");
            }

            let files = Path::new(&path)
                .parent()
                .unwrap()
                .join("**/*.rs")
                .display()
                .to_string()
                .replace('\\', "/");
            for file in glob::glob(&files).unwrap_or_else(|e| fatal!("{e}")) {
                let file = file.unwrap_or_else(|e| fatal!("{e}"));
                custom_fmt_files.push(file);
            }
        } else {
            if let Err(e) = util::cmd("cargo fmt", &[check], &[]) {
                fatal!("{e}");
            }

            for path in util::workspace_manifest_paths() {
                let files = path.parent().unwrap().join("**/*.rs").display().to_string().replace('\\', "/");
                for file in glob::glob(&files).unwrap_or_else(|e| fatal!("{e}")) {
                    let file = file.unwrap_or_else(|e| fatal!("{e}"));
                    custom_fmt_files.push(file);
                }
            }
        }
    }

    custom_fmt_files.par_iter().for_each(|file| {
        if let Err(e) = custom_fmt(file, args.check) {
            fatal!("error {action} `{}`, {e}", file.display());
        }
    });
}

fn custom_fmt(rs_file: &Path, check: bool) -> io::Result<()> {
    let file = fs::read_to_string(rs_file)?;

    // skip UTF-8 BOM
    let file_code = file.strip_prefix('\u{feff}').unwrap_or(file.as_str());
    // skip shebang line
    let file_code = if file_code.starts_with("#!") && !file_code.starts_with("#![") {
        &file_code[file_code.find('\n').unwrap_or(file_code.len())..]
    } else {
        file_code
    };

    let mut formatted_code = file[..file.len() - file_code.len()].to_owned();
    formatted_code.reserve(file.len());

    let file_stream: TokenStream = file_code
        .parse()
        .unwrap_or_else(|e| fatal!("cannot parse `{}`, {e}", rs_file.display()));

    formatted_code.push_str(&fmt_code(file_code, file_stream));

    if formatted_code != file {
        if check {
            fatal!("extended format does not match in file `{}`", rs_file.display());
        }
        fs::write(rs_file, formatted_code)?;
    }

    Ok(())
}

fn fmt_code(code: &str, stream: TokenStream) -> String {
    let mut formatted_code = String::new();
    let mut last_already_fmt_start = 0;

    let mut stream_stack = vec![stream.into_iter()];
    let next = |stack: &mut Vec<proc_macro2::token_stream::IntoIter>| {
        while !stack.is_empty() {
            let tt = stack.last_mut().unwrap().next();
            if tt.is_some() {
                return tt;
            }
            stack.pop();
        }
        None
    };
    let mut tail2 = Vec::with_capacity(2);

    let mut skip_next_group = false;
    while let Some(tt) = next(&mut stream_stack) {
        match tt {
            TokenTree::Group(g) => {
                if tail2.len() == 2
                    && matches!(g.delimiter(), Delimiter::Brace)
                    && matches!(&tail2[0], TokenTree::Punct(p) if p.as_char() == '!')
                    && matches!(&tail2[1], TokenTree::Ident(_))
                {
                    // macro! {}
                    if std::mem::take(&mut skip_next_group) {
                        continue;
                    }

                    let bang = tail2[0].span().byte_range().start;
                    let line_start = code[..bang].rfind('\n').unwrap_or(0);
                    let base_indent = code[line_start..bang]
                        .chars()
                        .skip_while(|&c| c != ' ')
                        .take_while(|&c| c == ' ')
                        .count();

                    let group_bytes = g.span().byte_range();
                    let group_code = &code[group_bytes.clone()];

                    if let Some(formatted) = try_fmt_macro(base_indent, group_code) {
                        if formatted != group_code {
                            // changed by custom format
                            if let Some(stable) = try_fmt_macro(base_indent, &formatted) {
                                if formatted == stable {
                                    // change is sable
                                    let already_fmt = &code[last_already_fmt_start..group_bytes.start];
                                    formatted_code.push_str(already_fmt);
                                    formatted_code.push_str(&formatted);
                                    last_already_fmt_start = group_bytes.end;
                                }
                            }
                        }
                    }
                } else if !tail2.is_empty()
                    && matches!(g.delimiter(), Delimiter::Bracket)
                    && matches!(&tail2[0], TokenTree::Punct(p) if p.as_char() == '#')
                {
                    // #[..]
                    let mut attr = g.stream().into_iter();
                    let attr = [attr.next(), attr.next(), attr.next(), attr.next(), attr.next()];
                    if let [
                        Some(TokenTree::Ident(i0)),
                        Some(TokenTree::Punct(p0)),
                        Some(TokenTree::Punct(p1)),
                        Some(TokenTree::Ident(i1)),
                        None,
                    ] = attr
                    {
                        if i0 == "rustfmt" && p0.as_char() == ':' && p1.as_char() == ':' && i1 == "skip" {
                            // #[rustfmt::skip]
                            skip_next_group = true;
                        }
                    }
                } else if !std::mem::take(&mut skip_next_group) {
                    stream_stack.push(g.stream().into_iter());
                }
                tail2.clear();
            }
            tt => {
                if tail2.len() == 2 {
                    tail2.pop();
                }
                tail2.insert(0, tt);
            }
        }
    }

    formatted_code.push_str(&code[last_already_fmt_start..]);

    if formatted_code != code {
        // custom format can cause normal format to change
        // example: ui_vec![Wgt!{<many properties>}, Wgt!{<same>}]
        //   Wgt! gets custom formatted onto multiple lines, that causes ui_vec![\n by normal format.
        formatted_code = rustfmt_stdin_frag(&formatted_code).unwrap_or(formatted_code);
    }

    formatted_code
}

fn try_fmt_macro(base_indent: usize, group_code: &str) -> Option<String> {
    let mut replaced_code = replace_event_args(group_code, false);
    let is_event_args = matches!(&replaced_code, Cow::Owned(_));

    let mut is_widget = false;
    if !is_event_args {
        replaced_code = replace_widget_when(group_code, false);
        is_widget = matches!(&replaced_code, Cow::Owned(_));

        let tmp = replace_widget_prop(&replaced_code, false);
        if let Cow::Owned(tmp) = tmp {
            is_widget = true;
            replaced_code = Cow::Owned(tmp);
        }
    }

    let mut is_expr_var = false;
    if !is_event_args && !is_widget {
        replaced_code = replace_expr_var(group_code, false);
        is_expr_var = matches!(&replaced_code, Cow::Owned(_));
    }

    let code = rustfmt_stdin_frag(&replaced_code)?;

    let code = if is_event_args {
        replace_event_args(&code, true)
    } else if is_widget {
        let code = replace_widget_when(&code, true);
        let code = replace_widget_prop(&code, true).into_owned();
        Cow::Owned(code)
    } else if is_expr_var {
        replace_expr_var(&code, true)
    } else {
        Cow::Owned(code)
    };

    let code_stream: TokenStream = code.parse().unwrap_or_else(|e| panic!("{e}\ncode:\n{code}"));
    let code_tt = code_stream.into_iter().next().unwrap();
    let code_stream = match code_tt {
        TokenTree::Group(g) => g.stream(),
        _ => unreachable!(),
    };
    let code = fmt_code(&code, code_stream);

    let mut out = String::new();
    let mut lb_indent = String::with_capacity(base_indent + 1);
    for line in code.lines() {
        if line.is_empty() {
            if !lb_indent.is_empty() {
                out.push('\n');
            }
        } else {
            out.push_str(&lb_indent);
        }
        out.push_str(line);
        // "\n    "
        if lb_indent.is_empty() {
            lb_indent.push('\n');
            for _ in 0..base_indent {
                lb_indent.push(' ');
            }
        }
    }
    Some(out)
}
// replace line with only `..` tokens with:
//
// ```
// // cargo-zng::fmt::dot_dot
// }
// impl CargoZngFmt {
//
// ```
fn replace_event_args(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*(\.\.)\s*$").unwrap());
    static MARKER: &str = "// cargo-zng::fmt::dot_dot\n}\nimpl CargoZngFmt {\n";
    static RGX_REV: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)^(\s+)// cargo-zng::fmt::dot_dot\n\s*}\n\s*impl CargoZngFmt\s*\{\n").unwrap());

    if !reverse {
        RGX.replace_all(code, |caps: &regex::Captures| {
            format!(
                "{}{MARKER}{}",
                &caps[0][..caps.get(1).unwrap().start() - caps.get(0).unwrap().start()],
                &caps[0][caps.get(1).unwrap().end() - caps.get(0).unwrap().start()..]
            )
        })
    } else {
        RGX_REV.replace_all(code, "\n$1..\n\n")
    }
}
// replace `prop = 1, 2;` with `prop = (1, 2);`
// AND replace `prop = { a: 1, b: 2, };` with `prop = __A_ { a: 1, b: 2, }`
fn replace_widget_prop(code: &str, reverse: bool) -> Cow<'_, str> {
    static NAMED_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)\w+\s+=\s+(\{)").unwrap());
    static NAMED_MARKER: &str = "__A_ ";

    static UNNAMED_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?ms)\w+\s+=\s+([^\(\{\n\)]+?)(?:;|}$)").unwrap());
    static UNNAMED_RGX_REV: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?ms)__a_\((.+?)\)").unwrap());

    if !reverse {
        let named_rpl = NAMED_RGX.replace_all(code, |caps: &regex::Captures| {
            format!(
                "{}{NAMED_MARKER} {{",
                &caps[0][..caps.get(1).unwrap().start() - caps.get(0).unwrap().start()],
            )
        });
        let mut has_unnamed = false;
        let unnamed_rpl = UNNAMED_RGX.replace_all(&named_rpl, |caps: &regex::Captures| {
            let cap = caps.get(1).unwrap();
            let cap_str = cap.as_str().trim();
            fn more_than_one_expr(code: &str) -> bool {
                let stream: TokenStream = match code.parse() {
                    Ok(s) => s,
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        panic!("{_e}\ncode:\n{code}");
                        #[cfg(not(debug_assertions))]
                        return false;
                    }
                };
                for tt in stream {
                    if let TokenTree::Punct(p) = tt {
                        if p.as_char() == ',' {
                            return true;
                        }
                    }
                }
                false
            }
            if cap_str.contains(",") && more_than_one_expr(cap_str) {
                has_unnamed = true;

                format!(
                    "{}__a_({cap_str}){}",
                    &caps[0][..cap.start() - caps.get(0).unwrap().start()],
                    &caps[0][cap.end() - caps.get(0).unwrap().start()..],
                )
            } else {
                caps.get(0).unwrap().as_str().to_owned()
            }
        });
        if has_unnamed {
            Cow::Owned(unnamed_rpl.into_owned())
        } else {
            named_rpl
        }
    } else {
        let code = UNNAMED_RGX_REV.replace_all(code, |caps: &regex::Captures| {
            format!(
                "{}{}{}",
                &caps[0][..caps.get(1).unwrap().start() - caps.get(0).unwrap().start() - "__a_(".len()],
                caps.get(1).unwrap().as_str(),
                &caps[0][caps.get(1).unwrap().end() + ")".len() - caps.get(0).unwrap().start()..]
            )
        });
        Cow::Owned(code.replace(NAMED_MARKER, ""))
    }
}
// replace `when <expr> { <properties> }` with `for cargo_zng_fmt_when in <expr> { <properties> }`
// AND replace `#expr` with `__P_expr` AND `#{var}` with `__P_!{`
fn replace_widget_when(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)\n\s*(when) .+?\{").unwrap());
    static MARKER: &str = "for cargo_zng_fmt_when in";
    static POUND_MARKER: &str = "__P_";

    if !reverse {
        RGX.replace_all(code, |caps: &regex::Captures| {
            let prefix_spaces = &caps[0][..caps.get(1).unwrap().start() - caps.get(0).unwrap().start()];

            let expr = &caps[0][caps.get(1).unwrap().end() - caps.get(0).unwrap().start()..];
            let expr = POUND_RGX.replace_all(expr, |caps: &regex::Captures| {
                let c = &caps[0][caps.get(1).unwrap().end() - caps.get(0).unwrap().start()..];
                let marker = if c == "{" { POUND_VAR_MARKER } else { POUND_MARKER };
                format!("{marker}{c}")
            });

            format!("{prefix_spaces}{MARKER}{expr}")
        })
    } else {
        let code = code.replace(MARKER, "when");
        let r = POUND_REV_RGX.replace_all(&code, "#").into_owned();
        Cow::Owned(r)
    }
}
static POUND_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(#)[\w\{]").unwrap());
static POUND_REV_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"__P_!?\s?").unwrap());
static POUND_VAR_MARKER: &str = "__P_!";

// replace `#{` with `__P_!{`
fn replace_expr_var(code: &str, reverse: bool) -> Cow<'_, str> {
    if !reverse {
        POUND_RGX.replace(code, |caps: &regex::Captures| {
            let c = &caps[0][caps.get(1).unwrap().end() - caps.get(0).unwrap().start()..];
            if c == "{" {
                Cow::Borrowed("__P_!{")
            } else {
                Cow::Owned(caps[0].to_owned())
            }
        })
    } else {
        POUND_REV_RGX.replace(code, "#")
    }
}

fn rustfmt_stdin_frag(code: &str) -> Option<String> {
    let mut s = std::process::Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    s.stdin.take().unwrap().write_all(format!("fn __try_fmt(){code}").as_bytes()).ok()?;
    let s = s.wait_with_output().ok()?;

    if s.status.success() {
        let code = String::from_utf8(s.stdout).ok()?;
        let code = code.strip_prefix("fn __try_fmt()")?.trim_start().to_owned();
        Some(code)
    } else {
        None
    }
}

fn rustfmt_stdin(code: &str) -> Option<String> {
    let mut s = std::process::Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    s.stdin.take().unwrap().write_all(code.as_bytes()).ok()?;
    let s = s.wait_with_output().ok()?;

    if s.status.success() {
        let code = String::from_utf8(s.stdout).ok()?;
        Some(code)
    } else {
        None
    }
}
