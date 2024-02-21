#![doc = include_str!("../../zero-ui-app/README.md")]
//!
//! Webrender debug flags property for use with `zero-ui-view` view-process.

pub use webrender_api::DebugFlags;

use zero_ui_view_api::api_extension::{ApiExtensionId, ApiExtensionPayload};
use zero_ui_wgt::prelude::*;

#[property(CONTEXT, default(RendererDebug::disabled()))]
pub fn renderer_debug(child: impl UiNode, debug: impl IntoVar<RendererDebug>) -> impl UiNode {
    // !! TODO, on window init
    /*

    {
                let mut exts = vec![];
                self.vars.renderer_debug().with(|d| d.push_extension(&mut exts));
                exts
            }
     */

    let debug = debug.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&debug);
        }
        UiNodeOp::Update { .. } => {
            if let Some(dbg) = debug.get_new() {
                // !!: TODO
                // if let Some(view) = &self.window {
                //     if let Some(key) = dbg.extension_id() {
                //         let _ = view.renderer().render_extension::<_, ()>(key, dbg);
                //     }
                // }
            }
        }
        _ => {}
    })
}

/// Webrender renderer debug flags and profiler UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RendererDebug {
    /// Debug flags.
    #[serde(with = "serde_debug_flags")]
    pub flags: DebugFlags,
    /// Profiler UI rendered when [`DebugFlags::PROFILER_DBG`] is set.
    ///
    /// # Syntax
    ///
    /// Comma-separated list of of tokens with trailing and leading spaces trimmed.
    /// Each tokens can be:
    /// - A counter name with an optional prefix. The name corresponds to the displayed name.
    ///   - By default (no prefix) the counter is shown as average + max over half a second.
    ///   - With a '#' prefix the counter is shown as a graph.
    ///   - With a '*' prefix the counter is shown as a change indicator.
    ///   - Some special counters such as GPU time queries have specific visualizations ignoring prefixes.
    /// - A preset name to append the preset to the UI.
    /// - An empty token to insert a bit of vertical space.
    /// - A '|' token to start a new column.
    /// - A '_' token to start a new row.
    ///
    /// # Preset & Counter Names
    ///
    /// * `"Default"`: `"FPS,|,Slow indicators,_,Time graphs,|,Frame times, ,Transaction times, ,Frame stats, ,Memory, ,Interners,_,GPU time queries,_,Paint phase graph"`
    /// * `"Compact"`: `"FPS, ,Frame times, ,Frame stats"`
    ///
    /// See the `webrender/src/profiler.rs` file for more details and more counter names.
    pub profiler_ui: String,
}
impl Default for RendererDebug {
    /// Disabled
    fn default() -> Self {
        Self::disabled()
    }
}
impl RendererDebug {
    /// Default mode, no debugging enabled.
    pub fn disabled() -> Self {
        Self {
            flags: DebugFlags::empty(),
            profiler_ui: String::new(),
        }
    }

    /// Enable profiler UI rendering.
    pub fn profiler(ui: impl Into<String>) -> Self {
        Self {
            flags: DebugFlags::PROFILER_DBG,
            profiler_ui: ui.into(),
        }
    }

    /// Custom flags with no UI string.
    pub fn flags(flags: DebugFlags) -> Self {
        Self {
            flags,
            profiler_ui: String::new(),
        }
    }

    /// If no flag nor profiler UI are set.
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty() && self.profiler_ui.is_empty()
    }

    fn extension_id(&self) -> Option<ApiExtensionId> {
        zero_ui_app::view_process::VIEW_PROCESS
            .extension_id("zero-ui-view.webrender_debug")
            .ok()
            .flatten()
    }

    fn push_extension(&self, exts: &mut Vec<(ApiExtensionId, ApiExtensionPayload)>) {
        if !self.is_empty() {
            if let Some(id) = self.extension_id() {
                exts.push((id, ApiExtensionPayload::serialize(self).unwrap()));
            }
        }
    }
}
impl_from_and_into_var! {
    fn from(profiler_default: bool) -> RendererDebug {
        if profiler_default {
            Self::profiler("Default")
        } else {
            Self::disabled()
        }
    }

    fn from(profiler: &str) -> RendererDebug {
        Self::profiler(profiler)
    }

    fn from(profiler: Txt) -> RendererDebug {
        Self::profiler(profiler)
    }

    fn from(flags: DebugFlags) -> RendererDebug {
        Self::flags(flags)
    }
}

