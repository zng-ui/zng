use std::{cell::Cell, collections::HashSet, path::Path};

use lol_html::html_content::ContentType;
use regex::Regex;

/// Edit Widget module pages and module lists.
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/index.html", |file, html| {
        let html = transform_widget_mod_page(&html).unwrap_or(html);
        if let Some(html) = transform_mod_list(&html) {
            std::fs::write(file, html.as_bytes()).unwrap();
        }
    });
    transform_sidebars(docs_root);
}

fn transform_widget_mod_page(_html: &str) -> Option<String> {
    None
}

fn transform_mod_list(html: &str) -> Option<String> {
    let modules = Cell::new(false);
    let mod_entry = Cell::new(0);
    let mut tagged_mods = HashSet::new();

    super::util::analyze_html(
        html,
        vec![
            lol_html::element!("h2", |h2| {
                if let Some(id) = h2.get_attribute("id") {
                    modules.set(id == "modules");
                }
                Ok(())
            }),
            lol_html::element!("div.item-row", |_| {
                if modules.get() {
                    mod_entry.set(mod_entry.get() + 1);
                }
                Ok(())
            }),
            lol_html::text!("div.item-row code", |t| {
                if modules.get() && t.as_str() == "widget" {
                    tagged_mods.insert(mod_entry.get());
                }
                Ok(())
            }),
        ],
    );

    if tagged_mods.is_empty() {
        return None;
    }

    let move_all_mods = tagged_mods.len() == mod_entry.get();

    modules.set(false);
    let rmv_strong = Cell::new(false);

    let mut transforms = if move_all_mods {
        vec![
            lol_html::element!("h2", |h2| {
                if let Some(id) = h2.get_attribute("id") {
                    modules.set(id == "modules");
                    h2.set_attribute("id", "widgets").unwrap();
                }
                Ok(())
            }),
            lol_html::element!("a[href='#widgets']", |a| {
                a.set_attribute("href", "#widgets").unwrap();
                Ok(())
            }),
            lol_html::text!("a[href='#widgets']", |t| {
                if t.as_str() == "Modules" {
                    t.replace("Widgets", ContentType::Text);
                }
                Ok(())
            }),
            lol_html::element!("div.item-row", |_| {
                if modules.get() {
                    rmv_strong.set(true);
                }
                Ok(())
            }),
        ]
    } else {
        mod_entry.set(0);

        vec![
            lol_html::element!("h2", |h2| {
                if let Some(id) = h2.get_attribute("id") {
                    modules.set(id == "modules");
                }
                Ok(())
            }),
            lol_html::element!("div.item-row", |div| {
                if modules.get() {
                    mod_entry.set(mod_entry.get() + 1);
                    if tagged_mods.contains(&mod_entry.get()) {
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

    let mut html = super::util::rewrite_html(html, transforms).unwrap();

    if !move_all_mods {
        let cut = Regex::new(r"<!-- CUT \s*((?s).+?)\s* -->").unwrap();

        let mut widgets =
            r##"<h2 id="widgets" class="small-section-header"><a href="#widgets">Widgets</a></h2><div class="item-table">"##.to_owned();
        for cap in cut.captures_iter(&html) {
            widgets.push_str(&cap[1]);
        }
        widgets.push_str("</div>");

        modules.set(false);

        html = super::util::rewrite_html(
            &html,
            vec![
                lol_html::element!("h2", |h2| {
                    if let Some(id) = h2.get_attribute("id") {
                        modules.set(id == "modules");
                    }
                    Ok(())
                }),
                lol_html::element!("div.item-table", |div| {
                    if modules.get() {
                        div.after(&widgets, ContentType::Html);
                    }
                    Ok(())
                }),
                lol_html::comments!("div.item-table", |c| {
                    if c.text().contains("CUT") {
                        c.remove();
                    }
                    Ok(())
                }),
                lol_html::element!(".sidebar-elems a[href='#modules']", |a| {
                    let props = r##"</li><li><a href="#widgets">Widgets</a>"##;
                    a.after(props, ContentType::Html);
                    Ok(())
                }),
            ],
        )
        .unwrap();
    }

    Some(html)
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
