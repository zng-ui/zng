#![cfg(feature = "post")]

use std::{env, path::PathBuf};

fn main() {
    let mut args = env::args();
    let _ = args.next();
    let docs_root = args
        .next()
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .expect("no docs root");

    println!("post-processing docs at `{}`", docs_root.display());

    zero_ui_docs::transform(docs_root);
}
