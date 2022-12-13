use std::fmt;
use std::sync::Arc;

use zero_ui_core::widget_info::WidgetInfo;

use crate::core::{image::ImageSource, task::http::Uri, text::ToText};
use crate::prelude::new_property::*;
use crate::widgets::scroll::WidgetInfoExt as _;

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
            if let Some(w) = ctx.info_tree.get(ctx.path.widget_id()) {
                if let Some(md) = w.self_and_ancestors().find(|w| w.is_markdown()) {
                    if let Some(target) = md.find_anchor(anchor) {
                        for scroll in target.ancestors().filter(|&a| a.is_scroll()) {
                            crate::widgets::scroll::commands::scroll_to(
                                ctx.events,
                                scroll.widget_id(),
                                target.widget_id(),
                                crate::widgets::scroll_wgt::commands::ScrollToMode::minimal(10),
                            );
                        }
                        return true;
                    }
                }
            }
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
                let ok = c.success() || (cfg!(windows) && c.code() == Some(1));
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

static ANCHOR_ID: StaticStateId<Text> = StaticStateId::new_unique();

pub(super) static MARKDOWN_INFO_ID: StaticStateId<()> = StaticStateId::new_unique();

/// Set a label that identifies the widget in the context of the parent markdown.
///
/// The anchor can be retried in the widget info using [`WidgetInfoExt::anchor`]. It is mostly used
/// by markdown links to find scroll targets.
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

/// Markdown extension methods for widget info.
pub trait WidgetInfoExt<'a> {
    /// Gets the [`anchor`].
    fn anchor(self) -> Option<&'a Text>;

    /// If this widget is a [`markdown!`].
    ///
    /// [`markdown!`]: mod@crate::widgets::markdown
    #[allow(clippy::wrong_self_convention)] // WidgetInfo is a reference.
    fn is_markdown(self) -> bool;

    /// Find descendant tagged by the given anchor.
    fn find_anchor(self, anchor: &str) -> Option<WidgetInfo<'a>>;
}
impl<'a> WidgetInfoExt<'a> for WidgetInfo<'a> {
    fn anchor(self) -> Option<&'a Text> {
        self.meta().get(&ANCHOR_ID)
    }

    fn is_markdown(self) -> bool {
        self.meta().contains(&MARKDOWN_INFO_ID)
    }

    fn find_anchor(self, anchor: &str) -> Option<WidgetInfo<'a>> {
        self.descendants().find(|d| d.anchor().map(|a| a == anchor).unwrap_or(false))
    }
}

/// Generate an anchor label for a header.
pub fn heading_anchor(header: &str) -> Text {
    header.chars().filter_map(slugify).collect::<String>().into()
}
fn slugify(c: char) -> Option<char> {
    if c.is_alphanumeric() || c == '-' || c == '_' {
        if c.is_ascii() {
            Some(c.to_ascii_lowercase())
        } else {
            Some(c)
        }
    } else if c.is_whitespace() && c.is_ascii() {
        Some('-')
    } else {
        None
    }
}
