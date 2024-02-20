use std::borrow::Cow;

use zero_ui_app::{
    render::{FontSynthesis, FrameValueKey},
    widget::{
        border::{LineOrientation, LineStyle},
        node::{match_node, match_node_leaf, UiNode, UiNodeOp},
        WIDGET,
    },
};
use zero_ui_color::Rgba;
use zero_ui_ext_font::{Font, ShapedColoredGlyphs};
use zero_ui_ext_input::focus::FOCUS_CHANGED_EVENT;
use zero_ui_layout::{
    context::LAYOUT,
    unit::{Px, PxRect, PxSize},
};
use zero_ui_view_api::{config::FontAntiAliasing, display_list::FrameValue, webrender_api::GlyphInstance};
use zero_ui_wgt::prelude::*;

use crate::{
    TextOverflow, FONT_AA_VAR, FONT_COLOR_VAR, FONT_PALETTE_COLORS_VAR, FONT_PALETTE_VAR, IME_UNDERLINE_STYLE_VAR, OVERLINE_COLOR_VAR,
    OVERLINE_STYLE_VAR, SELECTION_COLOR_VAR, STRIKETHROUGH_COLOR_VAR, STRIKETHROUGH_STYLE_VAR, TEXT_EDITABLE_VAR, TEXT_OVERFLOW_VAR,
    UNDERLINE_COLOR_VAR, UNDERLINE_STYLE_VAR,
};

use super::TEXT;

