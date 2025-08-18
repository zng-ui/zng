use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    io::{self, BufRead, Read, Write},
    ops,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    task::Poll,
    time::{Duration, SystemTime},
};

use clap::*;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rayon::prelude::*;
use regex::Regex;
use sha2::Digest;

use crate::util;

/// Bump this for every change that can affect format result.
const FMT_VERSION: &str = "1";

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

    /// Format the stdin to the stdout
    #[arg(short, long, action)]
    stdin: bool,

    /// Rustfmt style edition, enforced for all files
    #[arg(long, default_value = "2024")]
    edition: String,

    /// Output rustfmt stderr, for debugging
    #[arg(long, action, hide = true)]
    rustfmt_errors: bool,
}

pub fn run(mut args: FmtArgs) {
    if args.rustfmt_errors {
        SHOW_RUSTFMT_ERRORS.store(true, Relaxed);
    }

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

        if let Some(code) = rustfmt_stdin(&code, &args.edition) {
            let stream = code.parse().unwrap_or_else(|e| fatal!("cannot parse stdin, {e}"));

            let fmt_server = FmtFragServer::spawn(args.edition.clone());
            let mut formatted = Box::pin(try_fmt_child_macros(&code, stream, &fmt_server));
            let formatted = loop {
                std::thread::sleep(Duration::from_millis(50));
                match formatted
                    .as_mut()
                    .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
                {
                    Poll::Ready(r) => break r,
                    Poll::Pending => {}
                }
            };

            if let Err(e) = std::io::stdout().write_all(formatted.as_bytes()) {
                fatal!("stdout write error, {e}");
            }
        }

        return;
    }

    let mut file_patterns = vec![];
    if let Some(glob) = &args.files {
        file_patterns.push(PathBuf::from(glob));
    }
    if let Some(pkg) = &args.package {
        if args.manifest_path.is_some() {
            fatal!("expected only one of --package, --manifest-path");
        }
        match util::manifest_path_from_package(pkg) {
            Some(m) => args.manifest_path = Some(m),
            None => fatal!("package `{pkg}` not found in workspace"),
        }
    }
    let manifest_paths = if let Some(path) = &args.manifest_path {
        vec![PathBuf::from(path)]
    } else if args.files.is_none() {
        let workspace_root = workspace_root().unwrap_or_else(|e| fatal!("cannot find workspace root, {e}"));
        file_patterns.push(workspace_root.join("README.md"));
        file_patterns.push(workspace_root.join("docs/**/*.md"));
        util::workspace_manifest_paths()
    } else {
        vec![]
    };
    for path in manifest_paths {
        let r = (|path: &Path| -> Result<(), Box<dyn std::error::Error>> {
            let path = path.parent().ok_or("root dir")?;
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let ft = entry.file_type()?;
                if ft.is_dir() {
                    if entry.file_name() != "target" {
                        file_patterns.push(entry.path().join("**/*.rs"));
                        file_patterns.push(entry.path().join("**/*.md"));
                    }
                } else if ft.is_file() {
                    file_patterns.push(entry.path());
                }
            }

            Ok(())
        })(&path);
        if let Err(e) = r {
            error!("failed to select files for {}, {e}", path.display())
        }
    }

    // if the args are on record only select files modified since then
    let mut history = match FmtHistory::load() {
        Ok(h) => h,
        Err(e) => {
            warn!("cannot load fmt history, {e}");
            FmtHistory::default()
        }
    };
    let cutout_time = history.insert(&args);
    let files: HashSet<PathBuf> = file_patterns
        .into_par_iter()
        .flat_map(|pattern| {
            let files = match glob::glob(&pattern.display().to_string().replace('\\', "/")) {
                Ok(f) => f.flat_map(|e| e.ok()).collect(),
                Err(_) => vec![],
            };
            files
                .into_par_iter()
                .filter(|f| matches!(f.extension(), Some(ext) if ext == "rs" || ext == "md"))
        })
        .collect();

    let mut files: Vec<_> = files
        .into_par_iter()
        .filter_map(|p| {
            if let Ok(meta) = std::fs::metadata(&p)
                && let Ok(modified) = meta.modified()
            {
                let modified = FmtHistory::time(modified);
                if modified > cutout_time {
                    return Some((p, modified));
                };
            }
            None
        })
        .collect();

    // latest modified first
    files.sort_by(|a, b| b.1.cmp(&a.1));

    let files: Vec<_> = files.into_iter().map(|(p, _)| p).collect();

    let fmt_server = FmtFragServer::spawn(args.edition.clone());

    files.par_chunks(64).for_each(|c| {
        // apply normal format first
        rustfmt_files(c, &args.edition, args.check);
    });

    // apply custom format
    let check = args.check;
    let fmt_server2 = fmt_server.clone();
    let mut futs: Vec<_> = files
        .par_iter()
        .map(move |file| {
            let fmt_server = fmt_server2.clone();
            Some(Box::pin(async move {
                let is_rs = file.extension().unwrap() == "rs";
                let r = if is_rs {
                    custom_fmt_rs(file.clone(), check, fmt_server).await
                } else {
                    debug_assert!(file.extension().unwrap() == "md");
                    custom_fmt_md(file.clone(), check, fmt_server).await.map(|_| None)
                };
                match r {
                    Ok(r) => r,
                    Err(e) => {
                        error!("{e}");
                        None
                    }
                }
            }))
        })
        .collect();

    let reformat = Mutex::new(vec![]);
    loop {
        std::thread::sleep(Duration::from_millis(25));
        futs.par_iter_mut().for_each(|f| {
            match f
                .as_mut()
                .unwrap()
                .as_mut()
                .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            {
                Poll::Ready(changed) => {
                    if let Some(p) = changed {
                        reformat.lock().push(p);
                    }
                    *f = None
                }
                Poll::Pending => {}
            }
        });
        futs.retain(|t| t.is_some());
        if futs.is_empty() {
            break;
        }
    }

    let reformat = reformat.into_inner();
    if !reformat.is_empty() {
        reformat.par_chunks(64).for_each(|c| {
            // apply normal format again, see `custom_fmt` docs
            rustfmt_files(c, &args.edition, args.check);
        });
    }

    if let Err(e) = history.save() {
        warn!("cannot save fmt history, {e}")
    }
}

