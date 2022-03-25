use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use lol_html::*;
use rayon::prelude::*;
use regex::Regex;

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
    .unwrap();
    if r != html {
        Some(r)
    } else {
        None
    }
}

/// Extension methods for [`Regex`].
pub trait RegexExt {
    /// Split including the delimiter as a prefix.
    fn split_keep<'r, 's>(&'r self, text: &'s str) -> SplitKeep<'r, 's>;
}
impl RegexExt for Regex {
    fn split_keep<'r, 's>(&'r self, text: &'s str) -> SplitKeep<'r, 's> {
        SplitKeep {
            text,
            find_iter: self.find_iter(text),
            start: 0,
        }
    }
}

/// See [`RegexExt::split_keep`].
pub struct SplitKeep<'r, 's> {
    text: &'s str,
    find_iter: regex::Matches<'r, 's>,
    start: usize,
}
impl<'r, 's> Iterator for SplitKeep<'r, 's> {
    type Item = &'s str;

    fn next(&mut self) -> Option<&'s str> {
        match self.find_iter.next() {
            Some(m) => {
                let s = &self.text[self.start..m.start()];
                if s.is_empty() {
                    self.next()
                } else {
                    self.start = m.start();
                    Some(s)
                }
            },
            None => if self.start == self.text.len() {
                None
            } else {
                let s = &self.text[self.start..];
                self.start = self.text.len();
                Some(s)
            },
        }
    }
}