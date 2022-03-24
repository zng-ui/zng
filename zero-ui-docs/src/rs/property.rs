use std::path::Path;

/// Edit property function pages and function lists.
pub fn transform(docs_root: &Path) {
    transform_property_fn_pages(docs_root);
    transform_fn_lists(docs_root);
    transform_sidebars(docs_root);
}

/// Edit function pages for property functions.
fn transform_property_fn_pages(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/fn.*.html", |file, html| {
        let html = super::util::rewrite_html(&html, vec![]);

        if let Some(html) = html {
            std::fs::write(file, html.as_bytes()).unwrap();
        }
    });
}

/// Edit function lists in module pages, creates a new "Properties" section.
fn transform_fn_lists(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/index.html", |_file, _html| {
        // TODO
    });
}

/// Edit sidebar lists, creates a new "Properties" section.
fn transform_sidebars(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/sidebar-items.js", |file, js| {
        if !js.starts_with("initSidebarItems(") {
            return;
        }

        let edit = js.contains("`property` ");

        if edit {
            // TODO

            std::fs::write(file, js.as_bytes()).unwrap();
        }
    });
}