/// Applies custom format for all macro bodies in file, if changed writes the file and returns
/// the file path for reformat.
///
/// Changed files need reformat in cases like `[Macro!{..}, Macro!{..}]` where the custom macro format
/// introduces line break, this causes rustfmt to also make the `[]` multiline.
async fn custom_fmt_rs(rs_file: PathBuf, check: bool, fmt: FmtFragServer) -> io::Result<Option<PathBuf>> {
    let file = fs::read_to_string(&rs_file)?;

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

    let file_stream = match file_code.parse() {
        Ok(s) => s,
        Err(e) => {
            if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                error!("cannot parse `{}`, {e}", rs_file.display());
            }
            return Ok(None);
        }
    };
    formatted_code.push_str(&try_fmt_child_macros(file_code, file_stream, &fmt).await);

    let formatted_code = custom_fmt_docs(&formatted_code, &fmt, &rs_file).await;

    if formatted_code != file {
        if check {
            fatal!("format does not match in file `{}`", rs_file.display());
        }
        fs::write(&rs_file, formatted_code)?;
        Ok(Some(rs_file))
    } else {
        Ok(None)
    }
}
async fn custom_fmt_docs(code: &str, fmt: &FmtFragServer, rs_file: &Path) -> String {
    let mut formatted_code = String::new();
    let mut lines = code.lines().peekable();
    while let Some(mut line) = lines.next() {
        let maybe = line.trim_start();
        if maybe.starts_with("//!") || maybe.starts_with("///") {
            // enter docs sequence
            let prefix = &line[..line.find("//").unwrap() + 3];
            loop {
                // push markdown lines, or doctest header line
                formatted_code.push_str(line);
                formatted_code.push('\n');

                let doc_line = line.strip_prefix(prefix).unwrap();
                match doc_line.trim().strip_prefix("```") {
                    Some("" | "rust" | "should_panic" | "no_run" | "edition2015" | "edition2018" | "edition2021" | "edition2024") => {
                        // is doctest header line

                        let mut code = String::new();
                        let mut close_line = "";
                        while let Some(l) = lines.next_if(|l| l.starts_with(prefix)) {
                            let doc_line = l.strip_prefix(prefix).unwrap();
                            if doc_line.trim_start().starts_with("```") {
                                close_line = l;
                                break;
                            }
                            code.push_str(doc_line);
                            code.push('\n');
                        }

                        static HIDDEN_LINES_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^ *#(?: +(.*)$|$)").unwrap());
                        if !close_line.is_empty() // is properly closed
                        && !code.trim().is_empty() // is not empty
                        && let Some(mut code) = {
                            let escaped = format!("fn __zng_fmt() {{\n{}\n}}", HIDDEN_LINES_RGX.replace_all(&code, "// __# $1"));
                            fmt.format(escaped).await
                        } {
                            // rustfmt ok

                            let stream = code
                                .parse::<proc_macro2::TokenStream>()
                                .map(pm2_send::TokenStream::from)
                                .map_err(|e| e.to_string());
                            match stream {
                                Ok(s) => code = try_fmt_child_macros(&code, s, fmt).await,
                                Err(e) => {
                                    if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                                        error!("cannot parse doctest block in `{}`, {e}", rs_file.display());
                                    }
                                }
                            };

                            let code = code
                                .strip_prefix("fn __zng_fmt() {")
                                .unwrap()
                                .trim_end()
                                .strip_suffix('}')
                                .unwrap()
                                .replace("// __# ", "# ")
                                .replace("// __#", "#");
                            let mut fmt_code = String::new();
                            let mut wrapper_tabs = String::new();
                            for line in code.lines() {
                                if line.trim().is_empty() {
                                    fmt_code.push('\n');
                                } else {
                                    if wrapper_tabs.is_empty() {
                                        for _ in 0..(line.len() - line.trim_start().len()) {
                                            wrapper_tabs.push(' ');
                                        }
                                    }
                                    fmt_code.push_str(line.strip_prefix(&wrapper_tabs).unwrap_or(line));
                                    fmt_code.push('\n');
                                }
                            }
                            for line in fmt_code.trim().lines() {
                                formatted_code.push_str(prefix);
                                if !line.trim().is_empty() {
                                    formatted_code.push(' ');
                                    formatted_code.push_str(line);
                                }
                                formatted_code.push('\n');
                            }
                        } else {
                            // failed format
                            for line in code.lines() {
                                formatted_code.push_str(prefix);
                                formatted_code.push_str(line);
                                formatted_code.push('\n');
                            }
                        }
                        if !close_line.is_empty() {
                            formatted_code.push_str(close_line);
                            formatted_code.push('\n');
                        }
                    }
                    Some(_) => {
                        // is Markdown code block for `ignore`, `compile_fail` or another language
                        while let Some(l) = lines.next_if(|l| l.starts_with(prefix)) {
                            formatted_code.push_str(l);
                            formatted_code.push('\n');
                            let doc_line = l.strip_prefix(prefix).unwrap();
                            if doc_line.trim_start().starts_with("```") {
                                break;
                            }
                        }
                    }
                    None => {}
                }

                match lines.next_if(|l| l.starts_with(prefix)) {
                    Some(l) => line = l, // advance to next line in the same doc sequence
                    None => break,       // continue to seek next doc sequence
                }
            }
        } else {
            // normal code lines, already formatted
            formatted_code.push_str(line);
            formatted_code.push('\n');
        }
    }
    formatted_code
}
/// Applies rustfmt and custom format for all Rust code blocks in the Markdown file
async fn custom_fmt_md(md_file: PathBuf, check: bool, fmt: FmtFragServer) -> io::Result<()> {
    let file = fs::read_to_string(&md_file)?;

    let mut formatted = String::new();

    let mut lines = file.lines();
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("```rust") {
            formatted.push_str(line);
            formatted.push('\n');

            let mut code = String::new();
            let mut close_line = "";
            for line in lines.by_ref() {
                if line.trim_start().starts_with("```") {
                    close_line = line;
                    break;
                } else {
                    code.push_str(line);
                    code.push('\n');
                }
            }

            if close_line.is_empty() {
                formatted.push_str(&code);
                continue;
            }

            // format code block
            if !code.trim().is_empty()
                && let Some(mut code) = fmt.format(format!("fn __zng_fmt() {{\n{code}\n}}")).await
            {
                // rustfmt ok

                let stream = code
                    .parse::<proc_macro2::TokenStream>()
                    .map(pm2_send::TokenStream::from)
                    .map_err(|e| e.to_string());
                match stream {
                    Ok(s) => code = try_fmt_child_macros(&code, s, &fmt).await,
                    Err(e) => {
                        if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                            error!("cannot parse code block in `{}`, {e}", md_file.display());
                        }
                    }
                };
                let code = code.strip_prefix("fn __zng_fmt() {").unwrap().trim_end().strip_suffix('}').unwrap();
                let mut fmt_code = String::new();
                let mut wrapper_tabs = String::new();
                for line in code.lines() {
                    if line.trim().is_empty() {
                        fmt_code.push('\n');
                    } else {
                        if wrapper_tabs.is_empty() {
                            for _ in 0..(line.len() - line.trim_start().len()) {
                                wrapper_tabs.push(' ');
                            }
                        }
                        fmt_code.push_str(line.strip_prefix(&wrapper_tabs).unwrap_or(line));
                        fmt_code.push('\n');
                    }
                }
                formatted.push_str(fmt_code.trim());
                formatted.push('\n');
            } else {
                formatted.push_str(&code);
            }
            formatted.push_str(close_line);
            formatted.push('\n');
        } else {
            formatted.push_str(line);
            formatted.push('\n')
        }
    }

    if formatted != file {
        if check {
            fatal!("format does not match in file `{}`", md_file.display());
        }
        fs::write(&md_file, formatted)?;
    }

    Ok(())
}

