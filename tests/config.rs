use std::{
    fs,
    path::{Path, PathBuf},
};

use zng::{
    app::AppId,
    config::*,
    keyboard::{Key, KeyState},
    layout::*,
    mouse::{ButtonState, CursorIcon, MouseButton, MouseScrollDelta},
    prelude::*,
    render::FrameId,
    touch::{TouchForce, TouchPhase},
    widget::{BorderSides, BorderStyle, LineStyle},
    window::WindowState,
};

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

#[test]
fn yaml() {
    test_config("test.config.yml", |p| YamlConfig::sync(p));
}

fn test_config<C: AnyConfig>(file: &str, source: impl Fn(&Path) -> C) {
    let file = temp(file);

    fn run<C: AnyConfig>(source: impl Fn() -> C, test_read: bool) {
        let mut app = APP.defaults().run_headless(false);
        zng::app::test_log();

        CONFIG.load(source());
        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        TEST_READ.set(test_read);
        test_all();
        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });

        app.exit();
    }

    rmv_file_assert(&file);
    run(|| source(&file), false);
    assert!(file.exists());
    assert_ne!(std::fs::metadata(&file).unwrap().len(), 0);

    run(|| source(&file), true);
    assert!(file.exists());
    assert_ne!(std::fs::metadata(&file).unwrap().len(), 0);
}

zng::app::app_local! {
    static TEST_READ: bool = false;
}

