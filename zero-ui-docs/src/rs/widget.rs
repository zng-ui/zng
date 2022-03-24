use std::path::Path;

/// Edit Widget module pages and module lists.
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/index.html", |file, html| {
        let html = super::util::rewrite_html(&html, vec![]);

        if let Some(html) = html {
            std::fs::write(file, html.as_bytes()).unwrap();
        }
    });
    transform_sidebars(docs_root);
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
