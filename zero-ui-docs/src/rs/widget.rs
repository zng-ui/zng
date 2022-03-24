use std::path::Path;

use nipper::Document;

/// Edit Widget module pages and module lists.
pub fn tranform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/index.html", |file, html| {
        let doc = Document::from(&html);

        let edited = transform_widget_mod_page(&doc) | transform_mod_list(&doc);

        if edited {
            std::fs::write(file, doc.html().as_bytes()).unwrap();
        }
    });
    transform_sidebars(docs_root);
}

/// Edit module page for widget mod.
fn transform_widget_mod_page(_doc: &Document) -> bool {
    todo!()
}

/// Edit mod lists in module page, creates a new "Widgets" section.
fn transform_mod_list(_doc: &Document) -> bool {
    todo!()
}

/// Edit sidebar lists, creates a new "Widgets" section.
fn transform_sidebars(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/sidebar-items.js", |file, js| {
        if !js.starts_with("initSidebarItems(") {
            return;
        }

        let edit = js.contains("`widget` ");

        if edit {
            // TODO

            std::fs::write(file, js.as_bytes()).unwrap();
        }
    });
}
