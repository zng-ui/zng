#![cfg(feature = "third_party")]

//! Third party licenses service and types.
//!
//! Rust projects depend on many crated with a variety of licenses, some of these licenses require that they must be
//! displayed in the app binary, usually in an "about" screen. This module can be used together with the [`zng_tp_licenses`]
//! crate to collect and bundle licenses of all used crates in your project.
//!
//! The [`LICENSES`] service serves as an aggregation center for licenses of multiple sources, the [`OPEN_LICENSES_CMD`]
//! can be implemented [`on_pre_event`] to show a custom licenses screen, or it can just be used to show the default
//! screen provided by the default app.
//!
//! # Bundle Setup
//!
//! Follow these steps to configure your crate and build workflow to collect and bundle crate licenses.
//!
//! ### Install `cargo about`
//!
//! To collect and bundle licenses in your project you must have [`cargo-about`] installed:
//!
//! ```console
//! cargo install cargo-about
//! ```
//!
//! Next add file `.cargo/about.toml` in your crate or workspace root:
//!
//! ```toml
//! # cargo about generate -c .cargo/about.toml --format json --workspace --all-features
//!
//! accepted = [
//!     "Apache-2.0",
//!     "MIT",
//!     "MPL-2.0",
//!     "Unicode-DFS-2016",
//!     "BSL-1.0",
//!     "BSD-2-Clause",
//!     "BSD-3-Clause",
//!     "ISC",
//!     "Zlib",
//!     "CC0-1.0",
//! ]
//!
//! ignore-build-dependencies = true
//! ignore-dev-dependencies = true
//! filter-noassertion = true
//! private = { ignore = true }
//! ```
//!
//! Next call the command to test and modify the `accepted` config:
//!
//! ```console
//! cargo about generate -c .cargo/about.toml --format json --workspace --all-features
//! ```
//!
//! If the command prints a JSON dump you are done with this step.
//!
//! ### Add `zng-tp-licenses`
//!
//! Next, add dependency to the [`zng_tp_licenses`] your crate `Cargo.toml`:
//!
//! ```toml
//! [package]
//! resolver = "2" # recommended, to not include "build" feature in the normal dependency.
//!
//! [features]
//! # Recommended, so you only bundle in release builds.
//! #
//! # Note that if you use a feature, don't forget to build with `--features bundle_licenses`.
//! bundle_licenses = ["dep:zng-tp-licenses"]
//!
//! [dependencies]
//! zng-tp-licenses = { version = "0.2.9", features = ["bundle"], optional = true }
//!
//! [build-dependencies]
//! zng-tp-licenses = { version = "0.2.0", features = ["build"], optional = true }
//! ```
//!
//! ### Implement Bundle
//!
//! Next, in your crates build script (`build.rs`) add:
//!
//! ```
//! fn main() {
//!     #[cfg(feature = "bundle_licenses")]
//!     {
//!         let licenses = zng_tp_licenses::collect_cargo_about("../.cargo/about.toml");
//!         zng_tp_licenses::write_bundle(&licenses);
//!     }
//! }
//! ```
//!
//! Implement a function that includes the bundle and decodes it. Register the function it in your app init code:
//!
//! ```
//! #[cfg(feature = "bundle_licenses")]
//! fn bundled_licenses() -> Vec<zng::third_party::LicenseUsed> {
//!     zng_tp_licenses::include_bundle!()
//! }
//!
//! # fn demo() {
//! # use zng::prelude::*;
//! APP.defaults().run(async {
//!     #[cfg(feature = "bundle_licenses")]
//!     zng::third_party::LICENSES.register(bundled_licenses);
//! });
//! # }
//! # fn main() { }
//! ```
//!
//! ### Review Licenses
//!
//! Call the [`OPEN_LICENSES_CMD`] in a test button, check if all the required licenses are present,
//! `cargo about` and `zng_tp_licenses` are a **best effort only** helpers, you must ensure that the generated results
//! meet yours or your company's legal obligations.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn review_licenses() -> impl UiNode {
//!     // zng::third_party::LICENSES.include_view_process().set(false);
//!
//!     Button!(zng::third_party::OPEN_LICENSES_CMD)
//! }
//! ```
//!
//! #### Limitations
//!
//! Only crate licenses reachable thought cargo metadata are included. Static linked libraries in `-sys` crates may
//! have required licenses that are not included. Other bundled resources such as fonts and images may also be licensed.
//!
//! The [`LICENSES`] service accepts multiple sources, so you can implement your own custom bundle, the [`zng_tp_licenses`]
//! crate provides helpers for manually encoding (compressing) licenses. See the `zng-view` build script for an example of
//! how to include more licenses.
//!
//! # Full API
//!
//! See [`zng_app::third_party`] and [`zng_tp_licenses`] for the full API.
//!
//! [`zng_tp_licenses`]: https://zng-ui.github.io/doc/zng_tp_licenses/
//! [`cargo-about`]: https://github.com/EmbarkStudios/cargo-about/
//! [`on_pre_event`]: crate::event::Command::on_pre_event

