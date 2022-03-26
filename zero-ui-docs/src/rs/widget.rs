use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use lol_html::html_content::ContentType;
use regex::Regex;

/// Edit Widget module pages and module lists.
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/index.html", |file, mut html| {
        let mut edited = false;
        if let Some(h) = transform_widget_mod_page(&file, &html) {
            html = h;
            edited = true;
        }
        if let Some(h) = transform_mod_list(&html) {
            html = h;
            edited = true;
        }

        if edited {
            fs::write(file, html.as_bytes()).unwrap();
        }
    });
    transform_sidebars(docs_root);
}

fn transform_widget_mod_page(file: &Path, html: &str) -> Option<String> {
    if !html.contains(r#"<div class="docblock"><p><strong><code>widget</code></strong>"#) {
        return None;
    }

    let mut removed_tag = false;
    let modules = Cell::new(false);
    let mut removed_docs_entry = false;
    let mut remove_mods_section = true;
    let mut mod_sidebar_items_count = 0;

    let mut html = super::util::rewrite_html(
        html,
        vec![
            lol_html::text!("h1", |t| {
                if t.as_str() == "Module " {
                    t.replace("Widget ", ContentType::Text);
                }
                Ok(())
            }),
            lol_html::text!("h2", |t| {
                if let Some(m) = t.as_str().strip_prefix("Module ") {
                    let new_t = format!("Widget {m}");
                    t.replace(&new_t, ContentType::Text);
                }
                Ok(())
            }),
            lol_html::element!("div.docblock p strong", |strong| {
                if !removed_tag {
                    strong.remove();
                    removed_tag = true;
                }
                Ok(())
            }),
            lol_html::element!("iframe#wgt-docs-iframe", |iframe| {
                iframe.remove();
                Ok(())
            }),
            lol_html::element!("h2", |h2| {
                if let Some(id) = h2.get_attribute("id") {
                    modules.set(id == "modules");
                }
                Ok(())
            }),
            lol_html::element!("div.item-table div.item-row", |div| {
                if modules.get() {
                    if !removed_docs_entry {
                        removed_docs_entry = true;
                        div.remove();
                    } else {
                        remove_mods_section = false;
                    }
                }
                Ok(())
            }),
            lol_html::element!("div.sidebar-elems li", |_| {
                mod_sidebar_items_count += 1;
                Ok(())
            }),
        ],
    )
    .unwrap();

    if remove_mods_section {
        modules.set(false);
        let mut removed_sidebar_entry = false;
        mod_sidebar_items_count -= 1;
        html = super::util::rewrite_html(
            &html,
            vec![
                lol_html::element!("h2", |h2| {
                    if let Some(id) = h2.get_attribute("id") {
                        modules.set(id == "modules");
                        if modules.get() {
                            h2.remove();
                        }
                    }
                    Ok(())
                }),
                lol_html::element!("div.item-table", |div| {
                    if modules.get() {
                        div.remove();
                    }
                    Ok(())
                }),
                lol_html::element!("div.sidebar-elems li", |div| {
                    if !removed_sidebar_entry {
                        removed_sidebar_entry = true;
                        div.remove();
                    }
                    Ok(())
                }),
            ],
        )
        .unwrap();
    }

    let docs_file = file.parent().unwrap().join("__DOCS/index.html");
    let docs_html = fs::read_to_string(docs_file).unwrap();

    let first_docblock = Cell::new(true);
    let widget_sidebar_items = RefCell::new(vec![]);
    let mut removed_docblock_help = 0;

    let docs_html = super::util::rewrite_html(
        &docs_html,
        vec![
            lol_html::element!("div.docblock", |div| {
                if first_docblock.take() {
                    div.prepend("<!-- COPY ", ContentType::Html);
                    div.append(" -->", ContentType::Html);
                }
                Ok(())
            }),
            lol_html::element!("div.docblock p", |p| {
                if removed_docblock_help < 2 {
                    removed_docblock_help += 1;
                    p.remove();
                }
                Ok(())
            }),
            lol_html::element!("a[href^='../']", |a| {
                let href = a.get_attribute("href").unwrap();
                let href = href.strip_prefix("../").unwrap();

                a.set_attribute("href", href).unwrap();

                Ok(())
            }),
            lol_html::element!("h2#required-properties", |_| {
                widget_sidebar_items
                    .borrow_mut()
                    .push(("required-properties", "Required Properties"));
                Ok(())
            }),
            lol_html::element!("h2#normal-properties", |_| {
                widget_sidebar_items.borrow_mut().push(("normal-properties", "Normal Properties"));
                Ok(())
            }),
            lol_html::element!("h2#event-properties", |_| {
                widget_sidebar_items.borrow_mut().push(("event-properties", "Event Properties"));
                Ok(())
            }),
            lol_html::element!("h2#state-properties", |_| {
                widget_sidebar_items.borrow_mut().push(("state-properties", "State Properties"));
                Ok(())
            }),
            lol_html::element!("h2#when-properties", |_| {
                widget_sidebar_items.borrow_mut().push(("when-properties", "When Properties"));
                Ok(())
            }),
        ],
    )
    .unwrap();

    let inner_html = Regex::new(r#"<!-- COPY \s*((?s).+?)\s* -->"#).unwrap();
    let inner_html = &inner_html.captures(&docs_html).unwrap()[1];

    // matches:
    // <ul>\n<li><span id='{id}' class='wp-title'><strong><a href="{href}"><code>{name}</code></a></strong></span></li>\n</ul>
    let property_titles = Regex::new(r##"(?s)<ul>\s*<li><span id='(?P<id>[a-z\-_]+)' class='wp-title'><strong><a href="(?P<href>[\./a-z#\-_]+)"><code>(?P<name>[a-z_]+)</code></a></strong></span></li>\s*</ul>"##).unwrap();
    let inner_html = property_titles.replace_all(inner_html, |caps: &regex::Captures| {
        let id = &caps["id"];
        let href = &caps["href"];
        let name = &caps["name"];
        
        let prop_types = if let Some(pfn_file) = resolve_property_href(file, href) {
            todo!()
        } else {
            ""
        };

        format!(r##"<h3 id="{id}" class="wp-title variant small-section-header" style="overflow-x=visible;"><a href="#{id}" class="anchor field"></a><code style="background-color:transparent;"><a href="{href}">{name}</a>{prop_types}</code></h3>"##)
    });
    let inner_html = inner_html.as_ref();

    let mut sidebar_add = String::new();
    let widget_sidebar_items = widget_sidebar_items.into_inner();
    if !widget_sidebar_items.is_empty() {
        sidebar_add.push_str(r#"<div class="block"><h3 class="sidebar-title">Widget Items</h3><ul>"#);
        for (id, label) in widget_sidebar_items {
            sidebar_add.push_str(r##"<li><a href="#"##);
            sidebar_add.push_str(id);
            sidebar_add.push_str(r#"">"#);
            sidebar_add.push_str(label);
            sidebar_add.push_str("</a></li>");
        }
        sidebar_add.push_str("</ul></div>");
    }
    if mod_sidebar_items_count > 0 {
        sidebar_add.push_str(r#"<h3 class="sidebar-title">Module Items</h3>"#);
    }

    first_docblock.set(true);
    html = super::util::rewrite_html(
        &html,
        vec![
            lol_html::element!("div.docblock", move |div| {
                if first_docblock.take() {
                    div.append(inner_html, ContentType::Html);
                }
                Ok(())
            }),
            lol_html::element!("div.sidebar-elems section", |section| {
                section.prepend(&sidebar_add, ContentType::Html);
                Ok(())
            }),
        ],
    )
    .unwrap();

    Some(html)
}
fn resolve_property_href(file: &Path, href: &str) -> Option<PathBuf> {
    if let Some(name) = href.strip_prefix("fn@") {
        let pfn_file = format!("fn.__p_{name}.html");
        Some(file.join(pfn_file))
    } else if let Some((mod_file, id)) = href.rsplit_once('#') {
        let file = file.join(mod_file);
        if let Ok(html) = fs::read_to_string(file) {
            super::util::analyze_html(
                &html,
                vec![
                    lol_html::element!(format!("h3#{id} a"), |a| {
                        todo!("already rewritten, could copy <code> from here?");
                        Ok(())
                    }),
                    lol_html::element!(format!("span#{id} a"), |a| {
                        todo!("not rewritten");
                        Ok(())
                    }),
                ],
            );

            todo!()
        } else {
            None
        }
    } else {
        None
    }
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
                    if modules.get() {
                        h2.set_attribute("id", "widgets").unwrap();
                    }
                }
                Ok(())
            }),
            lol_html::element!("a[href='#modules']", |a| {
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

            fs::write(file, js.as_bytes()).unwrap();
        }
    });
}
