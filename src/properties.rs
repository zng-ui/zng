//! Common widget properties.

mod align_;
mod attached;
pub mod background;
pub mod border;
pub mod button_theme;
mod capture_mouse_;
pub mod capture_only;
mod clip_to_bounds_;
mod cursor_;
pub mod drag_move;
pub mod events;
pub mod filters;
pub mod focus;
pub mod foreground;
mod hit_testable_;
mod margin_;
mod position_;
pub mod size;
pub mod states;
pub mod text_theme;
mod title_;
pub mod transform;
mod visibility_;

pub use align_::*;
pub use attached::*;
pub use capture_mouse_::*;
pub use clip_to_bounds_::*;
pub use cursor_::*;
pub use hit_testable_::*;
pub use margin_::*;
pub use position_::*;
pub use title_::*;
pub use visibility_::*;
