#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;

use zero_ui::{
    core::image::{PathFilter, UriFilter},
    prelude::*,
    widgets::image::ImageLimits,
};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("markdown");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Markdown Example";
            child = scroll! {
                mode = ScrollMode::VERTICAL;
                padding = 10;
                child = markdown! {
                    md = std::fs::read_to_string("examples/res/markdown/sample.md").unwrap_or_else(|e| e.to_string());

                    // allow limited image download and read.
                    image::img_limits = ImageLimits::default()
                        .with_allow_uri(UriFilter::allow_host("httpbin.org"))
                        .with_allow_path(PathFilter::allow_dir("examples/res"));

                    /// fix path to local images.
                    image_resolver = markdown::ImageResolver::new(|img| {
                        let mut r: ImageSource = img.into();
                        if let ImageSource::Read(file) = &mut r {
                            if file.is_relative() {
                                *file = PathBuf::from("examples/res/markdown").join(&file);
                            }
                        }
                        r
                    });
                };
            };
        }
    })
}
