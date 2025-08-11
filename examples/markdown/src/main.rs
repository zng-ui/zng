//! Demonstrates the `Markdown!` widget.

use zng::{
    image::{self, ImageLimits, PathFilter, UriFilter},
    markdown::{self, Markdown},
    prelude::*,
    scroll::ScrollMode,
};

fn main() {
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            title = "Markdown Example";
            child = Scroll! {
                mode = ScrollMode::VERTICAL;
                padding = 10;
                child = Markdown! {
                    txt = std::fs::read_to_string(zng::env::res("sample.md")).unwrap_or_else(|e| e.to_string());
                    txt_selectable = true;

                    // allow limited image download and read.
                    image::img_limits = ImageLimits::default()
                        .with_allow_uri(UriFilter::allow_host("httpbin.org"))
                        .with_allow_path(PathFilter::allow_dir("examples"));

                    /// fix path to local images.
                    image_resolver = markdown::ImageResolver::new(|img| {
                        let mut r: ImageSource = img.into();
                        if let ImageSource::Read(file) = &mut r
                            && file.is_relative()
                        {
                            *file = zng::env::res(&file);
                        }
                        r
                    });

                    /// fix relative link to files.
                    link_resolver = markdown::LinkResolver::base_dir("examples/res/markdown");
                };
            };
        }
    })
}
