use std::fmt;
use std::sync::Arc;

use crate::core::{image::ImageSource, task::http::Uri, text::ToText};
use crate::prelude::new_property::*;

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
/// This can be used to expand or replace links.
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
    /// Resolve the image.
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
    /// No extra resolution, just pass the link provided.
    Default,
    /// Custom resolution.
    Resolve(Arc<dyn Fn(&str) -> Text + Send + Sync>),
}
impl LinkResolver {
    /// Resolve the link.
    pub fn resolve(&self, url: &str) -> Text {
        match self {
            Self::Default => url.to_text(),
            Self::Resolve(r) => r(url),
        }
    }

    /// New [`Resolve`](Self::Resolve).
    pub fn new(fn_: impl Fn(&str) -> Text + Send + Sync + 'static) -> Self {
        Self::Resolve(Arc::new(fn_))
    }
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
            Self::Default => write!(f, "Default"),
            Self::Resolve(_) => write!(f, "Resolve(_)"),
        }
    }
}

event! {
    /// Event raised by markdown links when clicked.
    pub static LINK_EVENT: LinkArgs;
}

event_property! {
    /// Markdown link click.
    pub fn link {
        event: LINK_EVENT,
        args: LinkArgs,
    }
}

event_args! {
    /// Arguments for the [`LINK_EVENT`].
    pub struct LinkArgs {
        /// Raw URL.
        pub url: Text,

        /// Link widget.
        pub link: InteractionPath,

        ..

        fn delivery_list(&self, delivery_list: &mut UpdateDeliveryList) {
            delivery_list.insert_path(self.link.as_path())
        }
    }
}

/// Default markdown link action.
///
/// Does [`try_scroll_link`] or [`try_open_link`].
pub fn try_default_link_action(ctx: &mut WidgetContext, args: &LinkArgs) -> bool {
    try_scroll_link(ctx, args) || try_open_link(args)
}

/// Try to scroll to the anchor, only workds if the `url` is in the format `#anchor`, the `ctx` is a [`markdown!`] or inside one,
/// and is also inside a [`scroll!`].
///
/// [`markdown!`]: crate::widgets::markdown
/// [`scroll!`]: crate::widgets::scroll
pub fn try_scroll_link(ctx: &mut WidgetContext, args: &LinkArgs) -> bool {
    if !args.propagation().is_stopped() {
        if let Some(anchor) = args.url.strip_prefix('#') {
            todo!()
        }
    }
    false
}

/// Try open link, only works if the `url` is valid or a file path, returns if suceeded and the event was handled.
pub fn try_open_link(args: &LinkArgs) -> bool {
    if !args.propagation().is_stopped() && args.url.parse::<Uri>().is_ok() {
        let open = if cfg!(windows) {
            "explorer"
        } else if cfg!(target_vendor = "apple") {
            "open"
        } else {
            "xdg-open"
        };

        let url = &args.url;
        let ok = match std::process::Command::new(open).arg(url.as_str()).status() {
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
            args.propagation().stop();
        }

        ok
    } else {
        false
    }
}

/// Label identifier for a markdown widget.
///
/// Is set by the [`anchor`] property in the widget info.
///
/// [`anchor`]: fn@anchor
pub static ANCHOR_ID: StaticStateId<Text> = StaticStateId::new_unique();

/// Set a [`ANCHOR_ID`] for the widget.
#[property(CONTEXT, default(""))]
pub fn anchor(child: impl UiNode, anchor: impl IntoVar<Text>) -> impl UiNode {
    #[ui_node(struct AnchorNode {
        child: impl UiNode,
        #[var] anchor: impl Var<Text>,
    })]
    impl UiNode for AnchorNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            if self.anchor.is_new(ctx) {
                ctx.updates.info();
            }
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            info.meta().set(&ANCHOR_ID, self.anchor.get());
            self.child.info(ctx, info);
        }
    }
    AnchorNode {
        child,
        anchor: anchor.into_var(),
    }
}
