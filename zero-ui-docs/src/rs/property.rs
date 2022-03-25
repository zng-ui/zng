use std::{cell::Cell, collections::HashSet, path::Path};

use lol_html::html_content::ContentType;
use regex::Regex;

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
    super::util::glob_par_each(docs_root, "**/index.html", |file, html| {
        let functions = Cell::new(false);
        let fn_entry = Cell::new(0);
        let mut tagged_fns = HashSet::new();

        super::util::analyze_html(
            &html,
            vec![
                lol_html::element!("h2", |h2| {
                    if let Some(id) = h2.get_attribute("id") {
                        functions.set(id == "functions");
                    }
                    Ok(())
                }),
                lol_html::element!("div.item-row", |_| {
                    if functions.get() {
                        fn_entry.set(fn_entry.get() + 1);
                    }
                    Ok(())
                }),
                lol_html::text!("div.item-row code", |t| {
                    if functions.get() && t.as_str() == "property" {
                        tagged_fns.insert(fn_entry.get());
                    }
                    Ok(())
                }),
            ],
        );

        if tagged_fns.is_empty() {
            return;
        }

        let move_all_fns = tagged_fns.len() == fn_entry.get();

        functions.set(false);
        let rmv_strong = Cell::new(false);

        let mut transforms = if move_all_fns {
            vec![
                lol_html::element!("h2", |h2| {
                    if let Some(id) = h2.get_attribute("id") {
                        functions.set(id == "functions");
                        h2.set_attribute("id", "properties").unwrap();
                    }
                    Ok(())
                }),
                lol_html::element!("a[href='#functions']", |a| {
                    a.set_attribute("href", "#properties").unwrap();
                    Ok(())
                }),
                lol_html::text!("a[href='#functions']", |t| {
                    if t.as_str() == "Functions" {
                        t.replace("Properties", ContentType::Text);
                    }
                    Ok(())
                }),
                lol_html::element!("div.item-row", |_| {
                    if functions.get() {
                        rmv_strong.set(true);
                    }
                    Ok(())
                }),
            ]
        } else {
            fn_entry.set(0);

            vec![
                lol_html::element!("h2", |h2| {
                    if let Some(id) = h2.get_attribute("id") {
                        functions.set(id == "functions");
                    }
                    Ok(())
                }),
                lol_html::element!("div.item-row", |div| {
                    if functions.get() {
                        fn_entry.set(fn_entry.get() + 1);
                        if tagged_fns.contains(&fn_entry.get()) {
                            rmv_strong.set(true);
                            div.before("<!-- CUT ", ContentType::Html);
                            div.after(" -->", ContentType::Html);
                        }
                    }
                    Ok(())
                }),
            ]
        };
        transforms.push(lol_html::element!("div.item-row strong", |s| {
            if rmv_strong.take() {
                s.remove();
            }
            Ok(())
        }));

        let mut html = super::util::rewrite_html(&html, transforms).unwrap();

        if !move_all_fns {
            let cut = Regex::new(r"<!-- CUT \s*((?s).+?)\s* -->").unwrap();

            let mut properties =
                r##"<h2 id="properties" class="small-section-header"><a href="#properties">Properties</a></h2><div class="item-table">"##
                    .to_owned();
            for cap in cut.captures_iter(&html) {
                properties.push_str(&cap[1]);
            }
            properties.push_str("</div>");

            html = super::util::rewrite_html(
                &html,
                vec![
                    lol_html::element!("h2#functions", |h2| {
                        h2.before(&properties, ContentType::Html);
                        Ok(())
                    }),
                    lol_html::comments!("div.item-table", |c| {
                        if c.text().contains("CUT") {
                            c.remove();
                        }
                        Ok(())
                    }),
                    lol_html::element!(".sidebar-elems a[href='#functions']", |a| {
                        let props = r##"<a href="#properties">Properties</a></li><li>"##;
                        a.before(props, ContentType::Html);
                        Ok(())
                    }),
                ],
            )
            .unwrap();
        }

        std::fs::write(file, html.as_bytes()).unwrap();
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
