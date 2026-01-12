#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Image loading and cache.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use zng_app::{AppExtension, update::EventUpdate, view_process::ViewImageHandle};
use zng_clone_move::async_clmv;
use zng_task as task;
use zng_task::channel::IpcBytes;
use zng_txt::*;
use zng_var::{Var, var};
use zng_view_api::image::ImageDecoded;

mod service;

mod types;
pub use types::*;

#[doc(inline)]
pub use service::render::{IMAGE_RENDER, IMAGES_WINDOW, ImageRenderWindowRoot, ImageRenderWindowsService, render_retain};

/// Application extension that provides an image cache.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`IMAGES`]
#[derive(Default)]
#[non_exhaustive]
pub struct ImageManager {}
impl AppExtension for ImageManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        service::on_app_event_preview(update);
    }

    fn update_preview(&mut self) {
        service::on_app_update_preview();
    }

    fn update(&mut self) {
        service::on_app_update();
    }
}

/// Image loading, cache and render service.
///
/// If the app is running without a [`VIEW_PROCESS`] all images are dummy, see [`load_in_headless`] for
/// details.
///
/// # Provider
///
/// This service is provided by the [`ImageManager`] extension, it will panic if used in an app not extended.
///
/// [`load_in_headless`]: IMAGES::load_in_headless
/// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
pub struct IMAGES;
impl IMAGES {
    /// If should still download/read image bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`VIEW_PROCESS`] is available, so
    /// images cannot be decoded, in this case all images are the [`dummy`] image and no attempt
    /// to download/read the image files is made. You can enable loading in headless tests to detect
    /// IO errors, in this case if there is an error acquiring the image file the image will be a
    /// [`dummy`] with error.
    ///
    /// [`dummy`]: IMAGES::dummy
    /// [`VIEW_PROCESS`]: zng_app::view_process::VIEW_PROCESS
    pub fn load_in_headless(&self) -> Var<bool> {
        service::load_in_headless()
    }

    /// Default loading and decoding limits for each image.
    pub fn limits(&self) -> Var<ImageLimits> {
        service::limits()
    }

    /// Returns a dummy image that reports it is loading or an error.
    pub fn dummy(&self, error: Option<Txt>) -> ImageVar {
        var(ImageEntry::new_empty(error.unwrap_or_default())).read_only()
    }

    /// Cache or load an image file from a file system `path`.
    pub fn read(&self, path: impl Into<PathBuf>) -> ImageVar {
        service::image(path.into().into(), ImageOptions::cache(), None)
    }

    /// Get a cached `uri` or download it.
    ///
    /// Optionally define the HTTP ACCEPT header, if not set all image formats supported by the view-process
    /// backend are accepted.
    #[cfg(feature = "http")]
    pub fn download<U>(&self, uri: U, accept: Option<Txt>) -> ImageVar
    where
        U: TryInto<task::http::Uri>,
        <U as TryInto<task::http::Uri>>::Error: ToTxt,
    {
        match uri.try_into() {
            Ok(uri) => service::image(ImageSource::Download(uri, accept), ImageOptions::cache(), None),
            Err(e) => self.dummy(Some(e.to_txt())),
        }
    }

    /// Get a cached image from `&'static [u8]` data.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    ///
    /// The image key is a [`ImageHash`] of the image data.
    ///
    /// # Examples
    ///
    /// Get an image from a PNG file embedded in the app executable using [`include_bytes!`].
    ///
    /// ```
    /// # use zng_ext_image::*;
    /// # macro_rules! include_bytes { ($tt:tt) => { &[] } }
    /// # fn demo() {
    /// let image_var = IMAGES.from_static(include_bytes!("ico.png"), "png");
    /// # }
    pub fn from_static(&self, data: &'static [u8], format: impl Into<ImageDataFormat>) -> ImageVar {
        service::image((data, format.into()).into(), ImageOptions::cache(), None)
    }

    /// Get a cached image from shared data.
    ///
    /// The image key is a [`ImageHash`] of the image data. The data reference is held only until the image is decoded.
    ///
    /// The data can be any of the formats described in [`ImageDataFormat`].
    pub fn from_data(&self, data: IpcBytes, format: impl Into<ImageDataFormat>) -> ImageVar {
        service::image((data, format.into()).into(), ImageOptions::cache(), None)
    }

