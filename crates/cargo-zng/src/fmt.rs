use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::Stdio,
};

use clap::*;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use rayon::prelude::*;

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
    #[arg(long)]
    package: Option<String>,

    /// Format all files matched by glob
    #[arg(long)]
    files: Option<String>,
}

pub fn run(mut args: FmtArgs) {
    let (check, action) = if args.check { ("--check", "checking") } else { ("", "formatting") };

    let mut custom_fmt_files = vec![];

    if let Some(glob) = args.files {
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
    let file_code = if file_code.starts_with("#!") {
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

    while let Some(tt) = next(&mut stream_stack) {
        match tt {
            TokenTree::Group(g) => {
                if tail2.len() == 2
                    && matches!(g.delimiter(), Delimiter::Brace)
                    && matches!(&tail2[0], TokenTree::Punct(p) if p.as_char() == '!')
                    && matches!(&tail2[1], TokenTree::Ident(_))
                {
                    // macro! {}

                    let bang = tail2[0].span().byte_range().start;
                    let line_start = code[..bang].rfind('\n').unwrap_or(0);
                    let base_indent = code[line_start..bang]
                        .chars()
                        .skip_while(|&c| c != ' ')
                        .take_while(|&c| c == ' ')
                        .count();

                    let group_bytes = g.span().byte_range();
                    let group_code = &code[group_bytes.clone()];

                    if let Some(formatted) = try_fmt_group(base_indent, group_code) {
                        if formatted != group_code {
                            // changed by custom format
                            if let Some(stable) = try_fmt_group(base_indent, &formatted) {
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
                } else {
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
        formatted_code = rustfmt_stdin(&formatted_code).unwrap_or(formatted_code);
    }

    formatted_code
}

fn try_fmt_group(base_indent: usize, group_code: &str) -> Option<String> {
    let code = rustfmt_stdin(group_code)?;

    let code_stream: TokenStream = code.parse().unwrap();
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
fn rustfmt_stdin(code: &str) -> Option<String> {
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

// !!: TODO
//
// * #[rustfmt::skip]
// * event_args! has a '.. fn'  token sequence
// * widget macros with 'when #property'
// * Review 'expr_var!'
// * ra/vscode integration