macro_rules! test_config {
    ($key:expr => $($init:tt)*) => {
        if TEST_READ.get() {
            let key = Txt::from_static($key);
            assert!(CONFIG.contains_key(key.clone()).get(), "did not find {key}");
            let read_value = CONFIG.get(key, $($init)*).get();
            let expected_value = { $($init)* };

            let read_value = format!("{read_value:#?}");
            let expected_value = format!("{expected_value:#?}");

            pretty_assertions::assert_eq!(expected_value, read_value, "test read {}", stringify!($key));
        } else {
            CONFIG
            .get($key, $($init)*)
                .update();
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
    test_config!("PxTransform::rotation" => PxTransform::rotation(1.0, 2.0, layout::AngleRadian::from(40.deg()).into()));
}

fn test_view_api_types() {
    test_config!(FrameId::first().next().next_update().next());
    test_config!(KeyState::Pressed);
    test_config!(ButtonState::Pressed);
    test_config!(MouseButton::Left);
    test_config!("MouseButton::Other" => MouseButton::Other(564));
    test_config!(TouchPhase::Start);
    test_config!(MouseScrollDelta::LineDelta(32.0, 34.0));
    test_config!(TouchForce::Calibrated {
        force: 5.0,
        max_possible_force: 10.0,
        altitude_angle: None
    });
    test_config!(Key::Char('G'));
    test_config!(CursorIcon::Alias);
    test_config!(WindowState::Normal);

    test_config!(zng_wgt_webrender_debug::RendererDebug::default());
}

fn test_core_app() {
    let id = AppId::named("app-name");
    test_config!("AppId" => id);
}

fn test_core_units() {
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
    test_config!("PxDensity" => 96.ppi());
}

fn test_core_border() {
    test_config!(LineStyle::Dashed);
    test_config!("LineStyle::Wavy" => LineStyle::Wavy(4.0));
    test_config!(BorderStyle::Dotted);
    test_config!(BorderSides::new_all(colors::RED));
}

#[test]
fn concurrent_read_write() {
    let file = temp("concurrent.json");

    {
        // setup
        rmv_file_assert(&file);
        let mut app = APP.defaults().run_headless(false);
        zng::app::test_log();
        CONFIG.load(JsonConfig::sync(&file));
        CONFIG.get("key", Txt::from_static("default/custom")).set("custom");

        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        app.exit();
    }

    // tests
    let threads: Vec<_> = (0..10)
        .map(|_| {
            std::thread::spawn(clmv!(file, || {
                let mut app = APP.defaults().run_headless(false);
                zng::app::test_log();
                CONFIG.load(JsonConfig::sync(file));

                app.run_task(async {
                    task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
                });

                let var = CONFIG.get("key", Txt::from_static("default/get"));
                for _ in 0..8 {
                    assert_eq!("custom", var.get());
                    var.set("custom");
                    app.update(false).assert_wait();
                }

                app.exit();
            }))
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }
}

#[test]
fn fallback_swap() {
    let main_cfg = temp("fallback_swap.target.json");
    let main_prepared_cfg = temp("fallback_swap.prep.json");
    let fallback_cfg = temp("fallback_swap.fallback.json");

    {
        // setup
        rmv_file_assert(&main_cfg);
        rmv_file_assert(&main_prepared_cfg);
        rmv_file_assert(&fallback_cfg);

        let mut app = APP.defaults().run_headless(false);
        zng::app::test_log();
        CONFIG.load(JsonConfig::sync(&fallback_cfg));
        CONFIG.get("key", Txt::from_static("default/fallback")).set("fallback");

        app.update(false).assert_wait();
        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        CONFIG.load(JsonConfig::sync(&main_prepared_cfg));
        CONFIG.get("key", Txt::from_static("default/main")).set("main");

        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        app.exit();
    }

    // test
    let mut app = APP.defaults().run_headless(false);
    zng::app::test_log();

    CONFIG.load(FallbackConfig::new(JsonConfig::sync(&main_cfg), JsonConfig::sync(fallback_cfg)));
    app.run_task(async {
        task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
    });
    let status = CONFIG.status().get();
    if status != ConfigStatus::Loaded {
        panic!("{status}");
    }

    app.update(false).assert_wait();

    let key = CONFIG.get("key", Txt::from_static("default/get"));
    assert_eq!("fallback", key.get());

    std::fs::rename(main_prepared_cfg, main_cfg).unwrap();
    app.update(false).assert_wait();
    app.run_task(async {
        task::deadline(1.60.secs()).await; // wait for system rename event (+ debounce)
        task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
    });
    let status = CONFIG.status().get();
    if status != ConfigStatus::Loaded {
        panic!("{status}");
    }

    assert_eq!("main", key.get());

    app.exit();
}

#[test]
fn fallback_reset() {
    let main_cfg = temp("fallback_reset.target.json");
    let fallback_cfg = temp("fallback_reset.fallback.json");

    {
        // setup
        rmv_file_assert(&main_cfg);
        rmv_file_assert(&fallback_cfg);

        let mut app = APP.defaults().run_headless(false);
        zng::app::test_log();
        CONFIG.load(JsonConfig::sync(&fallback_cfg));
        CONFIG.get("key", Txt::from_static("default/fallback")).set("fallback");

        app.update(false).assert_wait();
        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        CONFIG.load(JsonConfig::sync(&main_cfg));
        CONFIG.get("key", Txt::from_static("default/main")).set("main");

        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        app.exit();
    }

    // test
    let mut app = APP.defaults().run_headless(false);
    zng::app::test_log();

    CONFIG.load(FallbackConfig::new(JsonConfig::sync(&main_cfg), JsonConfig::sync(&fallback_cfg)));
    app.run_task(async {
        task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
    });
    let status = CONFIG.status().get();
    if status != ConfigStatus::Loaded {
        panic!("{status}");
    }

    app.update(false).assert_wait();

    let key = CONFIG.get("key", Txt::from_static("default/get"));
    assert_eq!("main", key.get());

    CONFIG.load(MemoryConfig::default());
    app.update(false).assert_wait();

    rmv_file_assert(&main_cfg);

    CONFIG.load(FallbackConfig::new(JsonConfig::sync(&main_cfg), JsonConfig::sync(&fallback_cfg)));
    app.run_task(async {
        task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
    });
    let status = CONFIG.status().get();
    if status != ConfigStatus::Loaded {
        panic!("{status}");
    }
    assert_eq!("fallback", key.get());

    app.exit();
}

#[test]
fn fallback_reset_entry() {
    let main_cfg = temp("fallback_reset_entry.target.json");
    let fallback_cfg = temp("fallback_reset_entry.fallback.json");

    {
        // setup
        rmv_file_assert(&main_cfg);
        rmv_file_assert(&fallback_cfg);

        let mut app = APP.defaults().run_headless(false);
        zng::app::test_log();
        CONFIG.load(JsonConfig::sync(&fallback_cfg));
        CONFIG.get("key", Txt::from_static("default/fallback")).set("fallback");

        app.update(false).assert_wait();
        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        CONFIG.load(JsonConfig::sync(&main_cfg));
        CONFIG.get("key", Txt::from_static("default/main")).set("main");

        app.run_task(async {
            task::with_deadline(CONFIG.wait_idle(), 60.secs()).await.unwrap();
        });
        let status = CONFIG.status().get();
        if status.is_err() {
            panic!("{status}");
        }

        app.exit();
    }

    // test
    let mut app = APP.defaults().run_headless(false);
    zng::app::test_log();

    let mut cfg = FallbackConfig::new(JsonConfig::sync(&main_cfg), JsonConfig::sync(&fallback_cfg));
    // wait_idle
    let status = cfg.status();
    app.run_task(async move {
        task::with_deadline(
            async move {
                while !status.get().is_idle() {
                    status.wait_update().await;
                }
            },
            5.secs(),
        )
        .await
        .unwrap();
    });

    let key = cfg.get("key", Txt::from_static("default/get"), false);
    assert_eq!("main", key.get());

    cfg.reset(&ConfigKey::from_static("key"));
    app.update(false).assert_wait();

    assert_eq!("fallback", key.get());

    // wait_idle
    let status = cfg.status();
    app.run_task(async move {
        task::with_deadline(
            async move {
                while !status.get().is_idle() {
                    status.wait_update().await;
                }
            },
            60.secs(),
        )
        .await
        .unwrap();
    });

    let raw_config = std::fs::read_to_string(&main_cfg).unwrap();
    assert!(!raw_config.contains("key"));

    app.exit();
}

fn rmv_file_assert(path: &Path) {
    if let Err(e) = std::fs::remove_file(path)
        && !matches!(e.kind(), std::io::ErrorKind::NotFound)
    {
        panic!("cannot remove `{}`\n{e}", path.display());
    }
}

fn temp(file_name: &str) -> PathBuf {
    let p = Path::new("../target/tmp/tests/config/");
    let _ = fs::create_dir_all(p);
    p.join(file_name)
}
