use std::path::{Path, PathBuf};

use zero_ui::core::{app_local, config::*};
use zero_ui::prelude::units::*;
use zero_ui::prelude::*;

#[test]
fn json() {
    test_config("test.config.json", |p| JsonConfig::sync(p));
}

#[test]
fn toml() {
    test_config("test.config.toml", |p| TomlConfig::sync(p));
}

#[test]
fn ron() {
    test_config("test.config.ron", |p| RonConfig::sync(p));
}

fn test_config<C: AnyConfig>(file: &str, source: impl Fn(&Path) -> C) {
    let file = PathBuf::from("../target/tmp").join(file);

    fn run<C: AnyConfig>(source: impl Fn() -> C, test_read: bool) {
        let mut app = App::default().run_headless(false);

        CONFIG.load(source());
        app.run_task(async {
            CONFIG.wait_loaded().await;
        });
        let errors = CONFIG.errors().get();
        if !errors.is_empty() {
            panic!("{errors}");
        }

        TEST_READ.set(test_read);
        test_all();
        let _ = app.update(false);
        app.run_task(async {
            CONFIG.wait_idle().await;
        });

        app.exit();
    }

    let _ = std::fs::remove_file(&file);
    run(|| source(&file), false);
    assert!(file.exists());
    run(|| source(&file), true);
}

app_local! {
    static TEST_READ: bool = false;
}

macro_rules! test_config {
    ($key:expr => $($init:tt)*) => {
        if TEST_READ.get() {
            let key = Txt::from_static($key);
            assert!(CONFIG.contains_key(&key), "did not find {key}");
            let read_value = CONFIG.get(key, || $($init)*).get();
            let expected_value = { $($init)* };

            let read_value = format!("{read_value:#?}");
            let expected_value = format!("{expected_value:#?}");

            pretty_assertions::assert_eq!(expected_value, read_value, "test read {}", stringify!($key));
        } else {
            CONFIG
            .get($key, ||  $($init)*)
                .touch()
                .unwrap();
        }
    };
    ($type:ident $($init:tt)*) => {
        test_config!(stringify!($type) => $type $($init)*)
    };
}

fn test_all() {
    test_view_api_units();
    test_view_api_types();
    test_core_app();
    test_core_units();
    test_core_border();
}

fn test_view_api_units() {
    test_config!(Px(32));
    test_config!(Dip::new(32));

    test_config!(PxPoint::new(Px(100), Px(200)));
    test_config!(DipPoint::new(Dip::new(100), Dip::new(200)));

    test_config!(PxVector::new(Px(100), Px(200)));
    test_config!(DipVector::new(Dip::new(100), Dip::new(200)));

    test_config!(PxSize::new(Px(100), Px(200)));
    test_config!(DipSize::new(Dip::new(100), Dip::new(200)));

    test_config!(PxRect::new(PxPoint::new(Px(100), Px(200)), PxSize::new(Px(1000), Px(2000))));
    test_config!(DipRect::new(
        DipPoint::new(Dip::new(100), Dip::new(200)),
        DipSize::new(Dip::new(100), Dip::new(200)),
    ));

    test_config!(PxBox::new(PxPoint::new(Px(100), Px(200)), PxPoint::new(Px(1000), Px(2000))));
    test_config!(DipBox::new(
        DipPoint::new(Dip::new(100), Dip::new(200)),
        DipPoint::new(Dip::new(100), Dip::new(200)),
    ));

    test_config!(PxSideOffsets::new(Px(1), Px(2), Px(3), Px(4)));
    test_config!(DipSideOffsets::new(Dip::new(1), Dip::new(2), Dip::new(3), Dip::new(4)));

    test_config!(PxCornerRadius::new(
        PxSize::splat(Px(1)),
        PxSize::splat(Px(2)),
        PxSize::splat(Px(3)),
        PxSize::splat(Px(4))
    ));
    test_config!(DipCornerRadius::new(
        DipSize::splat(Dip::new(1)),
        DipSize::splat(Dip::new(2)),
        DipSize::splat(Dip::new(3)),
        DipSize::splat(Dip::new(4))
    ));

    test_config!("PxTransform::identity" => PxTransform::identity());
    test_config!("PxTransform::translation" => PxTransform::translation(10.0, 20.0));
    test_config!("PxTransform::rotation" => PxTransform::rotation(1.0, 2.0, euclid::Angle::degrees(40.0)));
}

fn test_view_api_types() {
    use zero_ui::core::{
        app::view_process::zero_ui_view_api::EventFrameRendered,
        mouse::{MouseScrollDelta, TouchForce, TouchPhase},
        render::{webrender_api::DebugFlags, FrameId, RendererDebug},
    };

    test_config!(FrameId::first().next().next_update().next());
    test_config!(KeyState::Pressed);
    test_config!(ButtonState::Pressed);
    test_config!(MouseButton::Left);
    test_config!("MouseButton::Other" => MouseButton::Other(564));
    test_config!(TouchPhase::Started);
    test_config!(MouseScrollDelta::LineDelta(32.0, 34.0));
    test_config!(TouchForce::Calibrated {
        force: 5.0,
        max_possible_force: 10.0,
        altitude_angle: None
    });
    test_config!(Key::C);
    test_config!(CursorIcon::Alias);
    test_config!(WindowState::Normal);

    test_config!(EventFrameRendered {
        window: 3,
        frame: FrameId::first(),
        frame_image: None
    });

    test_config!(RendererDebug {
        flags: DebugFlags::DISABLE_ALPHA_PASS | DebugFlags::DISABLE_BATCHING,
        profiler_ui: "default".to_owned()
    });
}

fn test_core_app() {
    use zero_ui::core::app::*;

    let id = AppId::named("app-name");
    test_config!("AppId" => id);
}

fn test_core_units() {
    use zero_ui::core::units::*;

    test_config!("Factor" => 1.fct());
    test_config!("FactorPercent" => 100.pct());
    test_config!(Align::TOP_LEFT);
    test_config!("AlignCustom" => Align { x: 0.1.fct(), x_rtl_aware: true, y: 0.4.fct() });
    test_config!("AngleRadian" => 1.rad());
    test_config!("AngleGradian" => 50.grad());
    test_config!("AngleDegree" => 50.deg());
    test_config!("AngleTurn" => 2.turn());
    test_config!("ByteLength" => 1.megabytes());
    test_config!(PxConstraints::new_exact(Px(40)));
    test_config!("PxConstraints::unbounded" => PxConstraints::new_unbounded());
    test_config!(PxConstraints2d::new_bounded_size(PxSize::splat(Px(50))));
    test_config!("Length::Default" => Length::Default);
    test_config!("Length::Px" => Length::Px(Px(300)));
    test_config!("Length::Expr" => Length::Default * 20.pct());
    test_config!(GridSpacing::new(4.dip(), 5.dip()));
    test_config!("Ppi" => 96.ppi());
}

fn test_core_border() {
    test_config!(LineStyle::Dashed);
    test_config!("LineStyle::Wavy" => LineStyle::Wavy(4.0));
    test_config!(BorderStyle::Dotted);
    test_config!(BorderSides::new_all(colors::RED));
}