    /// Get or load an image with full configuration.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    pub fn image(&self, source: impl Into<ImageSource>, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar {
        service::image(source.into(), options, limits)
    }

    /// Await for an image source, then get or load the image.
    ///
    /// If `limits` is `None` the [`IMAGES.limits`] is used.
    ///
    /// This method returns immediately with a loading [`ImageVar`], when `source` is ready it
    /// is used to get the actual [`ImageVar`] and binds it to the returned image.
    ///
    /// Note that the [`cache_mode`] always applies to the inner image, and only to the return image if `cache_key` is set.
    ///
    /// [`IMAGES.limits`]: IMAGES::limits
    /// [`cache_mode`]: ImageOptions::cache_mode
    pub fn image_task<F>(&self, source: impl IntoFuture<IntoFuture = F>, options: ImageOptions, limits: Option<ImageLimits>) -> ImageVar
    where
        F: Future<Output = ImageSource> + Send + 'static,
    {
        self.image_task_impl(Box::pin(source.into_future()), options, limits)
    }
    fn image_task_impl(
        &self,
        source: Pin<Box<dyn Future<Output = ImageSource> + Send + 'static>>,
        options: ImageOptions,
        limits: Option<ImageLimits>,
    ) -> ImageVar {
        let img = var(ImageEntry::new_empty(Txt::from_static("")));
        task::spawn(async_clmv!(img, {
            let source = source.await;
            let actual_img = service::image(source, options, limits);
            actual_img.set_bind(&img).perm();
            img.hold(actual_img).perm();
        }));
        img.read_only()
    }

    /// Associate the `image` produced by direct interaction with the view-process with the `key` in the cache.
    ///
    /// If the `key` is not set the image is not cached, the service only manages it until it is loaded.
    ///
    /// Returns `Ok(ImageVar)` with the new image var that tracks `image`, or `Err(image, ImageVar)`
    /// that returns the `image` and a clone of the var already associated with the `key`.
    ///
    /// Note that you can register entries on the returned [`ImageEntry::insert_entry`].
    #[allow(clippy::result_large_err)] // boxing here does not really help performance
    pub fn register(
        &self,
        key: Option<ImageHash>,
        image: (ViewImageHandle, ImageDecoded),
    ) -> std::result::Result<ImageVar, ((ViewImageHandle, ImageDecoded), ImageVar)> {
        service::register(key, image, Txt::from_static(""))
    }

    /// Remove the image from the cache, if it is only held by the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was removed.
    pub fn clean(&self, key: ImageHash) -> bool {
        service::remove(key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// You can use [`ImageSource::hash128_read`] and [`ImageSource::hash128_download`] to get the `key`
    /// for files or downloads.
    ///
    /// Returns `true` if the image was removed, that is, if it was cached.
    pub fn purge(&self, key: ImageHash) -> bool {
        service::remove(key, true)
    }

    /// Gets the cache key of an image.
    pub fn cache_key(&self, image: &ImageEntry) -> Option<ImageHash> {
        if let Some(key) = &image.cache_key
            && service::contains_key(key)
        {
            return Some(*key);
        }
        None
    }

    /// If the image is cached.
    pub fn is_cached(&self, image: &ImageEntry) -> bool {
        image.cache_key.as_ref().map(service::contains_key).unwrap_or(false)
    }

    /// Returns an image that is not cached.
    ///
    /// If the `image` is the only reference returns it and removes it from the cache. If there are other
    /// references a new [`ImageVar`] is generated from a clone of the image.
    pub fn detach(&self, image: ImageVar) -> ImageVar {
        service::detach(image)
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_all(&self) {
        service::clean_all()
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&self) {
        service::purge_all()
    }

    /// Add an images service extension.
    ///
    /// See [`ImagesExtension`] for extension capabilities.
    pub fn extend(&self, extension: Box<dyn ImagesExtension>) {
        service::extend(extension)
    }

    /// Image formats implemented by the current view-process and extensions.
    pub fn available_formats(&self) -> Vec<ImageFormat> {
        service::available_formats()
    }
}

fn absolute_path(path: &Path, base: impl FnOnce() -> PathBuf, allow_escape: bool) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        let mut dir = base();
        if allow_escape {
            dir.push(path);
            normalize_path(&dir)
        } else {
            dir.push(normalize_path(path));
            dir
        }
    }
}
/// Resolves `..` components, without any system request.
///
/// Source: https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
