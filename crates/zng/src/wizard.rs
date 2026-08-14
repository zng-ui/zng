#![cfg(feature = "wizard")]

//! !!: TODO

pub use zng_wgt_wizard::{
    BACK_CMD, CANCEL_CMD, ContentFnArgs, FINISH_CMD, FooterFnArgs, HeaderFnArgs, NEXT_CMD, Page, PageArgs, PanelFnArgs, SideFnArgs, Wizard,
    content_fn, footer_fn, header_background_fn, header_fn, panel_fn, side_background_fn, side_fn,
};
