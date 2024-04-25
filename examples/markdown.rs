//! Demonstrates the `Markdown!` widget.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;

use zng::{
    image::{self, ImageLimits, PathFilter, UriFilter},
    markdown::{self, Markdown},
    prelude::*,
    scroll::ScrollMode,
};

use zng::view_process::prebuilt as view_process;

fn main() {
    examples_util::print_info();
    view_process::init();
    zng::app::crash_handler::init_debug();

    // let rec = examples_util::record_profile("markdown");

    // view_process::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            title = "Markdown Example";
            child = Scroll! {
                mode = ScrollMode::VERTICAL;
                padding = 10;
                child = Markdown! {
                    txt = std::fs::read_to_string("examples/res/markdown/sample.md").unwrap_or_else(|e| e.to_string());

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

                    /// fix relative link to files.
                    link_resolver = markdown::LinkResolver::base_dir("examples/res/markdown");
                };
            };
        }
    })
}
