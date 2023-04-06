use crate::core::image::ImageSource;
use crate::prelude::new_widget::*;

pub mod image_properties;
pub mod nodes;

/// Image presenter.
///
/// This widget loads a still image from a variety of sources and presents it.
#[widget($crate::widgets::image)]
pub mod image {
    pub use super::nodes;
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::image_properties::*;

    properties! {
        /// The image source.
        ///
        /// Can be a file path, an URI, binary included in the app and more.
        pub source(impl IntoVar<ImageSource>);
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(on_build);
    }

    fn on_build(wgt: &mut WidgetBuilding) {
        let node = nodes::image_presenter();
        let node = nodes::image_error_presenter(node);
        let node = nodes::image_loading_presenter(node);
        wgt.set_child(node);

        let source = wgt.capture_var::<ImageSource>(property_id!(self::source)).unwrap_or_else(|| {
            let error = Image::dummy(Some("no source".to_owned()));
            let error = ImageSource::Image(var(error).read_only());
            LocalVar(error).boxed()
        });
        wgt.push_intrinsic(NestGroup::EVENT, "image_source", |child| nodes::image_source(child, source));
    }
}

/// Shorthand form of [`image!`].
///
/// # Examples
///
/// Create an image button:
///
/// ```
/// # use zero_ui::prelude::*;
/// # use zero_ui::widgets::image::*;
/// # let _scope = App::minimal();
/// #
/// # let _ =
/// button! {
///     child = image("https://httpbin.org/image");
///     img_rendering = ImageRendering::Pixelated;
/// }
/// # ;
/// ```
///
/// Note that you can only define the [`source`] property in the image widget but you can
/// still use the [`image`] properties in the parent widget to define other properties.
///
/// [`image!`]: mod@image
/// [`image`]: mod@image
/// [`source`]: fn@image::source
pub fn image(source: impl IntoVar<ImageSource>) -> impl UiNode {
    image! { source }
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

        let img = var(crate::core::image::Image::dummy(Some("test error".to_string()))).read_only();

        let mut app = App::default().run_headless(false);
        IMAGES.load_in_headless().set(true);
        let ok = Arc::new(AtomicBool::new(false));
        let window_id = app.open_window(async_clmv!(ok, {
            window! {
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
