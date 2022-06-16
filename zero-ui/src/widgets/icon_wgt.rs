use crate::prelude::new_widget::*;

/// Render icons defined as glyphs in an icon font.
///
/// Note that no icons are embedded in this crate directly, you can manually create a [`GlyphIcon`]
/// or use an icon set crate. See the [`zero-ui-material-icons`] crate, it provides documented constants for
/// each icon in the fonts.
#[widget($crate::widgets::icon)]
pub mod icon {
    use super::*;

    properties! {
        /// The glyph icon.
        icon(impl IntoVar<GlyphIcon>);

        /// Icon size, best sizes are 18, 24, 36 or 48dip, default is 24dip.
        ///
        /// This is a single [`Length`] value that sets the "font size" of the icon glyph.
        theme::icon_size;

        /// Icon color.
        theme::icon_color as color;

        // TODO, this panics in the `#[widget]`
        // /// Disabled color.
        // when self.is_disabled {
        //     color = theme::disabled::IconColorVar;
        // }
    }

    fn new_child(icon: impl IntoVar<GlyphIcon>) -> impl UiNode {
        nodes::icon(icon)
    }

    /// Nodes that implement the icon rendering.
    pub mod nodes {
        use super::*;
        use theme::{IconColorVar, IconSizeVar};

        use std::cell::RefCell;

        /// Renders the `icon` using the contextual [`theme::IconSizeVar`] and [`theme::IconColorVar`].
        pub fn icon(icon: impl IntoVar<GlyphIcon>) -> impl UiNode {
            #[derive(Default)]
            struct ShapedIcon {
                face: Option<FontFaceRef>,
                font: Option<FontRef>,

                glyph: GlyphInstance,
                baseline: Px,
                bounds: PxSize,
            }

            impl ShapedIcon {
                fn update_glyph(&mut self, glyph: &GlyphSource) {
                    self.glyph.index = 0;
                    self.bounds = PxSize::zero();
                    self.baseline = Px(0);

                    if let Some(face) = &self.face {
                        match glyph {
                            GlyphSource::Glyph(id) => {
                                self.glyph.index = *id;
                            }
                            GlyphSource::Code(c) => {
                                if let Some(id) = face.glyph_for_char(*c) {
                                    self.glyph.index = id;
                                }
                            }
                            GlyphSource::Ligature(c) => todo!(),
                        }
                    }
                }

                fn layout(&mut self, vars: &VarsRead, metrics: &LayoutMetrics) -> PxSize {
                    if let Some(face) = &self.face {
                        let size = IconSizeVar::get(vars).layout(metrics.for_x(), |m| IconSizeVar::default_value().layout(m, |_| Px(24)));

                        if self.font.as_ref().map(|f| f.size() != size).unwrap_or(true) {
                            self.font = Some(face.sized(size, vec![]));
                        }

                        if let Some(font) = &self.font {
                            if self.glyph.index != 0 && self.bounds == PxSize::zero() {
                                if let Ok(shape_glyph) = font.shape_glyph(self.glyph.index) {
                                    (self.glyph, self.bounds, self.baseline) = shape_glyph;
                                } else {
                                    self.glyph.index = 0;
                                }
                            }
                        }
                    }
                    self.bounds
                }
            }
            struct IconNode<I> {
                icon: I,
                shaped: RefCell<ShapedIcon>,
            }
            #[impl_ui_node(none)]
            impl<I: Var<GlyphIcon>> UiNode for IconNode<I> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let s = self.shaped.get_mut();
                    let icon = self.icon.get(ctx.vars);
                    s.face = ctx.services.fonts().get_normal(&icon.font, &lang!(und));
                    s.update_glyph(&icon.glyph);
                }

                fn deinit(&mut self, _: &mut WidgetContext) {
                    let s = self.shaped.get_mut();
                    s.face = None;
                    s.font = None;
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
                    subs.vars(ctx).var(&self.icon).var(&IconSizeVar::new()).var(&IconColorVar::new());
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if let Some(ico) = self.icon.get_new(ctx.vars) {
                        let s = self.shaped.get_mut();

                        if let Some(face) = &s.face {
                            if face.family_name() != &ico.font {
                                s.face = ctx.services.fonts().get_normal(&ico.font, &lang!(und));
                                s.font = None;
                            }
                        }

                        s.update_glyph(&self.icon.get(ctx.vars).glyph);

                        ctx.updates.layout_and_render();
                    }

                    if IconSizeVar::is_new(ctx) {
                        self.shaped.get_mut().font = None;
                        ctx.updates.layout();
                    }
                    if IconColorVar::is_new(ctx) {
                        ctx.updates.render();
                    }
                }

                fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                    self.shaped.borrow_mut().layout(ctx.vars, ctx.metrics)
                }

                fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                    let s = self.shaped.get_mut();
                    let r = s.layout(ctx.vars, ctx.metrics);
                    wl.set_baseline(s.baseline);
                    r
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    let s = self.shaped.borrow();
                    if s.glyph.index != 0 {
                        if let Some(font) = &s.font {
                            let color = *IconColorVar::get(ctx);
                            frame.push_text(
                                PxRect::from_size(s.bounds),
                                &[s.glyph],
                                font,
                                color.into(),
                                FontSynthesis::DISABLED,
                                FontAntiAliasing::Default,
                            );
                        }
                    }
                }
            }
            IconNode {
                icon: icon.into_var(),
                shaped: RefCell::default(),
            }
        }
    }

    /// Identifies an icon glyph in the font set.
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    pub enum GlyphSource {
        /// Direct reference to the glyph.
        Glyph(GlyphIndex),
        /// Code "char" that is mapped to the glyph.
        Code(char),
        /// String that resolves to the glyph due to the default ligature config of the font.
        Ligature(Text),
    }
    impl_from_and_into_var! {
        fn from(id: GlyphIndex) -> GlyphSource {
            GlyphSource::Glyph(id)
        }
        fn from(code: char) -> GlyphSource {
            GlyphSource::Code(code)
        }
        fn from(ligature: &'static str) -> GlyphSource {
            Text::from_static(ligature).into()
        }
        fn from(ligature: Text) -> GlyphSource {
            GlyphSource::Ligature(ligature)
        }
    }

    /// Represents an icon glyph and font.
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    pub struct GlyphIcon {
        /// Icon set font name.
        pub font: FontName,
        /// Icon glyph.
        pub glyph: GlyphSource,
    }
    impl GlyphIcon {
        /// New icon.
        pub fn new(font: impl Into<FontName>, glyph: impl Into<GlyphSource>) -> Self {
            GlyphIcon {
                font: font.into(),
                glyph: glyph.into(),
            }
        }
    }
    impl_from_and_into_var! {
        fn from<F: Into<FontName> + Clone, G: Into<GlyphSource> + Clone>((name, glyph): (F, G)) -> GlyphIcon {
            GlyphIcon::new(name, glyph)
        }
    }

    /// Context variables and properties that affect icons.
    pub mod theme {
        use super::*;

        context_var! {
            /// Defines the size of an icon.
            ///
            /// Default is `24.dip()`.
            pub struct IconSizeVar: Length = 24.dip();

            /// Defines the color of an icon.
            ///
            /// Default `colors::WHITE`.
            pub struct IconColorVar: Rgba = colors::WHITE;
        }

        /// Sets the [`IconSizeVar`] that affects all icons inside the widget.
        #[property(context, default(IconSizeVar))]
        pub fn icon_size(child: impl UiNode, size: impl IntoVar<Length>) -> impl UiNode {
            with_context_var(child, IconSizeVar, size)
        }

        /// Sets the [`IconColorVar`] that affects all icons inside the widget.
        #[property(context, default(IconColorVar))]
        pub fn icon_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, IconColorVar, color)
        }

        /// Icon disabled values.
        pub mod disabled {
            use super::*;

            context_var! {
                /// Defines the color of a disabled icon.
                ///
                /// Default `colors::WHITE.darken(40.pct()`.
                pub struct IconColorVar: Rgba = colors::WHITE.darken(40.pct());
            }

            /// Sets the [`IconColorVar`] that affects all disabled icons inside the widget.
            #[property(context, default(IconColorVar))]
            pub fn icon_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, IconColorVar, color)
            }
        }
    }
}

/// Short form [`icon!`].
///
/// [`icon!`]: mod::icon;
pub fn icon(ico: impl IntoVar<icon::GlyphIcon>) -> impl Widget {
    icon!(icon = ico)
}
