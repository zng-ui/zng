use crate::core::image::ImageSource;
use crate::prelude::new_widget::*;

pub mod nodes;
pub mod properties;

/// Image presenter.
///
/// This widget loads a still image from a variety of sources and presents it.
#[widget($crate::widgets::image)]
pub mod image {
    use super::*;
    pub use super::{nodes, properties};

    properties! {
        /// The image source.
        ///
        /// Can be a file path, an URI, binary included in the app and more.
        source(impl IntoVar<ImageSource>);

        /// Sets the image final size mode.
        ///
        /// By default the [`Contain`] mode is used.
        ///
        /// [`Contain`]: ImageFit::Contain
        properties::image_fit as fit;

        /// Align of the image after the final size is calculated.
        ///
        /// If the image is smaller then the widget area it is aligned like normal, if it is larger the "viewport" is aligned,
        /// so for examples, alignment [`BOTTOM_RIGHT`] makes a smaller image sit at the bottom-right of the widget and makes
        /// a larger image bottom-right fill the widget, clipping the rest.
        ///
        /// By default the alignment is [`CENTER`]. The [`BASELINE`] alignment is treaded the same as [`BOTTOM`].
        ///
        /// [`BOTTOM_RIGHT`]: Align::BOTTOM_RIGHT
        /// [`CENTER`]: Align::CENTER
        /// [`BASELINE`]: Align::BASELINE
        /// [`BOTTOM`]: Align::BOTTOM
        properties::image_align;

        /// Offset applied to the image after the final size and alignment.
        ///
        /// Relative values are calculated from the widget final size. Note that this is different the applying the
        /// [`offset`] property on the widget it-self, the widget is not moved just the image within the widget area.
        ///
        /// By default no offset is applied.
        ///
        /// [`offset`]: crate::properties::offset
        properties::image_offset;

        /// Simple clip rectangle applied to the image before all layout.
        ///
        /// Relative values are calculated from the image pixel size, the [`scale_ppi`] is only considered after.
        /// Note that more complex clipping can be applied after to the full widget, this property exists primarily to
        /// render selections of a [texture atlas].
        ///
        /// By default no cropping is done.
        ///
        /// [`scale_ppi`]: #wp-scale_ppi
        /// [texture atlas]: https://en.wikipedia.org/wiki/Texture_atlas
        properties::image_crop as crop;

        /// Scale applied to the image desired size.
        ///
        /// The scaling is applied after [`scale_ppi`] if active.
        ///
        /// By default not scaling is done.
        ///
        /// [`scale_ppi`]: #wp-scale_ppi
        properties::image_scale;

        /// Sets the image scaling algorithm used to rescale the image in the renderer.
        ///
        /// If the image layout size is not the same as the `source` pixel size the image must be re-scaled
        /// during rendering, this property selects what algorithm is used to do this re-scaling.
        ///
        /// Note that the algorithms used in the renderer value performance over quality and do a good
        /// enough job for small or temporary changes in scale only. If the image stays at a very different scale
        /// after a short time a CPU re-scale task is automatically started to generate a better quality re-scaling.
        ///
        /// If the image is an app resource known during build time you should consider pre-scaling it to match the screen
        /// size at different DPIs using mipmaps.
        ///
        /// This is [`ImageRendering::Auto`] by default.
        properties::image_rendering as rendering;

        /// Sets if the [`source`] is cached.
        ///
        /// By default this is `true`, meaning the image is loaded from cache and if not present it is inserted into
        /// the cache, the cache lives for the app in the [`Images`] app, the image can be manually removed from cache.
        ///
        /// If set to `false` the image is always loaded and decoded on init or when [`source`] updates and is dropped when
        /// the widget is deinited or dropped.
        ///
        /// [`source`]: #wp-source
        /// [`Images`]: zero_ui::core::image::Images
        properties::image_cache as cache;

        /// If the image desired size is scaled by the screen scale factor.
        ///
        /// The image desired size is its original size after [`crop`], it is a pixel value, but widgets are layout using
        /// device independent pixels that automatically scale in higher definition displays, when this property is enabled
        /// the image size is also scaled so that the image will take the same screen space in all devices, the image can end
        ///
        /// This is enabled by default.
        ///
        /// [`crop`]: #wp-crop
        properties::image_scale_factor as scale_factor;

        /// If the image desired size is scaled by PPI.
        ///
        /// The image desired size is its original size, after [`crop`], and it can be in pixels or scaled considering
        /// the image PPI, monitor PPI and scale factor.
        ///
        /// By default this is `false`, if `true` the image is scaled in a attempt to recreate the original physical dimensions, but it
        /// only works if the image and monitor PPI are set correctly. The monitor PPI can be set using the [`Monitors`] service.
        ///
        /// [`crop`]: #wp-crop
        /// [`Monitors`]: zero_ui::core::window::Monitors
        properties::image_scale_ppi as scale_ppi;

        /// View generator that creates the loading content.
        properties::image_loading_view as loading_view;

        /// View generator that creates the error content when the image failed to load.
        properties::image_error_view as error_view;

        /// Sets custom image load and decode limits.
        ///
        /// If not set or set to `None` the [`Images::limits`] is used.
        properties::image_limits as limits;

        /// Block window load until the image loads.
        ///
        /// If the image widget is in the initial window content the window view opening is blocked until the image source
        /// loads, fails to load or a timeout elapses.
        ///
        /// You can enable this behavior by setting this to `true` for a timeout of `1.secs()`, or you can set it to a
        /// timeout duration directly. Note that the input is a fixed value, not a variable.
        properties::image_block_window_load as block_window_load;

        /// If the image successfully loaded.
        properties::is_loaded;

        /// Event called when the images successfully loads.
        properties::on_load;

        /// If the image failed to load.
        properties::is_error;

        /// Event called when the image fails to load.
        properties::on_error;
    }

    fn new_child() -> impl UiNode {
        let node = nodes::image_presenter();
        let node = nodes::image_error_presenter(node);
        nodes::image_loading_presenter(node)
    }

    fn new_event(child: impl UiNode, source: impl IntoVar<ImageSource>) -> impl UiNode {
        nodes::image_source(child, source)
    }
}

/// Shorthand form of [`image!`].
///
/// # Examples
///
/// Create an image button:
///
/// ```
/// use zero_ui::prelude::*;
/// use zero_ui::widgets::image::properties::*;
///
/// # let _ =
/// button! {
///     content = image("https://httpbin.org/image");
///     image_rendering = ImageRendering::Pixelated;
/// }
/// # ;
/// ```
///
/// Note that you can only define the [`source`] property in the image widget but you can
/// still use the [`image::properties`] in the parent widget to define other properties.
///
/// [`image!`]: mod@image
/// [`source`]: mod@image#wp-source
pub fn image(source: impl IntoVar<ImageSource>) -> impl Widget {
    image! { source }
}

#[cfg(test)]
mod tests {
    use crate::core::image::Images;
    use crate::prelude::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn error_view_recursion() {
        crate::core::test_log();

        let img = var(crate::core::image::Image::dummy(Some("test error".to_string()))).into_read_only();

        let mut app = App::default().run_headless(false);
        Images::req(&mut app).load_in_headless = true;
        let ok = Rc::new(Cell::new(false));
        let window_id = app.open_window(clone_move!(ok, |_| {
            window! {
                content = image! {
                    source = img.clone();
                    error_view = view_generator!(ok, |_, _| {
                        ok.set(true);
                        image! {
                            source = img.clone();
                        }
                    });
                }
            }
        }));

        let _ = app.update(false);
        app.close_window(window_id);

        assert!(ok.get());
    }
}
