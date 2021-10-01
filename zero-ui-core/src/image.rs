//! Image loading and cache.

use std::{collections::HashMap, future::Future, mem, path::PathBuf};

use zero_ui_view_api::ImageDataFormat;

use crate::{
    app::{
        raw_events::{RawImageLoadErrorEvent, RawImageLoadedEvent},
        view_process::{ViewImage, ViewProcess, ViewProcessRespawnedEvent},
        AppEventSender, AppExtension,
    },
    context::AppContext,
    event::EventUpdateArgs,
    service::Service,
    task::{
        fs,
        http::{self, header, Request, TryUri, Uri},
        io::*,
        ui::UiTask,
    },
    var::{var, RcVar, ReadOnlyRcVar, Var, Vars, WithVars},
};

/// Application extension that provides an image cache.
///
/// # Services
///
/// Services this extension provides.
///
/// * [Images]
///
/// # Default
///
/// This extension is included in the [default app], events provided by it
/// are required by multiple other extensions.
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct ImageManager {}
impl AppExtension for ImageManager {
    fn init(&mut self, ctx: &mut AppContext) {
        let images = Images::new(ctx.services.get::<ViewProcess>().cloned(), ctx.updates.sender());
        ctx.services.register(images);
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(image) = RawImageLoadedEvent
            .update(args)
            .map(|a| &a.image)
            .or_else(|| RawImageLoadErrorEvent.update(args).map(|a| &a.image))
        {
            // image is ready for use or failed to decode, remove from `decoding`
            // and notify the ViewImage inner state update.
            let images = ctx.services.images();
            let vars = ctx.vars;
            if let Some(i) = images.decoding.iter().position(|v| v.get(vars).view.as_ref().unwrap() == image) {
                let var = images.decoding.swap_remove(i);
                var.touch(ctx.vars);
            }
            todo!()
        } else if ViewProcessRespawnedEvent.update(args).is_some() {
            let images = ctx.services.images();
            for v in images.cache.values() {
                todo!("reload images")
            }
        }
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        // update loading tasks:

        let images = ctx.services.images();
        let view = &images.view;
        let decoding = &mut images.decoding;
        images.loading.retain(|(task, var)| {
            const DROP: bool = false;
            const RETAIN: bool = true;

            if let Some(d) = task.update() {
                if d.data.is_empty() {
                    // load error.
                    var.set(
                        ctx.vars,
                        Image {
                            view: Some(ViewImage::dummy(Some(mem::take(&mut d.error)))),
                        },
                    );
                    DROP
                } else if let Some(vp) = view {
                    // success and we have a view-process.
                    let mut respawn_retries = 0;
                    loop {
                        match vp.cache_image(d.data, d.format) {
                            Ok(img) => {
                                // request send, add to `decoding` will receive
                                // `RawImageLoadedEvent` or `RawImageLoadErrorEvent` event
                                // when done.
                                var.set(ctx.vars, Image { view: Some(img) });
                                decoding.push(var.clone());
                                break;
                            }
                            Err(Respawned) => {
                                respawn_retries += 1;
                                if respawn_retries == 5 {
                                    var.set(
                                        ctx.vars,
                                        Image {
                                            view: Some(ViewImage::dummy(Some("view-process respawned 5 times".to_owned()))),
                                        },
                                    );
                                    break;
                                } else {
                                    continue;
                                }
                            }
                        }
                    }
                    DROP
                } else {
                    // success, but we are only doing `load_in_headless` validation.
                    var.set(
                        ctx.vars,
                        Image {
                            view: Some(ViewImage::dummy(None)),
                        },
                    );
                    DROP
                }
            } else {
                RETAIN
            }
        });
    }
}

/// The [`Image`] loading cache service.
///
/// If the app is running without a [`ViewProcess`] all images are dummy, see [`load_in_headless`] for
/// details.
///
/// [`load_in_headless`]: Images::load_in_headless
#[derive(Service)]
pub struct Images {
    /// If should still download/read image bytes in headless/renderless mode.
    ///
    /// When an app is in headless mode without renderer no [`ViewProcess`] is available, so
    /// images cannot be decoded, in this case all images are the [`dummy`] image and no attempt
    /// to download/read the image files is made. You can enable loading in headless tests to detect
    /// IO errors, in this case if there is an error acquiring the image file the image will be a
    /// [`dummy`] with error.
    ///
    /// [`dummy`]: Images::dummy
    pub load_in_headless: bool,

    view: Option<ViewProcess>,
    updates: AppEventSender,
    proxies: Vec<Box<dyn ImageCacheProxy>>,

    loading: Vec<(UiTask<ImageData>, RcVar<Image>)>,
    decoding: Vec<RcVar<Image>>,
    cache: HashMap<ImageCacheKey, RcVar<Image>>,
}
impl Images {
    fn new(view: Option<ViewProcess>, updates: AppEventSender) -> Self {
        Images {
            load_in_headless: false,
            view,
            updates,
            proxies: vec![],
            loading: vec![],
            decoding: vec![],
            cache: HashMap::default(),
        }
    }

