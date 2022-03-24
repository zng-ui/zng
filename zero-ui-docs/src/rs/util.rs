use std::{
    fs,
    path::{Path, PathBuf},
};

use nipper::{Document, Node, NodeId};
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

/// Extensions for [`Document`].
pub trait DocumentExt {
    /// Create a new node from HTML.
    fn create_node<T: AsRef<str>>(&self, html: T) -> Node;
}
impl DocumentExt for Document {
    fn create_node<T: AsRef<str>>(&self, html: T) -> Node {
        if let Some(mut s) = self.select("#zero-ui-post").iter().next() {
            s.set_html(html.as_ref());
            return s.nodes()[0].clone();
        }

        self.select("body").append_html("<div id='zero-ui-post' style='display: none;'></div>");

        self.create_node(html)
    }
}

/// Extensions for [`Node`].
pub trait NodeExt {
    /// Move node to be the next sibling of `self`.
    fn append_next_sibling(&self, id: &NodeId);
}
impl<'a> NodeExt for Node<'a> {
    fn append_next_sibling(&self, id: &NodeId) {
        if let Some(next) = self.next_sibling() {
            next.append_prev_sibling(id);
        } else if let Some(parent) = self.parent() {
            parent.append_child(id);
        } else {
            panic!("cannot append sibling to root");
        }
    }
}