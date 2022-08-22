use std::{borrow::Cow, cell::Cell, collections::HashSet, path::Path};

use lol_html::html_content::ContentType;
use regex::Regex;

use crate::rs::util::RegexExt;

/// Edit property function pages and function lists.
pub fn transform(docs_root: &Path) {
    transform_property_fn_pages(docs_root);
    transform_fn_lists(docs_root);
}

/// Edit function pages for property functions.
fn transform_property_fn_pages(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/fn.*.html", |file, html| {
        if !html.contains(r#"<div class="docblock"><p><strong><code>property</code></strong>"#) {
            return;
        }

        let mut removed_tag = false;

        let html = super::util::rewrite_html(
            &html,
            vec![
                lol_html::text!("h1", |t| {
                    if t.as_str() == "Function " {
                        t.replace("Property ", ContentType::Text);
                    }
                    Ok(())
                }),
                lol_html::element!("pre.rust.fn", |pre| {
                    pre.before("<!-- COPY ", ContentType::Html);
                    pre.after(" -->", ContentType::Html);
                    Ok(())
                }),
                lol_html::element!("div.docblock p strong", |strong| {
                    if !removed_tag {
                        strong.remove();
                        removed_tag = true;
                    }
                    Ok(())
                }),
            ],
        )
        .unwrap();

        let copy = Regex::new(r"<!-- COPY \s*((?s).+?)\s* -->").unwrap();
        let code = &copy.captures(&html).unwrap()[1];

        let transformed_code = &transform_property_decl(code);

        let html = super::util::rewrite_html(
            &html,
            vec![
                lol_html::element!("div.item-decl", |div| {
                    div.set_inner_content(transformed_code, ContentType::Html);
                    Ok(())
                }),
                lol_html::element!("h2#as-function", |h2| {
                    h2.after(code, ContentType::Html);
                    Ok(())
                }),
            ],
        )
        .unwrap();

        std::fs::write(file, html.as_bytes()).unwrap();
    });
}
fn transform_property_decl(pre: &str) -> String {
    let capture_only = pre.contains(r#"primitive.never.html">!</a></code></pre>"#);

    // remove return type
    let end_cut = pre.rfind(") -&gt; ").unwrap(); // match last " ) -> "
    let (pre, _after_arrow) = pre.split_at(end_cut + 1);

    let fn_idx = pre.find("fn ").unwrap();
    let (open_and_vis, pre) = pre.split_at(fn_idx);
    let open_and_vis = open_and_vis.trim_end();
    let pre = pre["fn ".len()..].trim_start();

    // match first < or (
    let first_paren = pre.find('(').unwrap();
    let first_open_generic = pre.find("&lt;").unwrap_or(usize::MAX);
    let name_end = first_paren.min(first_open_generic);
    let (name, pre) = pre.split_at(name_end);

    let (generic, pre) = if pre.starts_with("&lt;") {
        pre.split_at(first_paren - first_open_generic)
    } else {
        ("", pre)
    };

    let pre = pre.strip_prefix('(').unwrap().strip_suffix(')').unwrap();

    // split at ", input_name:"
    let comma_and_input_name = Regex::new(r",\s*(<br>)?(?:&nbsp;)*\s*\w+:").unwrap();
    let inputs: Vec<_> = comma_and_input_name.split_keep(pre).collect();
    let inputs = if capture_only { &inputs[..] } else { &inputs[1..] };

    let input = if inputs.len() == 1 {
        Cow::Borrowed(inputs[0].split_once(':').unwrap().1.trim().trim_end_matches("<br>"))
    } else {
        let mut r = "{<br>".to_owned();
        for input in inputs {
            let input = input
                .trim_start_matches(',')
                .trim_start()
                .trim_start_matches("<br>")
                .trim_start_matches("&nbsp;")
                .trim_end_matches("<br>");

            r.push_str("&nbsp;&nbsp;&nbsp;");
            r.push_str(input);
            r.push_str(",<br>");
        }
        r.push('}');
        Cow::Owned(r)
    };
    format!("{open_and_vis} {name}{generic} = {input};</code></pre>")
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
                        if functions.get() {
                            h2.set_attribute("id", "properties").unwrap();
                        }
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