/// Find "macro_ident! {}" and `try_fmt_macro` the macro body if it is not marked skip
async fn try_fmt_child_macros(code: &str, stream: pm2_send::TokenStream, fmt: &FmtFragServer) -> String {
    let mut formatted_code = String::new();
    let mut last_already_fmt_start = 0;

    let mut stream_stack = vec![stream.into_iter()];
    let next = |stack: &mut Vec<std::vec::IntoIter<pm2_send::TokenTree>>| {
        while !stack.is_empty() {
            let tt = stack.last_mut().unwrap().next();
            if let Some(tt) = tt {
                return Some(tt);
            }
            stack.pop();
        }
        None
    };
    let mut tail2: Vec<pm2_send::TokenTree> = Vec::with_capacity(2);

    let mut skip_next_group = false;
    while let Some(tt) = next(&mut stream_stack) {
        match tt {
            pm2_send::TokenTree::Group(g) => {
                if tail2.len() == 2
                    && matches!(g.delimiter(), pm2_send::Delimiter::Brace)
                    && matches!(&tail2[0], pm2_send::TokenTree::Punct(p) if p.as_char() == '!')
                    && matches!(&tail2[1], pm2_send::TokenTree::Ident(_))
                {
                    // macro! {}
                    if std::mem::take(&mut skip_next_group) {
                        continue;
                    }
                    if let pm2_send::TokenTree::Ident(i) = &tail2[1] {
                        if i == &"__P_" || i == &"quote" || i == &"quote_spanned" || i == &"parse_quote" || i == &"parse_quote_spanned" {
                            continue;
                        }
                    } else {
                        unreachable!()
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

                    if let Some(formatted) = try_fmt_macro(base_indent, group_code, fmt).await
                        && formatted != group_code
                    {
                        // changed by custom format
                        if let Some(stable) = try_fmt_macro(base_indent, &formatted, fmt).await {
                            if formatted == stable {
                                // change is sable
                                let already_fmt = &code[last_already_fmt_start..group_bytes.start];
                                formatted_code.push_str(already_fmt);
                                formatted_code.push_str(&formatted);
                                last_already_fmt_start = group_bytes.end;
                            } else if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                                error!("unstable format skipped");
                                // println!("FMT1:\n{formatted}\nFMT2:\n{stable}");
                            }
                        }
                    }
                } else if !tail2.is_empty()
                    && matches!(g.delimiter(), pm2_send::Delimiter::Bracket)
                    && matches!(&tail2[0], pm2_send::TokenTree::Punct(p) if p.as_char() == '#')
                {
                    // #[attribute ..]

                    struct Path(Vec<pm2_send::Ident>);
                    fn take_path(iter: &mut impl Iterator<Item = pm2_send::TokenTree>) -> Path {
                        use pm2_send::TokenTree::*;
                        let mut r = Path(vec![]);
                        let mut iter = iter.peekable();
                        'outer: while let Some(Ident(i)) = iter.next_if(|tt| matches!(tt, Ident(_))) {
                            r.0.push(i);
                            for _ in 0..2 {
                                if !matches!(iter.peek(), Some(Punct(p)) if p.as_char() == ':') {
                                    break 'outer;
                                }
                            }
                        }
                        r
                    }

                    // #[rustfmt::skip]
                    fn is_skip(stream: pm2_send::TokenStream) -> bool {
                        let mut attr = stream.into_iter();
                        let path = take_path(&mut attr);
                        path.0.len() == 2 && path.0[0] == "rustfmt" && path.0[1] == "skip"
                    }

                    // #[path::widget($crate::Foo { <macro_rules> })]
                    fn is_widget_custom(stream: pm2_send::TokenStream) -> Option<pm2_send::Group> {
                        use pm2_send::{TokenTree::*, Delimiter};
                        let mut attr = stream.into_iter();
                        let path = take_path(&mut attr);
                        if let Some(ident) = path.0.last() && ident == &"widget" && let Some(Group(g)) = attr.next() && g.delimiter() == Delimiter::Parenthesis {
                            let mut attr = g.stream().into_iter();
                            if let Some(Punct(p)) = attr.next() && p.as_char() == '$' && !take_path(&mut attr).0.is_empty()
                                && let Some(Group(g)) = attr.next() && g.delimiter() == Delimiter::Brace
                             {
                                return Some(g)
                            }
                        } 
                        None
                    }

                    if is_skip(g.stream()) {
                        skip_next_group = true;
                    } else if let Some(macro_rules_block) = is_widget_custom(g.stream()) {
                        // !!: IMPORTANT to fix unstable formatting, adjust spaces to match opening #[]
                        todo!("!!: replace with macro_rules for formatting")
                    }
                } else if !std::mem::take(&mut skip_next_group) {
                    stream_stack.push(g.stream().into_iter());
                }
                tail2.clear();
            }
            ref tt1 @ pm2_send::TokenTree::Ident(ref i) if i == &"macro_rules" => {
                if let Some(tt2) = next(&mut stream_stack) {
                    if matches!(tt2, pm2_send::TokenTree::Punct(ref p) if p.as_char() == '!') {
                        // macro_rules!
                        next(&mut stream_stack); // skip ident
                        next(&mut stream_stack); // skip body
                    } else {
                        tail2.clear();
                        tail2.push(tt2);
                        tail2.push(tt1.clone());
                    }
                } else {
                    // end
                }
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
    formatted_code
}

async fn try_fmt_macro(base_indent: usize, group_code: &str, fmt: &FmtFragServer) -> Option<String> {
    // replace supported macro syntax to equivalent valid Rust for rustfmt
    let mut replaced_code = Cow::Borrowed(group_code);

    let mut is_lazy_static = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_static_ref(group_code, false);
        is_lazy_static = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_bitflags = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_bitflags(group_code, false);
        is_bitflags = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_event_args = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_event_args(group_code, false);
        is_event_args = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_command = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_command(group_code, false);
        is_command = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_event_property = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_event_property(group_code, false);
        is_event_property = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_widget_impl = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_widget_impl(group_code, false);
        is_widget_impl = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_widget = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_widget(group_code, false);
        is_widget = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_expr_var = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_expr_var(group_code, false);
        is_expr_var = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_struct_like = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_struct_like(group_code, false);
        is_struct_like = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_simple_list = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_simple_ident_list(group_code, false);
        is_simple_list = matches!(&replaced_code, Cow::Owned(_));
    }

    let mut is_when_var = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_when_var(group_code, false);
        is_when_var = matches!(&replaced_code, Cow::Owned(_));
    }

    let replaced_code = replaced_code.into_owned();
    // fmt inner macros first, their final format can affect this macros format
    let code_stream: pm2_send::TokenStream = match replaced_code.parse() {
        Ok(t) => t,
        Err(e) => {
            if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                error!("internal error: {e}");
                eprintln!("CODE:\n{replaced_code}");
            }
            return None;
        }
    };
    let mut inner_group = None;
    for tt in code_stream {
        if let pm2_send::TokenTree::Group(g) = tt {
            // find the inner block, some replacements add prefixes or swap the delimiters
            inner_group = Some(g);
            break;
        }
    }
    let code_stream = match inner_group {
        Some(g) => g.stream(),
        None => {
            if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                error!("internal error, invalid replacement");
                eprintln!("CODE:\n{replaced_code}");
            }
            return None;
        }
    };
    let code = Box::pin(try_fmt_child_macros(&replaced_code, code_stream, fmt)).await;

    // apply rustfmt
    let code = fmt.format(code).await?;

    // restore supported macro syntax
    let code = if is_event_args {
        replace_event_args(&code, true)
    } else if is_widget {
        replace_widget(&code, true)
    } else if is_expr_var {
        replace_expr_var(&code, true)
    } else if is_lazy_static {
        replace_static_ref(&code, true)
    } else if is_command {
        replace_command(&code, true)
    } else if is_event_property {
        replace_event_property(&code, true)
    } else if is_struct_like {
        replace_struct_like(&code, true)
    } else if is_bitflags {
        replace_bitflags(&code, true)
    } else if is_simple_list {
        replace_simple_ident_list(&code, true)
    } else if is_widget_impl {
        replace_widget_impl(&code, true)
    } else if is_when_var {
        replace_when_var(&code, true)
    } else {
        Cow::Owned(code)
    };

    // restore indent
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
// replace `static IDENT = {` with `static IDENT: __fmt__ = {`
// AND replace `static IDENT;` with `static IDENT: __fmt__ = T;`
// AND replace `l10n!: ` with `l10n__fmt:`
fn replace_command(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)([^'])static +(\w+) ?= ?\{").unwrap());
    static RGX_DEFAULTS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)([^'])static +(\w+) ?;").unwrap());
    if !reverse {
        let cmd = RGX_DEFAULTS.replace_all(code, "${1}static $2: __fmt__ = T;");
        let mut cmd2 = RGX.replace_all(&cmd, "${1}static $2: __fmt__ = __A_ {");
        if let Cow::Owned(cmd) = &mut cmd2 {
            *cmd = cmd.replace("l10n!:", "l10n__fmt:");
        }
        match cmd2 {
            Cow::Borrowed(_) => cmd,
            Cow::Owned(s) => Cow::Owned(s),
        }
    } else {
        Cow::Owned(
            code.replace(": __fmt__ = T;", ";")
                .replace(": __fmt__ = __A_ {", " = {")
                .replace("l10n__fmt:", "l10n!:"),
        )
    }
}
// replace ` fn ident = { content }` with ` static __fmt_fn__ident: T = __A_ { content };/*__fmt*/`
fn replace_event_property(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m) fn +(\w+) +\{").unwrap());
    if !reverse {
        let mut r = RGX.replace_all(code, " static __fmt_fn__$1: T = __A_ {");
        if let Cow::Owned(r) = &mut r {
            const OPEN: &str = ": T = __A_ {";
            const CLOSE_MARKER: &str = "; /*__fmt*/";
            let mut start = 0;
            while let Some(i) = r[start..].find(OPEN) {
                let i = start + i + OPEN.len();
                let mut count = 1;
                let mut close_i = i;
                for (ci, c) in r[i..].char_indices() {
                    match c {
                        '{' => count += 1,
                        '}' => {
                            count -= 1;
                            if count == 0 {
                                close_i = i + ci + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                r.insert_str(close_i, CLOSE_MARKER);
                start = close_i + CLOSE_MARKER.len();
            }
        }
        r
    } else {
        Cow::Owned(
            code.replace(" static __fmt_fn__", " fn ")
                .replace(": T = __A_ {", " {")
                .replace("}; /*__fmt*/", "}"),
        )
    }
}
/// Escape widget macro syntax
fn replace_widget(code: &str, reverse: bool) -> Cow<'_, str> {
    static IGNORE_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m): +\w+\s+=\s+\{").unwrap());
    static PROPERTY_NAME_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*([\w:<>]+)\s+=\s+").unwrap());

    #[derive(Debug)]
    enum Item<'s> {
        Property { name: &'s str, value: &'s str },
        PropertyShorthand(&'s str),
        When { expr: &'s str, items: Vec<Item<'s>> },
        Text(&'s str),
        WidgetSetSelfExpr(&'s str),
    }
    #[allow(unused)]
    struct Error<'s> {
        partial: Vec<Item<'s>>,
        error: &'static str,
    }
    impl<'s> Error<'s> {
        fn new(partial: Vec<Item<'s>>, error: &'static str) -> Self {
            Self { partial, error }
        }
    }
    fn parse<'s>(code: &'s str, stream: proc_macro2::TokenStream, code_span: ops::Range<usize>) -> Result<Vec<Item<'s>>, Error<'s>> {
        use proc_macro2::{Delimiter, TokenTree as Tt};

        let mut items = vec![];
        let mut stream = stream.into_iter().peekable();
        let mut text_start = code_span.start;

        if code_span.start == 1 {
            if let Some(Tt::Group(g)) = stream.next()
                && g.delimiter() == Delimiter::Brace
            {
                stream = g.stream().into_iter().peekable();
            } else {
                return Err(Error::new(items, "expected macro block at root"));
            }

            if let Some(Tt::Punct(p)) = stream.peek()
                && p.as_char() == '&'
            {
                // widget_set! first line can be "&mut self_ident;"
                let amp = stream.next().unwrap();
                if let Some(Tt::Ident(m)) = stream.next()
                    && m == "mut"
                {
                    let start = amp.span().byte_range().start;
                    for tt in stream.by_ref() {
                        if let Tt::Punct(p) = tt
                            && p.as_char() == ';'
                        {
                            let end = p.span().byte_range().end;
                            items.push(Item::Text(&code[text_start..start]));
                            items.push(Item::WidgetSetSelfExpr(&code[start..end]));
                            text_start = end;
                            break;
                        }
                    }
                }
                if text_start == code_span.start {
                    return Err(Error::new(items, "expected &mut <self>"));
                }
            }
        }
        'outer: while let Some(tt_attr_or_name) = stream.next() {
            // skip attributes
            if let Tt::Punct(p) = &tt_attr_or_name
                && p.as_char() == '#'
            {
                if let Some(Tt::Group(g)) = stream.next()
                    && g.delimiter() == proc_macro2::Delimiter::Bracket
                {
                    continue 'outer;
                } else {
                    return Err(Error::new(items, "expected attribute"));
                }
            }

            // match property name or when
            if let Tt::Ident(ident) = &tt_attr_or_name {
                if ident == "when" {
                    items.push(Item::Text(&code[text_start..ident.span().byte_range().start]));

                    // `when` is like an `if <expr> <block>`, the <expr> can be <ident><block> (Foo { }.expr())
                    // easiest way to deal with this is to seek/peek the next when or property
                    let expr_start = ident.span().byte_range().end;
                    if stream.next().is_some()
                        && let Some(mut tt_block) = stream.next()
                    {
                        // needs at least two
                        loop {
                            if let Tt::Group(g) = &tt_block
                                && g.delimiter() == Delimiter::Brace
                                && stream
                                    .peek()
                                    .map(|tt| matches!(tt, Tt::Ident(_)) || matches!(tt, Tt::Punct(p) if p.as_char() == '#'))
                                    .unwrap_or(true)
                            {
                                // peek next is property_name, attribute, when or eof
                                let block_span = g.span().byte_range();
                                let expr = &code[expr_start..block_span.start];
                                items.push(Item::When {
                                    expr,
                                    items: parse(code, g.stream(), g.span_open().byte_range().end..g.span_close().byte_range().start)?,
                                });
                                text_start = block_span.end;
                                continue 'outer;
                            }
                            // take expr
                            if let Some(tt) = stream.next() {
                                tt_block = tt;
                            } else {
                                break;
                            }
                        }
                    } else {
                        return Err(Error::new(items, "expected when expression and block"));
                    }
                } else {
                    // take name, can be ident, path::to::ident, or ident::<Ty>
                    let name_start = tt_attr_or_name.span().byte_range().start;
                    let mut tt_name_end = tt_attr_or_name;
                    while let Some(tt) = stream
                        .next_if(|tt| matches!(tt, Tt::Ident(_)) || matches!(tt, Tt::Punct(p) if [':', '<', '>'].contains(&p.as_char())))
                    {
                        tt_name_end = tt;
                    }

                    items.push(Item::Text(&code[text_start..name_start]));

                    let name_end = tt_name_end.span().byte_range().end;
                    let name = &code[name_start..name_end];
                    if name.is_empty() {
                        return Err(Error::new(items, "expected property name"));
                    }

                    if let Some(tt_punct) = stream.next() {
                        if let Tt::Punct(p) = tt_punct {
                            if p.as_char() == ';' {
                                items.push(Item::PropertyShorthand(name));
                                text_start = p.span().byte_range().end;
                                continue 'outer;
                            } else if p.as_char() == '=' {
                                // take value
                                let value_start = p.span().byte_range().end;
                                if let Some(mut tt_value_end) = stream.next() {
                                    while let Some(tt) = stream.next_if(|tt| !matches!(tt, Tt::Punct(p) if p.as_char() == ';')) {
                                        tt_value_end = tt;
                                    }
                                    text_start = tt_value_end.span().byte_range().end;
                                    items.push(Item::Property {
                                        name,
                                        value: &code[value_start..text_start],
                                    });
                                    if let Some(tt_semi) = stream.next() {
                                        debug_assert!(matches!(&tt_semi, Tt::Punct(p) if p.as_char() == ';'));
                                        text_start = tt_semi.span().byte_range().end;
                                    }
                                    continue 'outer;
                                } else {
                                    return Err(Error::new(items, "expected value"));
                                }
                            }
                        } else {
                            return Err(Error::new(items, "expected = or ;"));
                        }
                    } else {
                        // EOF shorthand
                        items.push(Item::PropertyShorthand(name));
                        text_start = name_end;
                        continue 'outer;
                    }
                }
            }

            return Err(Error::new(items, "expected attribute or property name"));
        }

        items.push(Item::Text(&code[text_start..code_span.end]));

        Ok(items)
    }

    if !reverse {
        if !PROPERTY_NAME_RGX.is_match(&code[1..code.len() - 1]) || IGNORE_RGX.is_match(code) {
            // ignore static IDENT: Ty = expr
            return Cow::Borrowed(code);
        }
        let items = match code.parse() {
            Ok(t) => match parse(code, t, 1..code.len() - 1) {
                Ok(its) => its,
                Err(e) => {
                    if SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                        // the regex is a best shot to avoid paring
                        warn!("cannot parse widget, {}", e.error);
                    }
                    return Cow::Borrowed(code);
                }
            },
            Err(_) => return Cow::Borrowed(code),
        };

        fn escape(items: &[Item], r: &mut String) {
            for item in items {
                match item {
                    // path::ident = expr;
                    // OR ident = expr0, expr1;
                    // OR ident = { field: expr, };
                    Item::Property { name, value } => {
                        r.push_str(name); // even `path::ident::<Ty>` just works here
                        r.push_str(" =");

                        static NAMED_FIELDS_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*\{\s*\w+:\s+").unwrap());
                        if NAMED_FIELDS_RGX.is_match(value) {
                            r.push_str(" __ZngFmt");
                            r.push_str(value);
                            r.push(';');
                        } else if value.trim() == "unset!" {
                            r.push_str("__unset!();");
                        } else {
                            r.push_str(" __fmt(");
                            r.push_str(value);
                            r.push_str("); /*__fmt*/");
                        }
                    }
                    Item::PropertyShorthand(name) => {
                        r.push_str(name);
                        r.push(';');
                    }
                    Item::When { expr, items } => {
                        r.push_str("if __fmt_w(");
                        // replace #{}
                        let expr = replace_expr_var(expr, false);
                        // replace #path
                        static PROPERTY_REF_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)#([\w:]+)").unwrap());
                        r.push_str(&PROPERTY_REF_RGX.replace_all(&expr, "__P_($1)"));
                        r.push_str(") { /*__fmt*/");
                        escape(items, r);
                        r.push('}');
                    }
                    Item::Text(txt) => {
                        r.push_str(txt);
                    }
                    Item::WidgetSetSelfExpr(expr) => {
                        r.push_str("let __fmt_self = ");
                        r.push_str(expr);
                        r.push_str("; /*__zng-fmt*/");
                    }
                }
            }
        }
        let mut escaped = "{".to_owned();
        escape(&items, &mut escaped);
        escaped.push('}');
        Cow::Owned(escaped)
    } else {
        let code = code
            .replace("= __ZngFmt {", "= {")
            .replace("); /*__fmt*/", ";")
            .replace("if __fmt_w(", "when ")
            .replace("__unset!()", "unset!")
            .replace("let __fmt_self = ", "")
            .replace("; /*__zng-fmt*/", ";");

        static WHEN_REV_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\) \{\s+/\*__fmt\*/").unwrap());
        let code = WHEN_REV_RGX.replace_all(&code, " {");
        let code = match replace_expr_var(&code, true) {
            Cow::Borrowed(_) => Cow::Owned(code.into_owned()),
            Cow::Owned(o) => Cow::Owned(o),
        };

        // like `.replace("= __fmt(", "= ")`, but only adds the space after = if did not wrap
        // this is important to avoid inserting a trailing space and causing a reformat for every file
        static UNNAMED_VALUE_REV_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)=\s+__fmt\(([^\r\n]?)").unwrap());
        let replaced = UNNAMED_VALUE_REV_RGX.replace_all(&code, |caps: &regex::Captures| {
            let next_char = &caps[1];
            if next_char.is_empty() {
                "=".to_owned()
            } else {
                format!("= {next_char}")
            }
        });
        let code = match replaced {
            Cow::Borrowed(_) => Cow::Owned(code.into_owned()),
            Cow::Owned(o) => Cow::Owned(o),
        };

        static PROPERTY_REF_REV_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)__P_\(([\w:]+)\)").unwrap());
        match PROPERTY_REF_REV_RGX.replace_all(&code, "#$1") {
            Cow::Borrowed(_) => Cow::Owned(code.into_owned()),
            Cow::Owned(o) => Cow::Owned(o),
        }
    }
}

// replace `#{` with `__P_!{`
fn replace_expr_var(code: &str, reverse: bool) -> Cow<'_, str> {
    static POUND_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(#)[\w\{]").unwrap());
    static POUND_REV_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"__P_!\s?").unwrap());
    if !reverse {
        POUND_RGX.replace_all(code, |caps: &regex::Captures| {
            let c = &caps[0][caps.get(1).unwrap().end() - caps.get(0).unwrap().start()..];
            if c == "{" {
                Cow::Borrowed("__P_!{")
            } else {
                Cow::Owned(caps[0].to_owned())
            }
        })
    } else {
        POUND_REV_RGX.replace_all(code, "#")
    }
}
// replace `{ pattern => expr }` with `static __zng_fmt__: T = match 0 { pattern => expr }; /*__zng-fmt*/`
fn replace_when_var(code: &str, reverse: bool) -> Cow<'_, str> {
    if !reverse {
        let stream: proc_macro2::TokenStream = match code[1..code.len() - 1].parse() {
            Ok(s) => s,
            Err(_) => return Cow::Borrowed(code),
        };
        let mut arrow_at_root = false;
        let mut stream = stream.into_iter();
        while let Some(tt) = stream.next() {
            if let proc_macro2::TokenTree::Punct(p) = tt
                && p.as_char() == '='
                && let Some(proc_macro2::TokenTree::Punct(p2)) = stream.next()
                && p2.as_char() == '>'
            {
                arrow_at_root = true;
                break;
            }
        }
        if arrow_at_root {
            Cow::Owned(format!("static __zng_fmt__: T = match 0 {code}; /*__zng-fmt*/"))
        } else {
            Cow::Borrowed(code)
        }
    } else {
        Cow::Owned(
            code.replace("static __zng_fmt__: T = match 0 {", "{")
                .replace("}; /*__zng-fmt*/", "}"),
        )
    }
}
// replace `static ref ` with `static __fmt_ref__`
fn replace_static_ref(code: &str, reverse: bool) -> Cow<'_, str> {
    if !reverse {
        if code.contains("static ref ") {
            Cow::Owned(code.replace("static ref ", "static __fmt_ref__"))
        } else {
            Cow::Borrowed(code)
        }
    } else {
        Cow::Owned(code.replace("static __fmt_ref__", "static ref "))
    }
}

