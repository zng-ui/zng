use std::{
    fs,
    path::{Path, PathBuf},
};

use nipper::{Node, Selection};
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

/// Extensions for [`Node`].
pub trait NodeExt {
    /// Parse `html` element and append after `self`.
    fn append_next_sibling_html(&self, html: &str);
}
impl<'a> NodeExt for Node<'a> {
    fn append_next_sibling_html(&self, html: &str) {
        let parent = self.parent().expect("cannot append sibling to root");

        let mut parent = Selection::from(parent.clone());
        parent.append_html(format!("<div id='append_next_sibling_html-temp'>{html}</div>"));

        let temp = parent.select("#append_next_sibling_html-temp").nodes()[0].clone();
        let new = temp.first_child().unwrap();
        
        temp.remove_from_parent();

        let next = self.next_sibling().unwrap();
        next.append_prev_sibling(&new.id);

    }
}