/// An Ui node that renders the default underline visual using the parent [`LaidoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_strikethroughs`] node.
///
/// [`LaidoutText`]: super::LaidoutText
pub fn render_underlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&UNDERLINE_STYLE_VAR).sub_var_render(&UNDERLINE_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = TEXT.laidout();

            if !t.underlines.is_empty() {
                let style = UNDERLINE_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = UNDERLINE_COLOR_VAR.get();
                    for &(origin, width) in &t.underlines {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.underline_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the default IME preview underline visual using the parent [`LaidoutText`].
///
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_underlines`] node.
///
/// [`LaidoutText`]: super::LaidoutText
pub fn render_ime_preview_underlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&IME_UNDERLINE_STYLE_VAR).sub_var_render(&FONT_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = TEXT.laidout();

            if !t.ime_underlines.is_empty() {
                let style = IME_UNDERLINE_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = FONT_COLOR_VAR.get();
                    for &(origin, width) in &t.ime_underlines {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.ime_underline_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the default strikethrough visual using the parent [`LaidoutText`].
///
/// The lines are rendered after `child`, over it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_overlines`] node.
///
/// [`LaidoutText`]: super::LaidoutText
pub fn render_strikethroughs(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&STRIKETHROUGH_STYLE_VAR)
                .sub_var_render(&STRIKETHROUGH_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = TEXT.laidout();
            if !t.strikethroughs.is_empty() {
                let style = STRIKETHROUGH_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = STRIKETHROUGH_COLOR_VAR.get();
                    for &(origin, width) in &t.strikethroughs {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.strikethrough_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the default overline visual using the parent [`LaidoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
///
/// [`LaidoutText`]: super::LaidoutText
pub fn render_overlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&OVERLINE_STYLE_VAR).sub_var_render(&OVERLINE_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = TEXT.laidout();
            if !t.overlines.is_empty() {
                let style = OVERLINE_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = OVERLINE_COLOR_VAR.get();
                    for &(origin, width) in &t.overlines {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.overline_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the text selection background.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
///
/// [`LaidoutText`]: super::LaidoutText
pub fn render_selection(child: impl UiNode) -> impl UiNode {
    let mut is_focused = false;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&SELECTION_COLOR_VAR);
            is_focused = false;
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                let new_is_focused = args.is_focus_within(WIDGET.id());
                if is_focused != new_is_focused {
                    WIDGET.render();
                    is_focused = new_is_focused;
                }
            }
        }
        UiNodeOp::Render { frame } => {
            let r_txt = TEXT.resolved();

            if let Some(range) = r_txt.caret.selection_range() {
                let l_txt = TEXT.laidout();
                let txt = r_txt.segmented_text.text();

                let mut selection_color = SELECTION_COLOR_VAR.get();
                if !is_focused && !r_txt.selection_toolbar_is_open {
                    selection_color = selection_color.desaturate(100.pct());
                }

                for line_rect in l_txt.shaped_text.highlight_rects(range, txt) {
                    if !line_rect.size.is_empty() {
                        frame.push_color(line_rect, FrameValue::Value(selection_color));
                    }
                }
            };
        }
        _ => {}
    })
}

/// An UI node that renders the parent [`LaidoutText`].
///
/// This node renders the text only, decorators are rendered by other nodes.
///
/// This is the `Text!` widget inner most child node.
///
/// [`LaidoutText`]: super::LaidoutText
pub fn render_text() -> impl UiNode {
    #[derive(Clone, Copy, PartialEq)]
    struct RenderedText {
        version: u32,
        synthesis: FontSynthesis,
        color: Rgba,
        aa: FontAntiAliasing,
    }

    let mut reuse = None;
    let mut rendered = None;
    let mut color_key = None;

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render_update(&FONT_COLOR_VAR)
                .sub_var_render(&FONT_AA_VAR)
                .sub_var(&FONT_PALETTE_VAR)
                .sub_var(&FONT_PALETTE_COLORS_VAR);

            if FONT_COLOR_VAR.capabilities().contains(VarCapabilities::NEW) {
                color_key = Some(FrameValueKey::new_unique());
            }
        }
        UiNodeOp::Deinit => {
            color_key = None;
            reuse = None;
            rendered = None;
        }
        UiNodeOp::Update { .. } => {
            if FONT_PALETTE_VAR.is_new() || FONT_PALETTE_COLORS_VAR.is_new() {
                if let Some(t) = TEXT.try_laidout() {
                    if t.shaped_text.has_colored_glyphs() {
                        WIDGET.render();
                    }
                }
            }
        }
        UiNodeOp::Measure { desired_size, .. } => {
            let txt = TEXT.laidout();
            *desired_size = LAYOUT.constraints().fill_size_or(txt.shaped_text.size())
        }
        UiNodeOp::Layout { final_size, .. } => {
            // layout implemented in `layout_text`, it sets the size as an exact size constraint, we return
            // the size here for foreign nodes in the CHILD_LAYOUT+100 ..= CHILD range.
            let txt = TEXT.laidout();
            *final_size = LAYOUT.constraints().fill_size_or(txt.shaped_text.size())
        }
        UiNodeOp::Render { frame } => {
            let r = TEXT.resolved();
            let mut t = TEXT.layout();

            let lh = t.shaped_text.line_height();
            let clip = PxRect::from_size(t.shaped_text.align_size()).inflate(lh, lh); // clip inflated to allow some weird glyphs
            let color = FONT_COLOR_VAR.get();
            let color_value = if let Some(key) = color_key {
                key.bind(color, FONT_COLOR_VAR.is_animating())
            } else {
                FrameValue::Value(color)
            };

            let aa = FONT_AA_VAR.get();

            let rt = Some(RenderedText {
                version: t.shaped_text_version,
                synthesis: r.synthesis,
                color,
                aa,
            });
            if rendered != rt {
                rendered = rt;
                reuse = None;
            }

            t.render_info.transform = *frame.transform();
            t.render_info.scale_factor = frame.scale_factor();

            frame.push_reuse(&mut reuse, |frame| {
                if t.shaped_text.has_colored_glyphs() || t.overflow_suffix.as_ref().map(|o| o.has_colored_glyphs()).unwrap_or(false) {
                    let palette_query = FONT_PALETTE_VAR.get();
                    FONT_PALETTE_COLORS_VAR.with(|palette_colors| {
                        let mut push_font_glyphs = |font: &Font, glyphs, offset: Option<euclid::Vector2D<f32, Px>>| {
                            let mut palette = None;

                            match glyphs {
                                ShapedColoredGlyphs::Normal(glyphs) => {
                                    if let Some(offset) = offset {
                                        let mut glyphs = glyphs.to_vec();
                                        for g in &mut glyphs {
                                            g.point.x += offset.x;
                                            g.point.y += offset.y;
                                        }
                                        frame.push_text(clip, &glyphs, font, color_value, r.synthesis, aa);
                                    } else {
                                        frame.push_text(clip, glyphs, font, color_value, r.synthesis, aa);
                                    }
                                }
                                ShapedColoredGlyphs::Colored { point, glyphs, .. } => {
                                    for (index, color_i) in glyphs.iter() {
                                        let color = if let Some(color_i) = color_i {
                                            if let Some(i) = palette_colors.iter().position(|(ci, _)| *ci == color_i as u16) {
                                                palette_colors[i].1
                                            } else {
                                                // FontFace only parses colored glyphs if the font has at least one
                                                // palette, so it is safe to unwrap here
                                                let palette = palette
                                                    .get_or_insert_with(|| font.face().color_palettes().palette(palette_query).unwrap());

                                                // the font could have a bug and return an invalid palette index
                                                palette.colors.get(color_i).copied().unwrap_or(color)
                                            }
                                        } else {
                                            // color_i is None, meaning the base color.
                                            color
                                        };

                                        let mut g = GlyphInstance { point, index };
                                        if let Some(offset) = offset {
                                            g.point.x += offset.x;
                                            g.point.y += offset.y;
                                        }
                                        frame.push_text(clip, &[g], font, FrameValue::Value(color), r.synthesis, aa);
                                    }
                                }
                            }
                        };

                        match (&t.overflow, TEXT_OVERFLOW_VAR.get(), TEXT_EDITABLE_VAR.get()) {
                            (Some(o), TextOverflow::Truncate(_), false) => {
                                for glyphs in &o.included_glyphs {
                                    for (font, glyphs) in t.shaped_text.colored_glyphs_slice(glyphs.clone()) {
                                        push_font_glyphs(font, glyphs, None)
                                    }
                                }

                                if let Some(suf) = &t.overflow_suffix {
                                    let suf_offset = o.suffix_origin.to_vector().cast_unit();
                                    for (font, glyphs) in suf.colored_glyphs() {
                                        push_font_glyphs(font, glyphs, Some(suf_offset))
                                    }
                                }
                            }
                            _ => {
                                // no overflow truncating
                                for (font, glyphs) in t.shaped_text.colored_glyphs() {
                                    push_font_glyphs(font, glyphs, None)
                                }
                            }
                        }
                    });
                } else {
                    // no colored glyphs

                    let mut push_font_glyphs = |font: &Font, glyphs: Cow<[GlyphInstance]>| {
                        frame.push_text(clip, glyphs.as_ref(), font, color_value, r.synthesis, aa);
                    };

                    match (&t.overflow, TEXT_OVERFLOW_VAR.get(), !TEXT_EDITABLE_VAR.get()) {
                        (Some(o), TextOverflow::Truncate(_), false) => {
                            for glyphs in &o.included_glyphs {
                                for (font, glyphs) in t.shaped_text.glyphs_slice(glyphs.clone()) {
                                    push_font_glyphs(font, Cow::Borrowed(glyphs))
                                }
                            }

                            if let Some(suf) = &t.overflow_suffix {
                                let suf_offset = o.suffix_origin.to_vector().cast_unit();
                                for (font, glyphs) in suf.glyphs() {
                                    let mut glyphs = glyphs.to_vec();
                                    for g in &mut glyphs {
                                        g.point += suf_offset;
                                    }
                                    push_font_glyphs(font, Cow::Owned(glyphs))
                                }
                            }
                        }
                        _ => {
                            // no overflow truncating
                            for (font, glyphs) in t.shaped_text.glyphs() {
                                push_font_glyphs(font, Cow::Borrowed(glyphs))
                            }
                        }
                    }
                }
            });
        }
        UiNodeOp::RenderUpdate { update } => {
            TEXT.layout().render_info.transform = *update.transform();

            if let Some(key) = color_key {
                let color = FONT_COLOR_VAR.get();

                update.update_color(key.update(color, FONT_COLOR_VAR.is_animating()));

                let mut r = rendered.unwrap();
                r.color = color;
                rendered = Some(r);
            }
        }
        _ => {}
    })
}
