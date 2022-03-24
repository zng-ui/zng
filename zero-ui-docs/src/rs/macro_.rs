use nipper::*;
use std::path::Path;

/// Remove macro items from the crates front pages if their are tagged with `<span data-del-macro-root></span>`.
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "*/index.html", |file, html| {
        let doc = Document::from(&html);

        if !doc.select("h1").iter().any(|h1| h1.text().starts_with("Crate ")) {
            // not crate front page.
            return;
        }

        let mut edited = false;

        // remove tagged macro entries
        let tagged_entries = doc.select("span[data-del-macro-root]").parent().parent();
        if tagged_entries.exists() {
            edited = true;
            tagged_entries.parent().parent().remove();
        }

        // remove empty macros section
        let mut title = doc.select("#macros");
        let mut table = title.next_sibling();
        if !table.select("a").exists() {
            edited = true;

            table.remove();
            title.remove();

            // remove from sidebar.
            doc.select("a[href='#macros']").remove();
        }

        if edited {
            std::fs::write(file, doc.html().as_bytes()).unwrap();
        }
    });
}
