use std::fmt;
use std::sync::Arc;

use crate::core::{image::ImageSource, task::parking_lot::Mutex};
use crate::prelude::new_property::*;
use crate::properties::events::gesture::on_click;

context_var! {
    /// Markdown image resolver.
    pub static IMAGE_RESOLVER_VAR: ImageResolver = ImageResolver::Default;

    /// Markdown link resolver.
    pub static LINK_RESOLVER_VAR: LinkResolver = LinkResolver::Default;
}

/// Markdown image resolver.
///
/// This can be used to override image source resolution, by default the image URL or URI is passed as parsed to the [`image_view`].
///
/// Note that image downloads are blocked by default, you can enable this by using the [`image::img_limits`] property.
///
/// Sets the [`IMAGE_RESOLVER_VAR`].
///
/// [`image_view`]: fn@image_view
/// [`image::img_limits`]: fn@crate::widgets::image::img_limits
#[property(CONTEXT, default(IMAGE_RESOLVER_VAR))]
pub fn image_resolver(child: impl UiNode, resolver: impl IntoVar<ImageResolver>) -> impl UiNode {
    with_context_var(child, IMAGE_RESOLVER_VAR, resolver)
}

/// Markdown link resolver.
///
/// This can be used to override link resolution, by default only scroll links
///
/// Sets the [`LINK_RESOLVER_VAR`].
#[property(CONTEXT, default(LINK_RESOLVER_VAR))]
pub fn link_resolver(child: impl UiNode, resolver: impl IntoVar<LinkResolver>) -> impl UiNode {
    with_context_var(child, LINK_RESOLVER_VAR, resolver)
}

/// Markdown image resolver.
///
/// See [`IMAGE_RESOLVER_VAR`] for more details.
#[derive(Clone)]
pub enum ImageResolver {
    /// No extra resolution, just convert into [`ImageSource`].
    Default,
    /// Custom resolution.
    Resolve(Arc<dyn Fn(&str) -> ImageSource + Send + Sync>),
}
impl ImageResolver {
    /// Apply the text transform.
    pub fn resolve(&self, img: &str) -> ImageSource {
        match self {
            ImageResolver::Default => img.into(),
            ImageResolver::Resolve(r) => r(img),
        }
    }

    /// New [`Resolve`](Self::Resolve).
    pub fn new(fn_: impl Fn(&str) -> ImageSource + Send + Sync + 'static) -> Self {
        ImageResolver::Resolve(Arc::new(fn_))
    }
}
impl Default for ImageResolver {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for ImageResolver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "ImgSourceResolver::")?;
        }
        match self {
            ImageResolver::Default => write!(f, "Default"),
            ImageResolver::Resolve(_) => write!(f, "Resolve(_)"),
        }
    }
}

/// Markdown link resolver.
///
/// See [`LINK_RESOLVER_VAR`] for more details.
#[derive(Clone)]
pub enum LinkResolver {
    /// Scroll `#anchor` links, ignore the rest.
    Default,
    /// Custom resolution.
    Resolve(Arc<dyn Fn(&str) -> LinkAction + Send + Sync>),
}
impl Default for LinkResolver {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for LinkResolver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "LinkResolver::")?;
        }
        match self {
            LinkResolver::Default => write!(f, "Default"),
            LinkResolver::Resolve(_) => write!(f, "Resolve(_)"),
        }
    }
}

/// Arguments for a custom [`LinkAction`].
#[derive(Debug, Clone)]
pub struct LinkActionArgs {
    /// Propagation handle of the event that activated the link.
    pub propagation: EventPropagationHandle,
}

/// Action that runs when a markdown link is clicked, or otherwise activated.
#[derive(Clone)]
pub struct LinkAction(pub Option<Arc<Mutex<dyn FnMut(&mut WidgetContext, &LinkActionArgs) + Send>>>);
impl fmt::Debug for LinkAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkAction(_)")
    }
}
impl LinkAction {
    /// New `None`.
    pub fn none() -> Self {
        Self(None)
    }

    /// Scroll to make the markdown generated widget associated with the `anchor` text visible.
    ///
    /// The anchor is resolved in the parent [`markdown!`] context and the [`SCROLL_TO_CMD`] command used to request the scroll.
    ///
    /// [`markdown!`]: mod@markdown
    /// [`SCROLL_TO_CMD`]: crate::widgets::scroll::commands::SCROLL_TO_CMD
    pub fn scroll(anchor: &str, mode: crate::widgets::scroll::commands::ScrollToMode) -> Self {
        let anchor = anchor.to_owned();
        Self::new(|ctx, args| {
            todo!("find parents markdown! and scroll!, resolve anchor, request scroll");
        })
    }

    /// Open the *url* or file externally.
    ///
    /// Request is done as a command, `explorer` in Windows, `open` in Mac and `xdg-open` in Linux.
    pub fn open(url: &str) -> Self {
        let url = url.to_owned();

        let open = if cfg!(windows) {
            "explorer"
        } else if cfg!(target_vendor = "apple") {
            "open"
        } else {
            "xdg-open"
        };

        Self::new(move |_, args| {
            let ok = match std::process::Command::new(open).arg(&url).status() {
                Ok(c) => {
                    let ok = c.success();
                    if !ok {
                        tracing::error!("error opening \"{url}\", code: {c}");
                    }
                    ok
                }
                Err(e) => {
                    tracing::error!("error opening \"{url}\", {e}");
                    false
                }
            };

            if ok {
                args.propagation.stop();
            }
        })
    }

    /// New custom action.
    /// 
    /// The context is the link widget.
    pub fn new(handle: impl FnMut(&mut WidgetContext, &LinkActionArgs) + Send + 'static) -> Self {
        Self(Some(Arc::new(Mutex::new(handle))))
    }
}
