use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use lol_html::*;
use rayon::prelude::*;

/// Glob in `root`.
pub fn glob(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let search = format!("{}/{pattern}", root.display());
    glob::glob(&search).unwrap().into_iter().filter_map(|r| r.ok()).collect()
}

/// Glob files, read then to `String` then call `for_each` in parallel.
pub fn glob_par_each(root: &Path, pattern: &str, for_each: impl Fn(PathBuf, String) + Sync + Send) {
    glob(root, pattern)
        .into_par_iter()
        .for_each(move |path| match fs::read_to_string(&path) {
            Ok(s) => for_each(path, s),
            Err(e) => eprintln!("{e}"),
        })
}

/// Apply rewrite matchers to `html`, discards rewrite requests.
pub fn analyze_html(html: &str, element_content_handlers: Vec<(Cow<Selector>, ElementContentHandlers)>) {
    let mut r = lol_html::HtmlRewriter::new(
        RewriteStrSettings {
            element_content_handlers,
            ..Default::default()
        }
        .into(),
        |_: &[u8]| {},
    );
    r.write(html.as_bytes()).unwrap();
    r.end().unwrap();
}

/// Rewrite `html`, returns `Some(_)` if any rewrite was applied.
pub fn rewrite_html(html: &str, element_content_handlers: Vec<(Cow<Selector>, ElementContentHandlers)>) -> Option<String> {
    let r = lol_html::rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers,
            ..Default::default()
        },
    )
    .ok()?;
    if r != html {
        Some(r)
    } else {
        None
    }
}
