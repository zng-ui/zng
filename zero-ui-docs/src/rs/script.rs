use std::{borrow::Cow, fs, path::Path};

use lol_html::{ElementContentHandlers, Selector};

/// Remove scripts inserted by [`crate::html_in_header`].
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/*.html", |file, html| {
        let html = super::util::rewrite_html(
            &html,
            vec![remove_dyn("macro.js"), remove_dyn("property.js"), remove_dyn("widget.js")],
        );
        if let Some(html) = html {
            fs::write(file, html.as_bytes()).unwrap();
        }
    })
}

fn remove_dyn(script_name: &str) -> (Cow<Selector>, ElementContentHandlers) {
    lol_html::element!(format!("script[data-zero-ui-dyn='{script_name}']"), |s| {
        s.remove();
        Ok(())
    })
}