pub use zng_app::third_party::{LICENSES, License, LicenseUsed, OPEN_LICENSES_CMD, User, UserLicense};

#[cfg(feature = "third_party_default")]
pub(crate) fn setup_default_view() {
    use crate::prelude::*;
    use zng_wgt_container::ChildInsert;

    let id = WindowId::named("zng-third_party-default");
    OPEN_LICENSES_CMD
        .on_event(
            true,
            app_hn!(|args: &zng_app::event::AppCommandArgs, _| {
                if args.propagation().is_stopped() {
                    return;
                }
                args.propagation().stop();

                let parent = WINDOWS.focused_window_id();

                WINDOWS.focus_or_open(id, async move {
                    if let Some(p) = parent {
                        if let Ok(p) = WINDOWS.vars(p) {
                            let v = WINDOW.vars();
                            p.icon().set_bind(&v.icon()).perm();
                        }
                    }

                    Window! {
                        title = l10n!("window.title", "{$app} - Third Party Licenses", app = zng::env::about().app.clone());
                        child = default_view();
                        parent;
                    }
                });
            }),
        )
        .perm();

    fn default_view() -> impl UiNode {
        let mut licenses = LICENSES.user_licenses();
        if licenses.is_empty() {
            licenses.push(UserLicense {
                user: User {
                    // l10n-# "user" is the package that uses the license
                    name: l10n!("license-none.user-name", "<none>").get(),
                    version: "".into(),
                    url: "".into(),
                },
                license: License {
                    id: l10n!("license-none.id", "<none>").get(),
                    // l10n-# License name
                    name: l10n!("license-none.name", "No license data").get(),
                    text: "".into(),
                },
            });
        }
        let selected = var(licenses[0].clone());
        let search = var(Txt::from(""));

        let actual_width = var(zng_layout::unit::Dip::new(0));
        let alternate_layout = actual_width.map(|&w| w <= 500 && w > 1);

        let selector = Container! {
            widget::background_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));

            // search
            child_top = TextInput! {
                txt = search.clone();
                style_fn = zng_wgt_text_input::SearchStyle!();
                zng_wgt_input::focus::focus_shortcut = [shortcut![CTRL+'F'], shortcut![Find]];
                placeholder_txt = l10n!("search.placeholder", "search licenses ({$shortcut})", shortcut="Ctrl+F");
            }, 0;
            // list
            child = Scroll! {
                layout::min_width = 100;
                layout::sticky_width = true;
                mode = zng::scroll::ScrollMode::VERTICAL;
                child_align = Align::FILL;
                child = DataView! {
                    view::<Txt> = search, hn!(selected, |a: &DataViewArgs<Txt>| {
                        let search = a.data().get();
                        let licenses = if search.is_empty() {
                            licenses.clone()
                        } else {
                            licenses.iter().filter(|t| t.user.name.contains(search.as_str())).cloned().collect()
                        };

                        a.set_view(Stack! {
                            toggle::selector = toggle::Selector::single(selected.clone());
                            direction = StackDirection::top_to_bottom();
                            children = licenses.into_iter().map(default_item_view).collect::<UiVec>();
                        })
                    });
                };
                when *#{alternate_layout.clone()} {
                    layout::max_height = 100; // placed on top in small width screens
                    layout::sticky_width = false; // reset sticky width
                }
            };
        };

        Container! {
            layout::actual_width;

            child_insert = {
                placement: alternate_layout.map(|&y| if y { ChildInsert::Top } else { ChildInsert::Start }),
                node: selector,
                spacing: 0,
            };
            // selected
            child = Scroll! {
                mode = zng::scroll::ScrollMode::VERTICAL;
                child_align = Align::TOP_START;
                padding = 10;
                child = zng::markdown::Markdown!(selected.map(default_markdown));
            };
        }
    }

    fn default_item_view(item: UserLicense) -> impl UiNode {
        let txt = if item.user.version.is_empty() {
            item.user.name.clone()
        } else {
            formatx!("{} - {}", item.user.name, item.user.version)
        };
        Toggle! {
            child = Text!(txt);
            value = item;
            child_align = layout::Align::START;
            widget::corner_radius = 0;
            layout::padding = 2;
            widget::border = unset!;
        }
    }

    fn default_markdown(item: &UserLicense) -> Txt {
        use std::fmt::*;

        let mut t = Txt::from("");

        if item.user.version.is_empty() {
            writeln!(&mut t, "# {}\n", item.user.name).unwrap();
        } else {
            writeln!(&mut t, "# {} - {}\n", item.user.name, item.user.version).unwrap();
        }
        if !item.user.url.is_empty() {
            writeln!(&mut t, "[{0}]({0})\n", item.user.url).unwrap();
        }

        writeln!(&mut t, "## {}\n\n", item.license.name).unwrap();

        if !item.license.text.is_empty() {
            writeln!(&mut t, "```\n{}\n```\n", item.license.text).unwrap();
        }

        t.end_mut();
        t
    }
}
