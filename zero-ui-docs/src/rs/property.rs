use std::path::Path;

use nipper::{Document, Selection};

use crate::rs::util::{DocumentExt, NodeExt};

/// Edit property function pages and function lists.
pub fn transform(docs_root: &Path) {

}

/// Edit function pages for property functions.
fn transform_property_fn_pages(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/fn.*.html", |file, html| {
        let doc = Document::from(&html);

        let mut edited = false;

        doc.select("code").iter().any(|mut c| {
            let is = c.text().as_ref() == "property";
            if is {
                c.remove();
                edited = true;
            }
            is
        });
        if !edited {
            return;
        }

        for mut h1 in doc.select("h1").iter() {
            if h1.text().starts_with("Function ") {
                let html = h1.html().as_ref().replace(">Function <", ">Property <");
                h1.set_html(html);
            }
        }

        let decl_code = doc.select("pre.rust.fn").first();
        assert!(decl_code.exists());

        let as_fn_title = doc.select("#as-function");
        let capture_only = !as_fn_title.exists();

        if !capture_only {
            let node = doc.create_node(decl_code.html());
            as_fn_title.get(0).unwrap().append_next_sibling(&node.id);
        }

        edit_prop_decl(capture_only, &decl_code);

        if edited {
            std::fs::write(file, doc.html().as_bytes()).unwrap();
        }
    });
}
fn edit_prop_decl(_capture_only: bool, pre: &Selection) {
    // remove where section
    let mut where_ = pre.select("span.where");
    where_.remove();



    // TODO
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