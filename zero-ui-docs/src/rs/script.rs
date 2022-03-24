use std::path::Path;

use nipper::Document;

/// Remove scripts inserted by [`crate::html_in_header`].
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/*.html", |file, html| {
        let doc = Document::from(&html);

        let mut scripts = doc.select("script[data-zero-ui]");

        let edit = scripts.exists();
        if edit {
            scripts.remove();

            std::fs::write(file, doc.html().as_bytes()).unwrap();
        }
    });
}