    /// Returns a dummy image that reports it is loaded with optional error.
    pub fn dummy(&self, error: Option<String>) -> ImageVar {
        var(Image {
            view: Some(ViewImage::dummy(error)),
        })
        .into_read_only()
    }

    /// Get or load an image file from a file system `path`.
    pub fn read<Vw: WithVars>(&mut self, vars: &Vw, path: impl Into<PathBuf>) -> ImageVar {
        self.get(vars, ImageCacheKey::Read(path.into()))
    }

    /// Get a cached `uri` or download it.
    pub fn download<Vw: WithVars>(&mut self, vars: &Vw, uri: impl TryUri) -> ImageVar {
        match uri.try_into() {
            Ok(uri) => self.get(vars, ImageCacheKey::Download(uri)),
            Err(e) => self.dummy(Some(e.to_string())),
        }
    }

    /// Get a cached image or add it to the cache.
    pub fn get<Vw: WithVars>(&mut self, vars: &Vw, key: ImageCacheKey) -> ImageVar {
        vars.with_vars(move |vars| self.proxy_then_get(key, vars))
    }

    /// Associate the `image` with the `key` in the cache.
    ///
    /// Returns `Some(previous)` if the `key` was already associated with an image.
    #[inline]
    pub fn register(&mut self, key: ImageCacheKey, image: ViewImage) -> Option<ImageVar> {
        self.cache.insert(key, var(Image { view: Some(image) })).map(|v| v.into_read_only())
    }

    /// Remove the image from the cache, if it is only held by the cache.
    pub fn clean(&mut self, key: ImageCacheKey) -> Option<ImageVar> {
        self.proxy_then_remove(key, false)
    }

    /// Remove the image from the cache, even if it is still referenced outside of the cache.
    ///
    /// Returns `Some(img)` if the image was cached.
    pub fn purge(&mut self, key: ImageCacheKey) -> Option<ImageVar> {
        self.proxy_then_remove(key, true)
    }

    fn proxy_then_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<ImageVar> {
        for proxy in &mut self.proxies {
            let r = proxy.remove(&key, purge);
            match r {
                ProxyRemoveResult::None => continue,
                ProxyRemoveResult::Remove(r, p) => return self.proxied_remove(r, p),
                ProxyRemoveResult::Removed(img) => return img,
            }
        }
        self.proxied_remove(key, purge)
    }
    fn proxied_remove(&mut self, key: ImageCacheKey, purge: bool) -> Option<ImageVar> {
        if purge {
            self.cache.remove(&key).map(|v| v.into_read_only())
        } else {
            todo!()
        }
    }

    /// Clear cached images that are not referenced outside of the cache.
    pub fn clean_all(&mut self) {
        self.proxies.iter_mut().for_each(|p| p.clear(false));
        todo!()
    }

    /// Clear all cached images, including images that are still referenced outside of the cache.
    ///
    /// Image memory only drops when all strong references are removed, so if an image is referenced
    /// outside of the cache it will merely be disconnected from the cache by this method.
    pub fn purge_all(&mut self) {
        self.cache.clear();
        self.proxies.iter_mut().for_each(|p| p.clear(true));
    }

    /// Add a cache proxy.
    ///
    /// Proxies can intercept cache requests and map to a different request or return an image directly.
    pub fn install_proxy(&mut self, proxy: Box<dyn ImageCacheProxy>) {
        self.proxies.push(proxy);
    }