// replace `{ foo: <rest> }` with `static __fmt__: T = __A_ { foo: <rest> } /*__zng-fmt*/`, if the `{` is the first token of  the line
// OR with `struct __ZngFmt__ {` if contains generics, signifying declaration
fn replace_struct_like(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*\{\s+(\w+):([^:])").unwrap());
    static RGX_GENERICS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m):\s*\w+<\w").unwrap());
    if !reverse {
        if RGX.is_match(code) {
            if RGX_GENERICS.is_match(code) {
                // probably struct declaration like
                RGX.replace_all(code, "struct __ZngFmt__ {\n$1:$2")
            } else {
                // probably struct init like
                let mut r = RGX.replace_all(code, "static __fmt__: T = __A_ {\n$1:$2").into_owned();

                const OPEN: &str = ": T = __A_ {";
                const CLOSE_MARKER: &str = "; /*__zng-fmt*/";
                let mut start = 0;
                while let Some(i) = r[start..].find(OPEN) {
                    let i = start + i + OPEN.len();
                    let mut count = 1;
                    let mut close_i = i;
                    for (ci, c) in r[i..].char_indices() {
                        match c {
                            '{' => count += 1,
                            '}' => {
                                count -= 1;
                                if count == 0 {
                                    close_i = i + ci + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    r.insert_str(close_i, CLOSE_MARKER);
                    start = close_i + CLOSE_MARKER.len();
                }
                Cow::Owned(r)
            }
        } else {
            Cow::Borrowed(code)
        }
    } else {
        Cow::Owned(
            code.replace("static __fmt__: T = __A_ {", "{")
                .replace("}; /*__zng-fmt*/", "}")
                .replace("struct __ZngFmt__ {", "{"),
        )
    }
}

// replace `pub struct Ident: Ty {` with `pub static __fmt_vis: T = T;\nimpl __fmt_Ident_C_Ty {`
// AND replace `const IDENT =` with `const IDENT: __A_ =`
fn replace_bitflags(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)struct +(\w+): +(\w+) +\{").unwrap());
    static RGX_CONST: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*const +(\w+) +=").unwrap());
    static RGX_REV: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)static __fmt_vis__: T = T;\s+impl __fmt_(\w+)__C_(\w+) \{").unwrap());
    if !reverse {
        let mut r = RGX.replace_all(code, "static __fmt_vis__: T = T;\nimpl __fmt_${1}__C_$2 {");
        if let Cow::Owned(r) = &mut r
            && let Cow::Owned(rr) = RGX_CONST.replace_all(r, "const $1: __A_ =")
        {
            *r = rr;
        }
        r
    } else {
        let code = RGX_REV.replace_all(code, "struct ${1}: $2 {");
        Cow::Owned(code.replace(": __A_ =", " ="))
    }
}

/// wrap simple list of idents
fn replace_simple_ident_list(code: &str, reverse: bool) -> Cow<'_, str> {
    static WORD_RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\w+$").unwrap());
    if !reverse {
        assert!(code.starts_with('{') && code.ends_with('}'));
        let inner = &code[1..code.len() - 1];
        if inner.contains(',') {
            let mut output = "static __fmt: T = [".to_owned();
            for ident in inner.split(',') {
                let ident = ident.trim();
                if WORD_RGX.is_match(ident) {
                    output.push_str(ident);
                    output.push_str(", ");
                } else if ident.is_empty() {
                    continue;
                } else {
                    return Cow::Borrowed(code);
                }
            }
            output.push_str("];");
            Cow::Owned(output)
        } else {
            let mut output = "static __fmt_s: T = [".to_owned();
            let mut any = false;
            for ident in inner.split(' ') {
                let ident = ident.trim();
                if WORD_RGX.is_match(ident) {
                    any = true;
                    output.push_str(ident);
                    output.push_str(", ");
                } else if ident.is_empty() {
                    continue;
                } else {
                    return Cow::Borrowed(code);
                }
            }
            if any {
                output.push_str("];");
                Cow::Owned(output)
            } else {
                Cow::Borrowed(code)
            }
        }
    } else {
        let code = if code.trim_end().contains('\n') {
            if code.contains("static __fmt: T = [") {
                code.replace("static __fmt: T = [", "{").replace("];", "}")
            } else {
                assert!(code.contains("static __fmt_s: T = ["));
                code.replace("static __fmt_s: T = [", "{").replace("];", "}").replace(',', "")
            }
        } else if code.contains("static __fmt: T = [") {
            code.replace("static __fmt: T = [", "{ ").replace("];", " }")
        } else {
            assert!(code.contains("static __fmt_s: T = ["));
            code.replace("static __fmt_s: T = [", "{ ").replace("];", " }").replace(',', "")
        };
        Cow::Owned(code)
    }
}

// replace `ident(args);` with `fn __fmt__ident(args);
fn replace_widget_impl(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)([:\w]+\((?:\w+: .+)\));").unwrap());
    if !reverse {
        RGX.replace_all(code, |caps: &regex::Captures| {
            // "fn __fmt__${1};" with colon escape
            let (a, b) = caps[1].split_once('(').unwrap();
            format!("fn __fmt__{}({b};", a.replace("::", "__C__"))
        })
    } else {
        Cow::Owned(code.replace("fn __fmt__", "").replace("__C__", "::"))
    }
}

/// rustfmt does not provide a crate and does not implement a server. It only operates in one-shot
/// calls and it is slow.
///
/// This service is a quick workaround that, it abuses the async state machine generation to inject
/// a batching feature in the middle of the recursive custom fmt logic. **It does not implement wakers**,
/// just keep polling to execute.
#[derive(Clone)]
struct FmtFragServer {
    data: Arc<Mutex<FmtFragServerData>>,
    edition: String,
}
struct FmtFragServerData {
    // [request, response]
    requests: HashMap<String, FmtFragRequest>,
}
struct FmtFragRequest {
    pending: bool,
    response: Arc<Mutex<String>>,
}

impl FmtFragRequest {
    fn new() -> Self {
        Self {
            pending: true,
            response: Arc::new(Mutex::new(String::new())),
        }
    }
}
impl FmtFragServer {
    pub fn spawn(edition: String) -> Self {
        let s = Self {
            data: Arc::new(Mutex::new(FmtFragServerData { requests: HashMap::new() })),
            edition,
        };
        let s_read = s.clone();
        std::thread::Builder::new()
            .name("rustfmt-frag-server".to_owned())
            .spawn(move || {
                loop {
                    s_read.poll();
                }
            })
            .unwrap();
        s
    }

    #[track_caller]
    pub fn format(&self, code: String) -> impl Future<Output = Option<String>> {
        let res = self
            .data
            .lock()
            .requests
            .entry(code)
            .or_insert_with(FmtFragRequest::new)
            .response
            .clone();
        std::future::poll_fn(move |_cx| {
            let res = res.lock();
            match res.as_str() {
                "" => Poll::Pending,
                "#rustfmt-error#" => Poll::Ready(None),
                _ => Poll::Ready(Some(res.clone())),
            }
        })
    }

    fn poll(&self) {
        let requests: Vec<_> = self
            .data
            .lock()
            .requests
            .iter_mut()
            .filter_map(|(k, v)| {
                if v.pending {
                    v.pending = false;
                    Some((k.clone(), v.response.clone()))
                } else {
                    None
                }
            })
            .collect();
        if requests.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
            return;
        }

        let edition = self.edition.clone();
        blocking::unblock(move || {
            if requests.len() == 1 {
                let (request, response) = requests.into_iter().next().unwrap();
                let r = match rustfmt_stdin(&Self::wrap_code_for_fmt(request.clone()), &edition) {
                    Some(f) => Self::unwrap_formatted_code(f),
                    None => "#rustfmt-error#".to_owned(),
                };
                *response.lock() = r;
            } else {
                match rustfmt_stdin(&Self::wrap_batch_for_fmt(requests.iter().map(|(k, _)| k.as_str())), &edition) {
                    Some(r) => {
                        let r = Self::unwrap_batch_for_fmt(r, requests.len());
                        for ((_, response), r) in requests.into_iter().zip(r) {
                            *response.lock() = r;
                        }
                    }
                    None => {
                        for (request, response) in requests {
                            let r = match rustfmt_stdin(&Self::wrap_code_for_fmt(request), &edition) {
                                Some(f) => Self::unwrap_formatted_code(f),
                                None => "#rustfmt-error#".to_owned(),
                            };
                            *response.lock() = r;
                        }
                    }
                }
            }
        })
        .detach();
    }

    const PREFIX: &str = "fn __frag__() ";
    fn wrap_code_for_fmt(code: String) -> String {
        if code.starts_with("{") {
            format!("{}{code}", Self::PREFIX)
        } else {
            code
        }
    }
    fn unwrap_formatted_code(fmt: String) -> String {
        match fmt.strip_prefix(Self::PREFIX) {
            Some(s) => s.to_owned(),
            None => fmt,
        }
    }

    fn wrap_batch_for_fmt<'a>(requests: impl Iterator<Item = &'a str>) -> String {
        let mut s = String::new();
        for code in requests {
            s.push_str("mod __batch__ {\n#![__zng_fmt_batch_tabs]\n");
            if code.starts_with("{") {
                s.push_str(Self::PREFIX);
            }
            s.push_str(code);
            s.push_str("\n}");
        }
        s
    }
    fn unwrap_batch_for_fmt(fmt: String, count: usize) -> Vec<String> {
        let mut item = String::new();
        let mut r = vec![];
        let mut lines = fmt.lines();
        let mut strip_tabs = String::new();
        while let Some(line) = lines.next() {
            if line.starts_with("mod __batch__") {
                if !item.is_empty() {
                    let it = item.trim();
                    if let Some(it) = it.strip_prefix(Self::PREFIX) {
                        r.push(it.to_owned());
                    } else {
                        r.push(it.to_owned());
                    }
                    item.clear();
                }

                let tabs_line = lines.next().unwrap();
                assert!(tabs_line.contains("#![__zng_fmt_batch_tabs]"));
                let count = tabs_line.len() - tabs_line.trim_start().len();
                strip_tabs.clear();
                for _ in 0..count {
                    strip_tabs.push(' ');
                }
            } else if line.is_empty() {
                item.push('\n');
            } else if let Some(line) = line.strip_prefix(&strip_tabs) {
                item.push_str(line);
                item.push('\n');
            } else if line != "}" {
                item.push_str(line);
                item.push('\n');
            }
        }
        if !item.is_empty() {
            let it = item.trim();
            if let Some(it) = it.strip_prefix(Self::PREFIX) {
                r.push(it.to_owned());
            } else {
                r.push(it.to_owned());
            }
        }
        assert_eq!(r.len(), count);
        r
    }
}

