use path_absolutize::*;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use zero_ui_core::focus::FOCUS;
use zero_ui_core::task::http::Uri;
use zero_ui_core::widget_info::WidgetInfo;
use zero_ui_core::window::TransformChangedArgs;

use crate::core::{focus::WidgetInfoFocusExt as _, image::ImageSource, text::ToText};
use crate::prelude::new_property::*;
use crate::widgets::scroll::commands::ScrollToMode;
use crate::widgets::scroll::WidgetInfoExt as _;

use super::Markdown;

context_var! {
    /// Markdown image resolver.
    pub static IMAGE_RESOLVER_VAR: ImageResolver = ImageResolver::Default;

    /// Markdown link resolver.
    pub static LINK_RESOLVER_VAR: LinkResolver = LinkResolver::Default;

    /// Scroll mode used by anchor links.
    pub static LINK_SCROLL_MODE_VAR: ScrollToMode = ScrollToMode::minimal(10);
}

/// Markdown image resolver.
///
/// This can be used to override image source resolution, by default the image URL or URI is passed as parsed to the [`image_fn`].
///
/// Note that image downloads are blocked by default, you can enable this by using the [`image::img_limits`] property.
///
/// Sets the [`IMAGE_RESOLVER_VAR`].
///
/// [`image_fn`]: fn@crate::widgets::markdown::image_fn
/// [`image::img_limits`]: fn@crate::widgets::image::img_limits
#[property(CONTEXT, default(IMAGE_RESOLVER_VAR), widget_impl(Markdown))]
pub fn image_resolver(child: impl UiNode, resolver: impl IntoVar<ImageResolver>) -> impl UiNode {
    with_context_var(child, IMAGE_RESOLVER_VAR, resolver)
}

/// Markdown link resolver.
///
/// This can be used to expand or replace links.
///
/// Sets the [`LINK_RESOLVER_VAR`].
#[property(CONTEXT, default(LINK_RESOLVER_VAR), widget_impl(Markdown))]
pub fn link_resolver(child: impl UiNode, resolver: impl IntoVar<LinkResolver>) -> impl UiNode {
    with_context_var(child, LINK_RESOLVER_VAR, resolver)
}

