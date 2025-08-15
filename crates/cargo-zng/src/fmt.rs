use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, atomic::AtomicBool},
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
    let action = if args.check { "checking" } else { "formatting" };

    if args.rustfmt_errors {
        SHOW_RUSTFMT_ERRORS.store(true, std::sync::atomic::Ordering::Relaxed);
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
            let mut formatted = Box::pin(fmt_code(&code, stream, &fmt_server));
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

    let mut custom_fmt_files = vec![];
    if let Some(glob) = &args.files {
        if args.manifest_path.is_some() || args.package.is_some() {
            fatal!("--files must not be set when crate is set");
        }

        for file in glob::glob(glob).unwrap_or_else(|e| fatal!("{e}")) {
            let file = file.unwrap_or_else(|e| fatal!("{e}"));
            custom_fmt_files.push(file);
        }
    } else {
        if let Some(pkg) = &args.package {
            if args.manifest_path.is_some() {
                fatal!("expected only one of --package, --manifest-path");
            }
            match util::manifest_path_from_package(pkg) {
                Some(m) => args.manifest_path = Some(m),
                None => fatal!("package `{pkg}` not found in workspace"),
            }
        }
        if let Some(path) = &args.manifest_path {
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
            for path in util::workspace_manifest_paths() {
                let files = path.parent().unwrap().join("**/*.rs").display().to_string().replace('\\', "/");
                for file in glob::glob(&files).unwrap_or_else(|e| fatal!("{e}")) {
                    let file = file.unwrap_or_else(|e| fatal!("{e}"));
                    custom_fmt_files.push(file);
                }
            }
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
    let mut custom_fmt_files: Vec<_> = custom_fmt_files
        .into_par_iter()
        .filter_map(|p| {
            let modified = std::fs::metadata(&p)
                .unwrap_or_else(|e| fatal!("{e}"))
                .modified()
                .unwrap_or_else(|e| fatal!("{e}"));
            let modified = FmtHistory::time(modified);
            if modified > cutout_time { Some((p, modified)) } else { None }
        })
        .collect();

    // latest modified first
    custom_fmt_files.sort_by(|a, b| b.1.cmp(&a.1));
    let custom_fmt_files: Vec<_> = custom_fmt_files.into_iter().map(|(p, _)| p).collect();

    let fmt_server = FmtFragServer::spawn(args.edition.clone());

    custom_fmt_files.par_chunks(64).for_each(|c| {
        // apply normal format first
        rustfmt_files(c, &args.edition, args.check);
    });

    // apply custom format
    let check = args.check;
    let fmt_server2 = fmt_server.clone();
    let mut futs: Vec<_> = custom_fmt_files
        .par_iter()
        .map(move |file| {
            let fmt_server = fmt_server2.clone();
            Some(Box::pin(async move {
                custom_fmt(file.clone(), check, fmt_server)
                    .await
                    .unwrap_or_else(|e| fatal!("error {action} `{}`, {e}", file.display()))
            }))
        })
        .collect();

    loop {
        std::thread::sleep(Duration::from_millis(25));
        futs.par_iter_mut().for_each(|f| {
            match f
                .as_mut()
                .unwrap()
                .as_mut()
                .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            {
                Poll::Ready(()) => *f = None,
                Poll::Pending => {}
            }
        });
        futs.retain(|t| t.is_some());
        if futs.is_empty() {
            break;
        }
    }

    if let Err(e) = history.save() {
        warn!("cannot save fmt history, {e}")
    }
}

async fn custom_fmt(rs_file: PathBuf, check: bool, fmt: FmtFragServer) -> io::Result<()> {
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

    let file_stream = file_code
        .parse()
        .unwrap_or_else(|e| fatal!("cannot parse `{}`, {e}", rs_file.display()));

    formatted_code.push_str(&fmt_code(file_code, file_stream, &fmt).await);

    if formatted_code != file {
        if check {
            fatal!("extended format does not match in file `{}`", rs_file.display());
        }
        fs::write(rs_file, formatted_code)?;
    }

    Ok(())
}

async fn fmt_code(code: &str, stream: pm2_send::TokenStream, fmt: &FmtFragServer) -> String {
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
                        if let Some(stable) = try_fmt_macro(base_indent, &formatted, fmt).await
                            && formatted == stable
                        {
                            // change is sable
                            let already_fmt = &code[last_already_fmt_start..group_bytes.start];
                            formatted_code.push_str(already_fmt);
                            formatted_code.push_str(&formatted);
                            last_already_fmt_start = group_bytes.end;
                        }
                    }
                } else if !tail2.is_empty()
                    && matches!(g.delimiter(), pm2_send::Delimiter::Bracket)
                    && matches!(&tail2[0], pm2_send::TokenTree::Punct(p) if p.as_char() == '#')
                {
                    // #[..]
                    let mut attr = g.stream().into_iter();
                    let attr = [attr.next(), attr.next(), attr.next(), attr.next(), attr.next()];
                    if let [
                        Some(pm2_send::TokenTree::Ident(i0)),
                        Some(pm2_send::TokenTree::Punct(p0)),
                        Some(pm2_send::TokenTree::Punct(p1)),
                        Some(pm2_send::TokenTree::Ident(i1)),
                        None,
                    ] = attr
                        && i0 == "rustfmt"
                        && p0.as_char() == ':'
                        && p1.as_char() == ':'
                        && i1 == "skip"
                    {
                        // #[rustfmt::skip]
                        skip_next_group = true;
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

    if formatted_code != code {
        // custom format can cause normal format to change
        // example: ui_vec![Wgt!{<many properties>}, Wgt!{<same>}]
        //   Wgt! gets custom formatted onto multiple lines, that causes ui_vec![\n by normal format.
        formatted_code = fmt.format(formatted_code.clone()).await.unwrap_or(formatted_code);
    }

    formatted_code
}

async fn try_fmt_macro(base_indent: usize, group_code: &str, fmt: &FmtFragServer) -> Option<String> {
    let mut replaced_code = Cow::Borrowed(group_code);

    let mut is_lazy_static = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_static_ref(group_code, false);
        is_lazy_static = matches!(&replaced_code, Cow::Owned(_));
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

    let mut is_widget = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_widget_when(group_code, false);
        is_widget = matches!(&replaced_code, Cow::Owned(_));

        let tmp = replace_widget_prop(&replaced_code, false);
        if let Cow::Owned(tmp) = tmp {
            is_widget = true;
            replaced_code = Cow::Owned(tmp);
        }
    }

    let mut is_expr_var = false;
    if matches!(&replaced_code, Cow::Borrowed(_)) {
        replaced_code = replace_expr_var(group_code, false);
        is_expr_var = matches!(&replaced_code, Cow::Owned(_));
    }

    let code = fmt.format(replaced_code.into_owned()).await?;

    let code = if is_event_args {
        replace_event_args(&code, true)
    } else if is_widget {
        let code = replace_widget_when(&code, true);
        let code = replace_widget_prop(&code, true).into_owned();
        Cow::Owned(code)
    } else if is_expr_var {
        replace_expr_var(&code, true)
    } else if is_lazy_static {
        replace_static_ref(&code, true)
    } else if is_command {
        replace_command(&code, true)
    } else {
        Cow::Owned(code)
    };

    let code_stream: pm2_send::TokenStream = code.parse().unwrap_or_else(|e| panic!("{e}\ncode:\n{code}"));
    let code_stream = {
        let code_tt = code_stream.into_iter().next().unwrap();
        match code_tt {
            pm2_send::TokenTree::Group(g) => g.stream(),
            _ => unreachable!(),
        }
    };
    let code = Box::pin(fmt_code(&code, code_stream, fmt)).await;

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
// replace `static IDENT = {` with `static IDENT: __zng_fmt__ = {`
// AND replace `l10n!: ` with `l10n__zng_fmt:`
fn replace_command(code: &str, reverse: bool) -> Cow<'_, str> {
    static RGX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)(?m)static +(\w+) ?= ?\{").unwrap());
    if !reverse {
        let mut cmd = RGX.replace_all(code, "static $1: __zng_fmt__ = __A_ {");
        if let Cow::Owned(cmd) = &mut cmd {
            *cmd = cmd.replace("l10n!:", "l10n__zng_fmt:");
        }
        cmd
    } else {
        Cow::Owned(code.replace(": __zng_fmt__ = __A_ {", " = {").replace("l10n__zng_fmt:", "l10n!:"))
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
                let stream: pm2_send::TokenStream = match code.parse() {
                    Ok(s) => s,
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        panic!("{_e}\ncode:\n{code}");
                        #[cfg(not(debug_assertions))]
                        return false;
                    }
                };
                for tt in stream {
                    if let pm2_send::TokenTree::Punct(p) = tt
                        && p.as_char() == ','
                    {
                        return true;
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
// replace `static ref ` with `static __zng_fmt_ref__`
fn replace_static_ref(code: &str, reverse: bool) -> Cow<'_, str> {
    if !reverse {
        if code.contains("static ref ") {
            Cow::Owned(code.replace("static ref ", "static __zng_fmt_ref__"))
        } else {
            Cow::Borrowed(code)
        }
    } else {
        Cow::Owned(code.replace("static __zng_fmt_ref__", "static ref "))
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
                let r = match rustfmt_stdin(&Self::wrap_code_for_fmt(request), &edition) {
                    Some(f) => Self::unwrap_formatted_code(f),
                    None => "#rustfmt-error#".to_owned(),
                };
                *response.lock() = r;
            } else {
                match rustfmt_stdin(&Self::wrap_batch_for_fmt(requests.iter().map(|(k, _)| k.as_str())), &edition) {
                    Some(r) => {
                        println!("!!: ok for {}", requests.len());
                        let r = Self::unwrap_batch_for_fmt(r, requests.len());
                        for ((_, response), r) in requests.into_iter().zip(r) {
                            *response.lock() = r;
                        }
                    }
                    None => {
                        println!("!!: error retries {}", requests.len());
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
            s.push_str("mod __batch__ {\n use __batch_tabs;\n");
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
    let mut fmt = std::process::Command::new("rustfmt");
    if !SHOW_RUSTFMT_ERRORS.load(std::sync::atomic::Ordering::Relaxed) {
        fmt.stderr(Stdio::null());
    }
    let mut s = fmt
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
    rustfmt.args(["--config", "skip_children=true"]);
    rustfmt.arg("--edition").arg(edition);
    if check {
        rustfmt.arg("--check");
    }
    rustfmt.args(files);

    match rustfmt.output() {
        Ok(s) => {
            if !s.status.success() {
                fatal!("rustfmt error {}", s.status)
            }
        }
        Err(e) => fatal!("{e}"),
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
        let target_dir = root_dir.join("target");
        let _ = std::fs::create_dir(&target_dir);
        Ok(target_dir.join(".cargo-zng-fmt-history"))
    }
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
