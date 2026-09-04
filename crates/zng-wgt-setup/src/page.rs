//! Common setup pages.

mod welcome;
pub use welcome::WelcomePage;

mod install_dir;
pub use install_dir::InstallDirPage;

mod eula;
pub use eula::{EulaPage, EulaTxt};