static SHOW_RUSTFMT_ERRORS: AtomicBool = AtomicBool::new(false);
fn rustfmt_stdin(code: &str, edition: &str) -> Option<String> {
    let mut rustfmt = std::process::Command::new("rustfmt");
    if !SHOW_RUSTFMT_ERRORS.load(Relaxed) {
        rustfmt.stderr(Stdio::null());
    }
    let mut s = rustfmt
        .arg("--edition")
        .arg(edition)
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

fn rustfmt_files(files: &[PathBuf], edition: &str, check: bool) {
    let mut rustfmt = std::process::Command::new("rustfmt");
    if !SHOW_RUSTFMT_ERRORS.load(Relaxed) {
        rustfmt.stderr(Stdio::null());
    }
    rustfmt.args(["--config", "skip_children=true"]);
    rustfmt.arg("--edition").arg(edition);
    if check {
        rustfmt.arg("--check");
    }
    let mut any = false;
    for file in files {
        if let Some(ext) = file.extension()
            && ext == "rs"
        {
            rustfmt.arg(file);
            any = true;
        }
    }
    if !any {
        return;
    }

    match rustfmt.status() {
        Ok(s) => {
            if !s.success() && SHOW_RUSTFMT_ERRORS.load(Relaxed) {
                error!("rustfmt error {s}");
            }
        }
        Err(e) => error!("{e}"),
    }
}

#[derive(Default)]
struct FmtHistory {
    /// args hash and timestamp
    entries: Vec<(String, u128)>,
}
impl FmtHistory {
    /// insert is called before formatting, but we need to actually save the
    /// timestamp after formatting, this value marks an inserted entry not saved yet.
    const TIMESTAMP_ON_SAVE: u128 = u128::MAX;

    const MAX_ENTRIES: usize = 30;

    pub fn load() -> io::Result<Self> {
        let now = Self::time(SystemTime::now());

        match std::fs::File::open(Self::path()?) {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                let mut out = Self { entries: vec![] };
                for line in reader.lines().take(Self::MAX_ENTRIES) {
                    if let Some((key, ts)) = line?.split_once(' ') {
                        let t: u128 = ts.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
                        if t > now {
                            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid timestamp"));
                        }
                        out.entries.push((key.to_owned(), t));
                    }
                }
                Ok(out)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self { entries: vec![] }),
            Err(e) => Err(e),
        }
    }

    /// Returns the previous timestamp for the same args or 0.
    pub fn insert(&mut self, args: &FmtArgs) -> u128 {
        let mut args_key = sha2::Sha256::new();
        if let Some(f) = &args.files {
            args_key.update(f.as_bytes());
        }
        if let Some(f) = &args.manifest_path {
            args_key.update(f.as_bytes());
        }
        args_key.update(args.edition.as_bytes());
        let rustfmt_version = std::process::Command::new("rustfmt")
            .arg("--version")
            .output()
            .unwrap_or_else(|e| fatal!("{e}"));
        if !rustfmt_version.status.success() {
            fatal!("rustfmt error {}", rustfmt_version.status);
        }
        let rustfmt_version = String::from_utf8_lossy(&rustfmt_version.stdout);
        args_key.update(rustfmt_version.as_bytes());
        let args_key = format!("{FMT_VERSION}:{:x}", args_key.finalize());

        for (key, t) in self.entries.iter_mut() {
            if key == &args_key {
                let prev_t = *t;
                assert_ne!(prev_t, Self::TIMESTAMP_ON_SAVE, "inserted called twice");
                *t = Self::TIMESTAMP_ON_SAVE;
                return prev_t;
            }
        }
        self.entries.push((args_key, Self::TIMESTAMP_ON_SAVE));
        if self.entries.len() > Self::MAX_ENTRIES {
            self.entries.remove(0);
        }
        0
    }

    pub fn save(&mut self) -> io::Result<()> {
        let now = Self::time(SystemTime::now());
        for (_, t) in self.entries.iter_mut() {
            if *t == Self::TIMESTAMP_ON_SAVE {
                *t = now;
            }
        }

        let mut file = std::fs::File::create(Self::path()?)?;
        for (key, t) in self.entries.iter() {
            writeln!(&mut file, "{key} {t}")?;
        }

        Ok(())
    }

    /// Convert to history time representation.
    pub fn time(time: SystemTime) -> u128 {
        time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_micros()
    }

    fn path() -> io::Result<PathBuf> {
        let root_dir = workspace_root()?;
        let target_dir = root_dir.join("target");
        let _ = std::fs::create_dir(&target_dir);
        Ok(target_dir.join(".cargo-zng-fmt-history"))
    }
}
fn workspace_root() -> io::Result<PathBuf> {
    let output = std::process::Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "workspace root not found"));
    }
    let root_dir = Path::new(std::str::from_utf8(&output.stdout).unwrap().trim()).parent().unwrap();
    Ok(root_dir.to_owned())
}

