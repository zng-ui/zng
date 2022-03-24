use std::{cell::Cell, collections::HashSet, path::Path};

/// Remove macro items from the crates front pages if their are tagged with `<span data-del-macro-root></span>`.
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "*/index.html", |file, html| {
        let is_front_page = Cell::new(false);
        let macros = Cell::new(false);
        let macro_entry = Cell::new(0);
        let mut tagged_macros = HashSet::new();
        super::util::analyze_html(
            &html,
            vec![
                lol_html::text!("h1", |t| {
                    if !is_front_page.get() && t.as_str().starts_with("Crate ") {
                        is_front_page.set(true);
                    }
                    Ok(())
                }),
                lol_html::element!("h2", |h2| {
                    if let Some(id) = h2.get_attribute("id") {
                        macros.set(id == "macros");
                    }
                    Ok(())
                }),
                lol_html::element!("div.item-row", |_| {
                    if macros.get() {
                        macro_entry.set(macro_entry.get() + 1);
                    }
                    Ok(())
                }),
                lol_html::element!("span[data-del-macro-root]", |_| {
                    tagged_macros.insert(macro_entry.get());
                    Ok(())
                }),
            ],
        );

        if !is_front_page.get() || tagged_macros.is_empty() {
            return;
        }

        let rmv_all_macros = tagged_macros.len() == macro_entry.get();
        macros.set(false);
        let mut macro_entry = 0;

        let mut transforms = vec![lol_html::element!("h2", |h2| {
            if let Some(id) = h2.get_attribute("id") {
                macros.set(id == "macros");
                if rmv_all_macros && macros.get() {
                    h2.remove();
                }
            }
            Ok(())
        })];
        if rmv_all_macros {
            transforms.push(lol_html::element!("div.item-table", |div| {
                if macros.get() {
                    div.remove();
                }
                Ok(())
            }));
            transforms.push(lol_html::element!("a[href='#macros']", |a| {
                a.remove();
                Ok(())
            }))
        } else {
            transforms.push(lol_html::element!("div.item-row", |div| {
                if macros.get() {
                    macro_entry += 1;
                    if tagged_macros.contains(&macro_entry) {
                        div.remove();
                    }
                }

                Ok(())
            }));
        }

        let html = super::util::rewrite_html(&html, transforms);

        if let Some(html) = html {
            std::fs::write(file, html.as_bytes()).unwrap();
        }
    });
}