/// Named DebugFlags in JSON serialization.
mod serde_debug_flags {
    use super::*;

    use serde::*;

    bitflags::bitflags! {
        #[repr(C)]
        #[derive(Default, Deserialize, Serialize)]
        #[serde(transparent)]
        struct DebugFlagsRef: u32 {
            const PROFILER_DBG = DebugFlags::PROFILER_DBG.bits();
            const RENDER_TARGET_DBG = DebugFlags::RENDER_TARGET_DBG.bits();
            const TEXTURE_CACHE_DBG = DebugFlags::TEXTURE_CACHE_DBG.bits();
            const GPU_TIME_QUERIES = DebugFlags::GPU_TIME_QUERIES.bits();
            const GPU_SAMPLE_QUERIES = DebugFlags::GPU_SAMPLE_QUERIES.bits();
            const DISABLE_BATCHING = DebugFlags::DISABLE_BATCHING.bits();
            const EPOCHS = DebugFlags::EPOCHS.bits();
            const ECHO_DRIVER_MESSAGES = DebugFlags::ECHO_DRIVER_MESSAGES.bits();
            const SHOW_OVERDRAW = DebugFlags::SHOW_OVERDRAW.bits();
            const GPU_CACHE_DBG = DebugFlags::GPU_CACHE_DBG.bits();
            const TEXTURE_CACHE_DBG_CLEAR_EVICTED = DebugFlags::TEXTURE_CACHE_DBG_CLEAR_EVICTED.bits();
            const PICTURE_CACHING_DBG = DebugFlags::PICTURE_CACHING_DBG.bits();
            const PRIMITIVE_DBG = DebugFlags::PRIMITIVE_DBG.bits();
            const ZOOM_DBG = DebugFlags::ZOOM_DBG.bits();
            const SMALL_SCREEN = DebugFlags::SMALL_SCREEN.bits();
            const DISABLE_OPAQUE_PASS = DebugFlags::DISABLE_OPAQUE_PASS.bits();
            const DISABLE_ALPHA_PASS = DebugFlags::DISABLE_ALPHA_PASS.bits();
            const DISABLE_CLIP_MASKS = DebugFlags::DISABLE_CLIP_MASKS.bits();
            const DISABLE_TEXT_PRIMS = DebugFlags::DISABLE_TEXT_PRIMS.bits();
            const DISABLE_GRADIENT_PRIMS = DebugFlags::DISABLE_GRADIENT_PRIMS.bits();
            const OBSCURE_IMAGES = DebugFlags::OBSCURE_IMAGES.bits();
            const GLYPH_FLASHING = DebugFlags::GLYPH_FLASHING.bits();
            const SMART_PROFILER = DebugFlags::SMART_PROFILER.bits();
            const INVALIDATION_DBG = DebugFlags::INVALIDATION_DBG.bits();
            const PROFILER_CAPTURE = DebugFlags::PROFILER_CAPTURE.bits();
            const FORCE_PICTURE_INVALIDATION = DebugFlags::FORCE_PICTURE_INVALIDATION.bits();
            const WINDOW_VISIBILITY_DBG = DebugFlags::WINDOW_VISIBILITY_DBG.bits();
            const RESTRICT_BLOB_SIZE = DebugFlags::RESTRICT_BLOB_SIZE.bits();
        }
    }
    impl From<DebugFlagsRef> for DebugFlags {
        fn from(value: DebugFlagsRef) -> Self {
            DebugFlags::from_bits(value.bits()).unwrap()
        }
    }
    impl From<DebugFlags> for DebugFlagsRef {
        fn from(value: DebugFlags) -> Self {
            DebugFlagsRef::from_bits(value.bits()).unwrap()
        }
    }

    pub fn serialize<S: serde::Serializer>(flags: &DebugFlags, serializer: S) -> Result<S::Ok, S::Error> {
        DebugFlagsRef::from(*flags).serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<DebugFlags, D::Error> {
        DebugFlagsRef::deserialize(deserializer).map(Into::into)
    }
}
