//! Image widget and helpers.

use crate::core::image::ImageSource;
use crate::prelude::new_widget::*;

mod image_properties;
pub use image_properties::*;

pub mod nodes;

/// Image presenter.
///
/// This widget loads a still image from a variety of sources and presents it.
///
#[widget($crate::widgets::Image {
    ($source:expr) => {
        source = $source;
    };
})]
pub struct Image(WidgetBase);
impl Image {
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.builder().push_build_action(on_build);
    }
}

/// The image source.
///
/// Can be a file path, an URI, binary included in the app and more.
#[property(CONTEXT, capture, impl(Image))]
pub fn source(child: impl UiNode, source: impl IntoVar<ImageSource>) -> impl UiNode {}

fn on_build(wgt: &mut WidgetBuilding) {
    let node = nodes::image_presenter();
    let node = nodes::image_error_presenter(node);
    let node = nodes::image_loading_presenter(node);
    wgt.set_child(node);

    let source = wgt.capture_var::<ImageSource>(property_id!(Self::source)).unwrap_or_else(|| {
        let error = Image::dummy(Some("no source".to_owned()));
        let error = ImageSource::Image(var(error).read_only());
        LocalVar(error).boxed()
    });
    wgt.push_intrinsic(NestGroup::EVENT, "image_source", |child| nodes::image_source(child, source));
}

#[cfg(test)]
mod tests {
    use crate::core::image::IMAGES;
    use crate::prelude::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn error_view_recursion() {
        crate::core::test_log();

        let img = var(crate::core::image::Img::dummy(Some("test error".to_string()))).read_only();

        let mut app = App::default().run_headless(false);
        IMAGES.load_in_headless().set(true);
        let ok = Arc::new(AtomicBool::new(false));
        let window_id = app.open_window(async_clmv!(ok, {
            WindowCfg! {
                child = image! {
                    source = img.clone();
                    img_error_fn = wgt_fn!(ok, |_| {
                        ok.store(true, Ordering::Relaxed);
                        image! {
                            source = img.clone();
                        }
                    });
                }
            }
        }));

        let _ = app.update(false);
        app.close_window(window_id);

        assert!(ok.load(Ordering::Relaxed));
    }
}