/// proc_macro2 types are not send, even when compiled outside of a proc-macro crate
/// this mod converts the token tree to a minimal Send model that only retains the info needed
/// to implement the custom formatting
mod pm2_send {
    use std::{ops, str::FromStr};

    pub use proc_macro2::Delimiter;

    #[derive(Clone, Debug)]
    pub struct TokenStream(Vec<TokenTree>);
    impl From<proc_macro2::TokenStream> for TokenStream {
        fn from(value: proc_macro2::TokenStream) -> Self {
            Self(value.into_iter().map(Into::into).collect())
        }
    }
    impl FromStr for TokenStream {
        type Err = <proc_macro2::TokenStream as FromStr>::Err;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            proc_macro2::TokenStream::from_str(s).map(Into::into)
        }
    }
    impl IntoIterator for TokenStream {
        type Item = TokenTree;

        type IntoIter = std::vec::IntoIter<Self::Item>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    #[derive(Clone, Debug)]
    pub enum TokenTree {
        Group(Group),
        Ident(Ident),
        Punct(Punct),
        Other(Span),
    }
    impl From<proc_macro2::TokenTree> for TokenTree {
        fn from(value: proc_macro2::TokenTree) -> Self {
            match value {
                proc_macro2::TokenTree::Group(group) => Self::Group(group.into()),
                proc_macro2::TokenTree::Ident(ident) => Self::Ident(ident.into()),
                proc_macro2::TokenTree::Punct(punct) => Self::Punct(punct.into()),
                proc_macro2::TokenTree::Literal(literal) => Self::Other(literal.span().into()),
            }
        }
    }
    impl TokenTree {
        pub fn span(&self) -> Span {
            match self {
                TokenTree::Group(group) => group.span.clone(),
                TokenTree::Ident(ident) => ident.span.clone(),
                TokenTree::Punct(punct) => punct.span.clone(),
                TokenTree::Other(span) => span.clone(),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct Group {
        delimiter: Delimiter,
        span: Span,
        stream: TokenStream,
    }
    impl From<proc_macro2::Group> for Group {
        fn from(value: proc_macro2::Group) -> Self {
            Self {
                delimiter: value.delimiter(),
                span: value.span().into(),
                stream: value.stream().into(),
            }
        }
    }
    impl Group {
        pub fn delimiter(&self) -> Delimiter {
            self.delimiter
        }

        pub fn span(&self) -> Span {
            self.span.clone()
        }

        pub fn stream(&self) -> TokenStream {
            self.stream.clone()
        }
    }

    #[derive(Clone, Debug)]
    pub struct Ident {
        span: Span,
        s: String,
    }
    impl From<proc_macro2::Ident> for Ident {
        fn from(value: proc_macro2::Ident) -> Self {
            Self {
                span: value.span().into(),
                s: value.to_string(),
            }
        }
    }
    impl<'a> PartialEq<&'a str> for Ident {
        fn eq(&self, other: &&'a str) -> bool {
            self.s == *other
        }
    }

    #[derive(Clone, Debug)]
    pub struct Punct {
        span: Span,
        c: char,
    }
    impl From<proc_macro2::Punct> for Punct {
        fn from(value: proc_macro2::Punct) -> Self {
            Self {
                span: value.span().into(),
                c: value.as_char(),
            }
        }
    }
    impl Punct {
        pub fn as_char(&self) -> char {
            self.c
        }
    }

    #[derive(Clone, Debug)]
    pub struct Span {
        byte_range: ops::Range<usize>,
    }
    impl From<proc_macro2::Span> for Span {
        fn from(value: proc_macro2::Span) -> Self {
            Self {
                byte_range: value.byte_range(),
            }
        }
    }
    impl Span {
        pub fn byte_range(&self) -> ops::Range<usize> {
            self.byte_range.clone()
        }
    }
}
