use std::path::Path;

/// Remove scripts inserted by [`crate::html_in_header`].
pub fn transform(docs_root: &Path) {
    super::util::glob_par_each(docs_root, "**/*.html", |file, html| {
        let html = super::util::rewrite_html(
            &html,
            vec![lol_html::element!("script[data-zero-ui]", |s| {
                s.remove();
                Ok(())
            })],
        );

        if let Some(html) = html {
            std::fs::write(file, html.as_bytes()).unwrap();
        }
    });
}