    fn proxy_then_get(&mut self, key: ImageCacheKey, vars: &Vars) -> ImageVar {
        for proxy in &mut self.proxies {
            let r = proxy.get(&key);
            match r {
                ProxyGetResult::None => continue,
                ProxyGetResult::Cache(r) => return self.proxied_get(r, vars),
                ProxyGetResult::Image(img) => return img,
            }
        }
        self.proxied_get(key, vars)
    }
    fn proxied_get(&mut self, key: ImageCacheKey, vars: &Vars) -> ImageVar {
        if let Some(img) = self.cache.get(&key) {
            return img.clone().into_read_only();
        }

        if self.view.is_none() && !self.load_in_headless {
            let dummy = var(Image {
                view: Some(ViewImage::dummy(None)),
            });
            self.cache.insert(key, dummy.clone());
            return dummy.into_read_only();
        }

        match key.clone() {
            ImageCacheKey::Read(path) => self.load_task(key, async {
                let mut r = ImageData {
                    format: path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| ImageDataFormat::FileExt(s.to_owned()))
                        .unwrap_or(ImageDataFormat::Unknown),
                    data: vec![],
                    error: String::new(),
                };

                let file = match fs::File::open(path).await {
                    Ok(f) => f,
                    Err(e) => {
                        r.error = e.to_string();
                        return r;
                    }
                };

                if let Err(e) = file.read_to_end(&mut r.data).await {
                    r.data = vec![];
                    r.error = e.to_string();
                }

                r
            }),
            ImageCacheKey::Download(uri) => self.load_task(key, async {
                let mut r = ImageData {
                    format: ImageDataFormat::Unknown,
                    data: vec![],
                    error: String::new(),
                };

                // TODO get supported decoders from view-process?
                //
                // for image crate:
                // image/webp decoder is only grayscale: https://docs.rs/image/0.23.14/image/codecs/webp/struct.WebPDecoder.html
                // image/avif decoder does not build in Windows
                let request = Request::get(uri)
                    .unwrap()
                    .header(header::ACCEPT, "image/apng,image/*")
                    .unwrap()
                    .build();

                match http::send(request).await {
                    Ok(rsp) => {
                        if let Some(m) = rsp.headers().get(&header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                            let m = m.to_lowercase();
                            if m.starts_with("image/") {
                                r.format = ImageDataFormat::Mime(m);
                            }
                        }

                        match rsp.bytes().await {
                            Ok(d) => {
                                r.data = d;
                            }
                            Err(e) => {
                                r.error = format!("download error: {}", e);
                            }
                        }

                        let _ = rsp.consume();
                    }
                    Err(e) => {
                        r.error = format!("request error: {}", e);
                    }
                }

                r
            }),
        }
    }

    fn load_task(&mut self, key: ImageCacheKey, fetch_bytes: impl Future<Output = ImageData> + 'static) -> ImageVar {
        let img = var(Image { view: None });
        let task = UiTask::new(&self.updates, fetch_bytes);

        self.cache.insert(key, img.clone());
        self.loading.push((task, img.clone()));

        img.into_read_only()
    }
}

struct ImageData {
    format: ImageDataFormat,
    data: Vec<u8>,
    error: String,
}

/// Key for a cached image in [`Images`].
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum ImageCacheKey {
    /// A path to an image file in the file system.
    Read(PathBuf),
    /// A uri to an image resource downloaded using HTTP GET.
    Download(Uri),
}

/// A custom proxy in [`Images`].
///
/// Implementers can intercept cache requests and redirect to another cache request or returns an image directly.
pub trait ImageCacheProxy {
    /// Intercept a get request.
    fn get(&mut self, key: &ImageCacheKey) -> ProxyGetResult {
        let _ = key;
        ProxyGetResult::None
    }

    /// Intercept a remove request.
    fn remove(&mut self, key: &ImageCacheKey, purge: bool) -> ProxyRemoveResult {
        let _ = (key, purge);
        ProxyRemoveResult::None
    }

    /// Called when the cache is cleaned or purged.
    fn clear(&mut self, purge: bool);
}

/// Result of an [`ImageCacheProxy`] *get* redirect.
pub enum ProxyGetResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Load and cache using the replacement key.
    Cache(ImageCacheKey),
    /// Return the image instead of hitting the cache.
    Image(ImageVar),
}

/// Result of an [`ImageCacheProxy`] *remove* redirect.
pub enum ProxyRemoveResult {
    /// Proxy does not intercept the request.
    ///
    /// The cache checks other proxies and fulfills the request if no proxy intercepts.
    None,
    /// Removes another cached entry.
    ///
    /// The `bool` indicates if the entry should be purged.
    Remove(ImageCacheKey, bool),
    /// Consider the request fulfilled.
    Removed(Option<ImageVar>),
}

/// Represents an [`Image`] tracked by the [`Images`] cache.
///
/// The variable updates when the image updates.
pub type ImageVar = ReadOnlyRcVar<Image>;

/// State of an [`ImageVar`].
///
/// Each instance of this struct represent a single state,
#[derive(Debug, Clone)]
pub struct Image {
    view: Option<ViewImage>,
}
impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}
impl Image {
    /// Returns `true` if the is still acquiring or decoding the image bytes.
    pub fn is_loading(&self) -> bool {
        match &self.view {
            Some(v) => !v.loaded() && !v.is_error(),
            None => true,
        }
    }

    /// If the image is successfully loaded in the view-process.
    pub fn is_loaded(&self) -> bool {
        match &self.view {
            Some(v) => v.loaded(),
            None => false,
        }
    }

    /// If the image failed to load.
    pub fn is_error(&self) -> bool {
        match &self.view {
            Some(v) => v.is_error(),
            None => false,
        }
    }

    /// Returns an error message if the image failed to load.
    pub fn error(&self) -> Option<String> {
        match &self.view {
            Some(v) => v.error(),
            None => None,
        }
    }

    /// Connection to the image resource, if it is loaded.
    pub fn img(&self) -> Option<&ViewImage> {
        match &self.view {
            Some(v) => {
                if v.loaded() {
                    Some(v)
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
