#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Setup wizard widgets.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_wgt_wizard::Wizard;

use zng_ext_l10n::l10n;
use zng_wgt::prelude::*;

zng_wgt::enable_widget_macros!();

pub mod page;

/// Represents a setup operation wizard.
#[widget($crate::SetupWizard)]
pub struct SetupWizard(Wizard);
impl SetupWizard {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            finish_cmd_name = SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!("FINISH_CMD.name-install", "Install"),
                SetupOp::Update => l10n!("FINISH_CMD.name-update", "Update"),
                SetupOp::Repair => l10n!("FINISH_CMD.name-repair", "Repair"),
                SetupOp::Uninstall => l10n!("FINISH_CMD.name-uninstall", "Uninstall"),
            });
        }
    }
}

context_var! {
    /// Operation the setup wizard is managing.
    ///
    /// Is `Install` by default.
    pub static SETUP_OP_VAR: SetupOp = SetupOp::Install;

    /// Unique identifier of the product being setup.
    ///
    /// Is the [`About::app_id`] by default.
    ///
    /// [`About::app_id`]: zng_env::About::app_id
    pub static APP_ID_VAR: Txt = zng_env::about().app_id.clone();

    /// Display name of the product being setup.
    ///
    /// Is the [`About::app`] by default.
    ///
    /// [`About::app`]: zng_env::About::app
    pub static APP_NAME_VAR: Txt = zng_env::about().app.clone();

    /// Display version of the product being setup.
    ///
    /// Is the [`About::version`] by default.
    ///
    /// [`About::version`]: zng_env::About::version
    pub static APP_VERSION_VAR: Txt = zng_env::about().version.to_txt();

    /// Display name of the app producer.
    ///
    /// Is the [`About::org`] by default.
    ///
    /// [`About::org`]: zng_env::About::org
    pub static APP_ORG_VAR: Txt = zng_env::about().version.to_txt();

    /// Display version of the product that is already installed.
    ///
    /// Is [`APP_VERSION_VAR`] by default.
    pub static INSTALLED_VERSION_VAR: Txt = APP_VERSION_VAR;
}

/// Defines what operation the wizard is configuring and running.
///
/// This property sets the [`SETUP_OP_VAR`].
#[property(CONTEXT, default(SETUP_OP_VAR), widget_impl(SetupWizard))]
pub fn setup_op(child: impl IntoUiNode, setup_op: impl IntoVar<SetupOp>) -> UiNode {
    with_context_var(child, SETUP_OP_VAR, setup_op)
}

/// Defines the unique identifier of the product being setup.
///
/// This must be a globally unique identifier. A reverse DNS name is recommended.
///
/// The default value is [`zng::env::about().app_id`], if the setup exe is also the product main exe
/// this does not need to be set.
///
/// This property sets the [`APP_ID_VAR`].
///
/// [`zng::env::about().app_id`]: zng_env::About::app_id
#[property(CONTEXT, default(APP_ID_VAR), widget_impl(SetupWizard))]
pub fn app_id(child: impl IntoUiNode, id: impl IntoVar<Txt>) -> UiNode {
    with_context_var(child, APP_ID_VAR, id)
}

/// Defines the display name of the product being setup.
///
/// The default value is [`zng::env::about().app`], if the setup exe is also the product main exe
/// this does not need to be set.
///
/// This property sets the [`APP_NAME_VAR`].
///
/// [`zng::env::about().app`]: zng_env::About::app
#[property(CONTEXT, default(APP_NAME_VAR), widget_impl(SetupWizard))]
pub fn app_name(child: impl IntoUiNode, name: impl IntoVar<Txt>) -> UiNode {
    with_context_var(child, APP_NAME_VAR, name)
}

/// Defines the display version of the product being setup.
///
/// The default value is [`zng::env::about().version`], if the setup exe is also the product main exe
/// this does not need to be set.
///
/// This property sets the [`APP_VERSION_VAR`].
///
/// [`zng::env::about().version`]: zng_env::About::version
#[property(CONTEXT, default(APP_VERSION_VAR), widget_impl(SetupWizard))]
pub fn app_version(child: impl IntoUiNode, version: impl IntoVar<Txt>) -> UiNode {
    with_context_var(child, APP_VERSION_VAR, version)
}

/// Defines the display name of the producer of the product being setup.
///
/// The default value is [`zng::env::about().org`], if the setup exe is also the product main exe
/// this does not need to be set.
///
/// This property sets the [`APP_ORG_VAR`].
///
/// [`zng::env::about().org`]: zng_env::About::org
#[property(CONTEXT, default(APP_ORG_VAR), widget_impl(SetupWizard))]
pub fn app_org(child: impl IntoUiNode, org: impl IntoVar<Txt>) -> UiNode {
    with_context_var(child, APP_ORG_VAR, org)
}

/// Defines the display version of the product that is already installed.
///
/// The default value is [`APP_VERSION_VAR`], must be changed for [`SetupOp::Update`].
///
/// This property sets the [`INSTALLED_VERSION_VAR`].
#[property(CONTEXT, default(INSTALLED_VERSION_VAR), widget_impl(SetupWizard))]
pub fn installed_version(child: impl IntoUiNode, version: impl IntoVar<Txt>) -> UiNode {
    with_context_var(child, INSTALLED_VERSION_VAR, version)
}

/// Represents the operation a [`SetupWizard!`] is managing.
///
/// The [`SETUP_OP_VAR`] defines the operation in a context.
///
/// [`SetupWizard!`]: struct@SetupWizard
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum SetupOp {
    /// Fresh install operation.
    Install,
    /// Replacing install operation.
    Update,
    /// Replacing install operation with the same version.
    Repair,
    /// Uninstall operation.
    Uninstall,
}