/// Scroll-to mode used by anchor links.
#[property(CONTEXT, default(LINK_SCROLL_MODE_VAR), widget_impl(Markdown))]
pub fn link_scroll_mode(child: impl UiNode, mode: impl IntoVar<ScrollToMode>) -> impl UiNode {
    with_context_var(child, LINK_SCROLL_MODE_VAR, mode)
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
impl PartialEq for ImageResolver {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // can only fail by returning `false` in some cases where the value pointer is actually equal.
            // see: https://github.com/rust-lang/rust/issues/103763
            //
            // we are fine with this, worst case is just an extra var update
            #[allow(clippy::vtable_address_comparisons)]
            (Self::Resolve(l0), Self::Resolve(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
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
    Resolve(Arc<dyn Fn(&str) -> Txt + Send + Sync>),
}
impl LinkResolver {
    /// Resolve the link.
    pub fn resolve(&self, url: &str) -> Txt {
        match self {
            Self::Default => url.to_text(),
            Self::Resolve(r) => r(url),
        }
    }

    /// New [`Resolve`](Self::Resolve).
    pub fn new(fn_: impl Fn(&str) -> Txt + Send + Sync + 'static) -> Self {
        Self::Resolve(Arc::new(fn_))
    }

    /// Resolve file links relative to `base`.
    ///
    /// The path is also absolutized, but not canonicalized.
    pub fn base_dir(base: impl Into<PathBuf>) -> Self {
        let base = base.into();
        Self::new(move |url| {
            if !url.starts_with('#') && url.parse::<Uri>().is_err() {
                if let Ok(path) = url.parse::<PathBuf>() {
                    if let Ok(path) = base.join(path).absolutize() {
                        return path.display().to_text();
                    }
                }
            }
            url.to_text()
        })
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
impl PartialEq for LinkResolver {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // can only fail by returning `false` in some cases where the value pointer is actually equal.
            // see: https://github.com/rust-lang/rust/issues/103763
            //
            // we are fine with this, worst case is just an extra var update
            #[allow(clippy::vtable_address_comparisons)]
            (Self::Resolve(l0), Self::Resolve(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
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
        pub url: Txt,

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
pub fn try_default_link_action(args: &LinkArgs) -> bool {
    try_scroll_link(args) || try_open_link(args)
}

/// Handle `url` in the format `#anchor`, by scrolling and focusing the anchor.
///
/// If the anchor is found scrolls to it and moves focus to the `#anchor` widget,
/// or the first focusable descendant of it, or the markdown widget or the first focusable ancestor of it.
///
/// Note that the request is handled even if the anchor is not found.
pub fn try_scroll_link(args: &LinkArgs) -> bool {
    if args.propagation().is_stopped() {
        return false;
    }
    // Note: file names can start with #, but we are choosing to always interpret URLs with this prefix as an anchor.
    if let Some(anchor) = args.url.strip_prefix('#') {
        let tree = WINDOW.widget_tree();
        if let Some(md) = tree.get(WIDGET.id()).and_then(|w| w.self_and_ancestors().find(|w| w.is_markdown())) {
            if let Some(target) = md.find_anchor(anchor) {
                // scroll-to
                for scroll in target.ancestors().filter(|a| a.is_scroll()) {
                    crate::widgets::scroll::commands::scroll_to(scroll.id(), target.id(), LINK_SCROLL_MODE_VAR.get());
                }

                // focus
                if let Some(focus) = target
                    .into_focus_info(false, false)
                    .self_and_descendants()
                    .find(|w| w.is_focusable())
                    .or_else(|| md.into_focus_info(false, false).self_and_ancestors().find(|w| w.is_focusable()))
                {
                    FOCUS.focus_widget(focus.info().id(), false);
                }
            }
        }
        args.propagation().stop();
        return true;
    }

    false
}

/// Try open link, only works if the `url` is valid or a file path, returns if the confirm tool-tip is visible.
pub fn try_open_link(args: &LinkArgs) -> bool {
    use crate::prelude::*;

    if args.propagation().is_stopped() {
        return false;
    }

    enum Link {
        Url(Uri),
        Path(PathBuf),
    }

    let link = if let Ok(url) = args.url.parse() {
        Link::Url(url)
    } else if let Ok(path) = args.url.parse() {
        Link::Path(path)
    } else {
        return false;
    };

    let popup_id = WidgetId::new_unique();

    let url = args.url.clone();

    #[derive(Clone, Debug, PartialEq)]
    enum Status {
        Pending,
        Ok,
        Err,
        Cancel,
    }
    let status = var(Status::Pending);

    let open_time = Instant::now();

    let popup = Container! {
        id = popup_id;

        padding = (2, 4);
        corner_radius = 2;
        drop_shadow = (2, 2), 2, colors::BLACK.with_alpha(50.pct());
        align = Align::TOP_LEFT;

        #[easing(200.ms())]
        opacity = 0.pct();
        #[easing(200.ms())]
        offset = (0, -10);

        background_color = color_scheme_map(colors::BLACK.with_alpha(90.pct()), colors::WHITE.with_alpha(90.pct()));

        when *#{status.clone()} == Status::Pending {
            opacity = 100.pct();
            offset = (0, 0);
        }
        when *#{status} == Status::Err {
            background_color = color_scheme_map(colors::DARK_RED.with_alpha(90.pct()), colors::PINK.with_alpha(90.pct()));
        }

        child = Stack! {
            direction = StackDirection::left_to_right();
            children = ui_vec![
                Button! {
                    style_fn = button::LinkStyle!();

                    focus_on_init = true;

                    child = Text!(url);
                    text::underline_skip = UnderlineSkip::SPACES;

                    on_blur = async_hn_once!(status, |_| {
                        if status.get() != Status::Pending {
                            return;
                        }

                        status.set(Status::Cancel);
                        task::deadline(200.ms()).await;

                        LAYERS.remove(popup_id);
                    });
                    on_move = async_hn!(status, |args: TransformChangedArgs| {
                        if status.get() != Status::Pending || args.timestamp().duration_since(open_time) < 300.ms() {
                            return;
                        }

                        status.set(Status::Cancel);
                        task::deadline(200.ms()).await;

                        LAYERS.remove(popup_id);
                    });

                    on_click = async_hn_once!(status, |args: ClickArgs| {
                        if status.get() != Status::Pending || args.timestamp().duration_since(open_time) < 300.ms() {
                            return;
                        }

                        args.propagation().stop();

                        let url = match link {
                            Link::Url(u) => u.to_string(),
                            Link::Path(p) => {
                                match p.canonicalize() {
                                    Ok(p) => p.display().to_string(),
                                    Err(e) => {
                                        tracing::error!("error canonicalizing \"{}\", {e}", p.display());
                                        return;
                                    }
                                }
                            }
                        };

                        let open = if cfg!(windows) {
                            "explorer"
                        } else if cfg!(target_vendor = "apple") {
                            "open"
                        } else {
                            "xdg-open"
                        };
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

                        status.set(if ok { Status::Ok } else { Status::Err });
                        task::deadline(200.ms()).await;

                        LAYERS.remove(popup_id);
                    });
                },
                Text!(" 🡵"),
            ];
        }
    };

    LAYERS.insert_anchored(
        LayerIndex::ADORNER,
        args.link.widget_id(),
        AnchorMode::none()
            .with_transform(layers::AnchorOffset::out_bottom())
            .with_viewport_bound(true),
        popup,
    );

    true
}

static ANCHOR_ID: StaticStateId<Txt> = StaticStateId::new_unique();

pub(super) static MARKDOWN_INFO_ID: StaticStateId<()> = StaticStateId::new_unique();

/// Set a label that identifies the widget in the context of the parent markdown.
///
/// The anchor can be retried in the widget info using [`WidgetInfoExt::anchor`]. It is mostly used
/// by markdown links to find scroll targets.
#[property(CONTEXT, default(""))]
pub fn anchor(child: impl UiNode, anchor: impl IntoVar<Txt>) -> impl UiNode {
    let anchor = anchor.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&anchor);
        }
        UiNodeOp::Info { info } => {
            info.set_meta(&ANCHOR_ID, anchor.get());
        }
        _ => {}
    })
}

/// Markdown extension methods for widget info.
pub trait WidgetInfoExt {
    /// Gets the [`anchor`].
    ///
    /// [`anchor`]: fn@anchor
    fn anchor(&self) -> Option<&Txt>;

    /// If this widget is a [`Markdown!`].
    ///
    /// [`Markdown!`]: struct@crate::widgets::Markdown
    fn is_markdown(&self) -> bool;

    /// Find descendant tagged by the given anchor.
    fn find_anchor(&self, anchor: &str) -> Option<WidgetInfo>;
}
impl WidgetInfoExt for WidgetInfo {
    fn anchor(&self) -> Option<&Txt> {
        self.meta().get(&ANCHOR_ID)
    }

    fn is_markdown(&self) -> bool {
        self.meta().contains(&MARKDOWN_INFO_ID)
    }

    fn find_anchor(&self, anchor: &str) -> Option<WidgetInfo> {
        self.descendants().find(|d| d.anchor().map(|a| a == anchor).unwrap_or(false))
    }
}

/// Generate an anchor label for a header.
pub fn heading_anchor(header: &str) -> Txt {
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
