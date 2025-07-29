mod any {
    use zng::{prelude::*, var::AnyVar};

    #[test]
    fn downcast() {
        let any_var = AnyVar::from(var(true));
        assert!(any_var.downcast::<bool>().is_ok())
    }
}

mod bindings {
    use zng::{prelude::*, var::VARS};
    use zng_app::AppControlFlow;

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_txt());

        let mut app = APP.minimal().run_headless(false);
        app.update(false).assert_wait();

        a.bind_map(&b, |a| a.to_txt()).perm();

        let mut updated = 0;
        let _ = app.update_observe(
            || {
                updated += 1;
            },
            false,
        );
        assert_eq!(0, updated);

        a.set(20);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(20i32), a.get_new());
                    assert_eq!(Some("20".to_txt()), b.get_new());
                }
            },
            false,
        );
        assert!(updated);

        a.set(13);

        updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(13i32), a.get_new());
                    assert_eq!(Some("13".to_txt()), b.get_new());
                }
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn two_way_binding() {
        let a = var(10);
        let b = var("".to_txt());

        let mut app = APP.minimal().run_headless(false);
        app.update(false).assert_wait();

        a.bind_map_bidi(&b, |a| a.to_txt(), |b| b.parse().unwrap()).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            || {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(20i32), a.get_new());
                    assert_eq!(Some("20".to_txt()), b.get_new());
                }
            },
            false,
        );
        assert!(updated);

        b.set("55");

        updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some("55".to_txt()), b.get_new());
                    assert_eq!(Some(55i32), a.get_new());
                }
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn one_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_txt());

        let mut app = APP.minimal().run_headless(false);
        app.update(false).assert_wait();

        a.bind_filter_map(&b, |a| if *a == 13 { None } else { Some(a.to_txt()) }).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            || {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(20i32), a.get_new());
                    assert_eq!(Some("20".to_txt()), b.get_new());
                }
            },
            false,
        );
        assert!(updated);

        a.set(13);

        updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(13i32), a.get_new());
                    assert_eq!("20".to_txt(), b.get());
                    assert!(!b.is_new());
                }
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn two_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_txt());

        let mut app = APP.minimal().run_headless(false);
        app.update(false).assert_wait();

        a.bind_filter_map_bidi(&b, |a| Some(a.to_txt()), |b| b.parse().ok()).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            || {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(20i32), a.get_new());
                    assert_eq!(Some("20".to_txt()), b.get_new());
                }
            },
            false,
        );
        assert!(updated);

        b.set("55");

        updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some("55".to_txt()), b.get_new());
                    assert_eq!(Some(55i32), a.get_new());
                }
            },
            false,
        );
        assert!(updated);

        b.set("not a i32");

        updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some("not a i32".to_txt()), b.get_new());
                    assert_eq!(55i32, a.get());
                    assert!(!a.is_new());
                }
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn binding_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = APP.minimal().run_headless(false);
        app.update(false).assert_wait();

        a.bind_map(&b, |a| *a + 1).perm();
        b.bind_map(&c, |b| *b + 1).perm();
        c.bind_map(&d, |c| *c + 1).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            || {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;

                    assert_eq!(Some(20), a.get_new());
                    assert_eq!(Some(21), b.get_new());
                    assert_eq!(Some(22), c.get_new());
                    assert_eq!(Some(23), d.get_new());
                }
            },
            false,
        );
        assert!(updated);

        a.set(30);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;

                    assert_eq!(Some(30), a.get_new());
                    assert_eq!(Some(31), b.get_new());
                    assert_eq!(Some(32), c.get_new());
                    assert_eq!(Some(33), d.get_new());
                }
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn binding_bidi_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = APP.minimal().run_headless(false);
        app.update(false).assert_wait();

        a.bind_bidi(&b).perm();
        b.bind_bidi(&c).perm();
        c.bind_bidi(&d).perm();

        let mut update_count = 0;
        let _ = app.update_observe(
            || {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(20);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;

                    assert_eq!(Some(20), a.get_new());
                    assert_eq!(Some(20), b.get_new());
                    assert_eq!(Some(20), c.get_new());
                    assert_eq!(Some(20), d.get_new());
                }
            },
            false,
        );
        assert!(updated);

        d.set(30);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;

                    assert_eq!(Some(30), a.get_new());
                    assert_eq!(Some(30), b.get_new());
                    assert_eq!(Some(30), c.get_new());
                    assert_eq!(Some(30), d.get_new());
                }
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn binding_drop() {
        let a = var(1);
        let b = var(1);

        let mut app = APP.minimal().run_headless(false);

        let handle = a.bind_map(&b, |i| *i + 1);

        a.set(10);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;

                    assert_eq!(Some(10), a.get_new());
                    assert_eq!(Some(11), b.get_new());
                }
            },
            false,
        );
        assert!(updated);

        drop(handle);

        a.set(100);

        updated = false;
        let _ = app.update_observe(
            || {
                if !updated {
                    updated = true;

                    assert_eq!(Some(100), a.get_new());
                    assert!(!b.is_new());
                    assert_eq!(11, b.get());
                }
            },
            false,
        );
        assert!(updated);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());
    }

    #[test]
    fn binding_bidi_set_both() {
        let mut app = APP.minimal().run_headless(false);

        let a = var(1);
        let b = var(1);
        a.bind_bidi(&b).perm();

        a.set(10);
        b.set(20);
        app.update(false).assert_wait();

        assert_eq!(20, a.get());
        assert_eq!(20, b.get());
    }

    #[test]
    fn binding_update_order() {
        let mut app = APP.minimal().run_headless(false);

        let a = var(0);
        let b = var(0);
        a.bind(&b).perm();

        a.set(1);
        b.set(10);

        app.update(false).assert_wait();

        assert_eq!(10, b.get());
    }

    #[test]
    fn binding_update_order2() {
        let mut app = APP.minimal().run_headless(false);

        let a = var(0);
        let b = var(0);

        a.set(1);
        b.set(a.get());
        a.bind(&b).perm();

        app.update(false).assert_wait();

        assert_eq!(0, b.get());
    }

    #[test]
    fn binding_update_order3() {
        let mut app = APP.minimal().run_headless(false);

        let a = var(0);
        let b = var(0);

        a.set(1);
        b.set_from(&a);
        a.bind(&b).perm();

        app.update(false).assert_wait();

        assert_eq!(1, b.get());
    }

    #[test]
    fn animation_and_set_from() {
        let mut app = APP.minimal().run_headless(false);

        let a = var(0);
        let b = var(0);

        VARS.animate(clmv!(a, |_| {
            a.set(1);
            APP.exit();
        }))
        .perm();

        b.set_from(&a);
        a.bind(&b).perm();

        while let AppControlFlow::Wait = app.update(true) {}

        assert_eq!(1, a.get());
        assert_eq!(1, b.get());
    }
}

mod context {
    use zng::{
        app::{AppExtended, AppExtension, HeadlessApp},
        prelude::*,
        prelude_wgt::*,
        var::{AnyWhenVarBuilder, ContextInitHandle},
    };

    context_var! {
        static TEST_VAR: Txt = "";
    }

    app_local! {
        static PROBE_ID: Txt = const { Txt::from_static("") };
    }

    #[property(CONTEXT, default(TEST_VAR))]
    fn test_prop(child: impl UiNode, value: impl IntoVar<Txt>) -> impl UiNode {
        with_context_var(child, TEST_VAR, value)
    }

    #[property(CONTEXT, default(TEST_VAR))]
    fn test_prop_a(child: impl UiNode, value: impl IntoVar<Txt>) -> impl UiNode {
        test_prop(child, value)
    }
    #[property(CONTEXT, default(TEST_VAR))]
    fn test_prop_b(child: impl UiNode, value: impl IntoVar<Txt>) -> impl UiNode {
        test_prop(child, value)
    }

    #[property(CONTEXT)]
    fn probe(child: impl UiNode, var: impl IntoVar<Txt>) -> impl UiNode {
        let var = var.into_var();
        match_node(child, move |_, op| {
            if let UiNodeOp::Init = op {
                *PROBE_ID.write() = var.get();
            }
        })
    }
    #[property(CONTEXT)]
    fn probe_a(child: impl UiNode, var: impl IntoVar<Txt>) -> impl UiNode {
        probe(child, var)
    }
    #[property(CONTEXT)]
    fn probe_b(child: impl UiNode, var: impl IntoVar<Txt>) -> impl UiNode {
        probe(child, var)
    }

    #[property(EVENT)]
    fn on_init(child: impl UiNode, mut handler: impl WidgetHandler<()>) -> impl UiNode {
        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                child.init();
                handler.event(&());
            }
            UiNodeOp::Update { updates } => {
                child.update(updates);
                handler.update();
            }
            _ => {}
        })
    }

    #[widget($crate::context::TestWgt)]
    struct TestWgt(WidgetBase);
    impl TestWgt {
        fn widget_intrinsic(&mut self) {
            self.widget_builder().push_build_action(|wgt| {
                if let Some(child) = wgt.capture_ui_node(property_id!(Self::child)) {
                    wgt.set_child(child);
                }
            });
        }
    }
    #[property(CHILD, capture, widget_impl(TestWgt))]
    fn child(child: impl UiNode) {}

    fn test_app(app: AppExtended<impl AppExtension>, root: impl UiNode) -> HeadlessApp {
        zng_app::test_log();

        let mut app = app.run_headless(false);
        WINDOWS.open(async move { window::WindowRoot::new_test(root) });
        let _ = app.update(false);
        app
    }

    #[test]
    fn context_var_basic() {
        let _test = test_app(
            APP.defaults(),
            TestWgt! {
                test_prop = "test!";

                child = TestWgt! {
                    probe = TEST_VAR;
                }
            },
        );

        assert_eq!(&*PROBE_ID.read(), &Txt::from("test!"));
    }

    #[test]
    fn context_var_map() {
        let _test = test_app(
            APP.defaults(),
            TestWgt! {
                test_prop = "test!";

                child = TestWgt! {
                    probe = TEST_VAR.map(|t| formatx!("map {t}"));
                }
            },
        );

        assert_eq!(&*PROBE_ID.read(), &Txt::from("map test!"));
    }

    #[test]
    fn context_var_map_cloned() {
        let app = APP.defaults();

        // mapped context var should depend on the context.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));

        let _test = test_app(
            app,
            TestWgt! {
                test_prop_a = "A!";

                child = TestWgt! {
                    probe = mapped.clone();
                    test_prop_b = "B!";

                    child = TestWgt! {
                        probe = mapped;
                    }
                }
            },
        );

        assert_eq!(&*PROBE_ID.read(), &Txt::from("map B!"));
    }

    #[test]
    fn context_var_map_cloned3() {
        let app = APP.defaults();
        // mapped context var should depend on the context.

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        let _test = test_app(
            app,
            TestWgt! {
                test_prop = "A!";

                child = TestWgt! {
                    probe = mapped.clone();
                    test_prop = "B!";

                    child = TestWgt! {
                        probe = mapped.clone();
                        test_prop = "C!";

                        child = TestWgt! {
                            probe = mapped;
                            test_prop = "D!";
                        }
                    }
                }
            },
        );

        assert_eq!(&*PROBE_ID.read(), &Txt::from("map C!"));
    }

    #[test]
    fn context_var_map_not_cloned() {
        let app = APP.defaults();

        // sanity check for `context_var_map_cloned`

        let _test = test_app(
            app,
            TestWgt! {
                test_prop_a = "A!";

                child = TestWgt! {
                    probe = TEST_VAR.map(|t| formatx!("map {t}"));
                    test_prop_b = "B!";

                    child = TestWgt! {
                        probe = TEST_VAR.map(|t| formatx!("map {t}"));
                    }
                }
            },
        );

        assert_eq!(&*PROBE_ID.read(), &Txt::from("map B!"));
    }

    #[test]
    fn context_var_map_moved_app_ctx() {
        let _app = APP.minimal();

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        let a = TEST_VAR.with_context_var(ContextInitHandle::new(), "A", || mapped.get());

        let b = TEST_VAR.with_context_var(ContextInitHandle::new(), "B", || mapped.get());

        assert_ne!(a, b);
    }

    #[test]
    fn context_var_cloned_same_widget() {
        let app = APP.defaults();

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));

        let _test = test_app(
            app,
            TestWgt! {
                test_prop_a = "A!";
                probe_a = mapped.clone();
                test_prop_b = "B!";
                probe_b = mapped;
            },
        );

        assert_eq!(&*PROBE_ID.read(), &Txt::from("map B!"));
    }

    #[test]
    fn context_var_set() {
        let mut app = test_app(APP.defaults(), NilUiNode);

        let backing_var = var(Txt::from(""));

        TEST_VAR.with_context_var(ContextInitHandle::new(), backing_var.clone(), || {
            let t = TEST_VAR;
            assert!(t.capabilities().contains(VarCapability::MODIFY));
            t.set("set!");
        });

        let _ = app.update(false);
        assert_eq!(backing_var.get(), "set!");
    }

    #[test]
    fn context_var_binding() {
        let app = APP.defaults();

        let input_var = var("Input!".to_txt());
        let other_var = var(".".to_txt());

        let mut test = test_app(
            app,
            TestWgt! {
                test_prop = input_var.clone();
                on_init = hn_once!(other_var, |_| {
                    TEST_VAR.bind(&other_var).perm();
                });
                child = NilUiNode;
            },
        );

        test.update(false).assert_wait();

        assert_eq!(".", other_var.get());

        input_var.set("Update!");

        test.update(false).assert_wait();

        assert_eq!("Update!", input_var.get());
        assert_eq!("Update!", other_var.get());
    }

    #[test]
    fn context_var_recursion_when1() {
        let _scope = APP.minimal();

        let var = when_var! {
            false => var("hello".to_txt()),
            _ => TEST_VAR,
        };

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_when2() {
        let _scope = APP.minimal();

        let var = when_var! {
            true => TEST_VAR,
            _ => var("hello".to_txt()),
        };

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_issue_when_any() {
        let _scope = APP.minimal();

        let mut var = AnyWhenVarBuilder::new(TEST_VAR.into());
        var.push(self::var(false), self::var("hello".to_txt()).into());
        let var = var.into_typed().build();

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_merge() {
        let _scope = APP.minimal();

        let var = merge_var!(TEST_VAR, var(true), |t, _| t.clone());

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }
}

mod flat_map {
    use std::fmt;
    use zng::prelude::*;

    #[derive(Clone)]
    pub struct Foo {
        pub bar: bool,
        pub var: Var<usize>,
    }
    impl PartialEq for Foo {
        fn eq(&self, other: &Self) -> bool {
            self.bar == other.bar && self.var.var_eq(&other.var)
        }
    }
    impl fmt::Debug for Foo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Foo").field("bar", &self.bar).finish_non_exhaustive()
        }
    }

    #[test]
    pub fn flat_map() {
        let mut app = APP.minimal().run_headless(false);

        let source = var(Foo { bar: true, var: var(32) });

        let test = source.flat_map(|f| f.var.clone());

        assert_eq!(32, test.get());

        source.get().var.set(42usize);

        let _ = app.update_observe(
            || {
                assert!(test.is_new());
                assert_eq!(42, test.get());
            },
            false,
        );

        let old_var = source.get().var;
        source.set(Foo { bar: false, var: var(192) });
        let _ = app.update_observe(
            || {
                assert!(test.is_new());
                assert_eq!(192, test.get());
            },
            false,
        );

        old_var.set(220usize);
        let _ = app.update_observe(
            || {
                assert!(!test.is_new());
                assert_eq!(192, test.get());
            },
            false,
        );
    }
}

mod modify_importance {
    use zng::{prelude::*, var::VARS};

    #[test]
    pub fn set_same_importance() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(Txt::from_static("v0"));
        test.set("v1");
        app.update(false).assert_wait();
        assert_eq!("v1", test.get());
        let importance = test.modify_importance();

        test.set("v2");
        app.update(false).assert_wait();
        assert_eq!("v2", test.get());
        assert_eq!(importance, test.modify_importance());
    }

    #[test]
    pub fn set_same_importance_in_vars() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(Txt::from_static("v0"));
        test.set("v1");
        app.update(false).assert_wait();
        assert_eq!("v1", test.get());
        let importance = VARS.current_modify().importance();

        test.set("v2");
        app.update(false).assert_wait();
        assert_eq!("v2", test.get());
        assert_eq!(importance, VARS.current_modify().importance());
    }

    #[test]
    pub fn animate_set_diff_importance() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(Txt::from_static("v0"));
        test.set("v1");
        app.update(false).assert_wait();
        assert_eq!("v1", test.get());
        let importance = test.modify_importance();

        test.step("v2", 0.ms()).perm();
        app.run_task(async_clmv!(test, {
            test.wait_animation().await;
        }));
        assert_eq!("v2", test.get());
        assert!(importance <= test.modify_importance());
        let importance = test.modify_importance();

        test.set("v3");
        app.update(false).assert_wait();
        assert_eq!("v3", test.get());
        assert!(importance <= test.modify_importance());
    }

    #[test]
    pub fn animate_set_diff_importance_in_vars() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(Txt::from_static("v0"));
        test.set("v1");
        app.update(false).assert_wait();
        assert_eq!("v1", test.get());
        let importance = VARS.current_modify().importance();

        test.step("v2", 0.ms()).perm();
        app.run_task(async_clmv!(test, {
            test.wait_animation().await;
        }));
        assert_eq!("v2", test.get());
        assert!(importance <= VARS.current_modify().importance());
        let importance = VARS.current_modify().importance();

        test.set("v3");
        app.update(false).assert_wait();
        assert_eq!("v3", test.get());
        assert!(importance <= VARS.current_modify().importance());
    }

    #[test]
    pub fn animate_in_hook() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(Txt::from_static("v0"));
        let ease = var(0i32);
        test.hook(clmv!(ease, |_| {
            ease.ease(100, 10.ms(), easing::linear).perm();
            false // once
        }))
        .perm();
        let importance = VARS.current_modify().importance();

        test.set("v1");
        app.update(false).assert_wait();
        assert_eq!("v1", test.get());

        app.run_task(async_clmv!(ease, {
            ease.wait_animation().await;
        }));
        assert_eq!(100, ease.get());

        assert!(importance < VARS.current_modify().importance());
    }
}

mod cow {
    use std::sync::Arc;

    use zng::{prelude::*, task::parking_lot::Mutex};

    #[test]
    pub fn cow_base_update() {
        let mut app = APP.minimal().run_headless(false);

        let base = var(false);
        let cow = base.cow();

        base.set(true);
        app.update(false).assert_wait();

        assert!(base.get());
        assert!(cow.get());
    }

    #[test]
    pub fn cow_update() {
        let mut app = APP.minimal().run_headless(false);

        let base = var(false);
        let cow = base.cow();

        cow.set(true);
        app.update(false).assert_wait();

        assert!(!base.get());
        assert!(cow.get());
    }

    #[test]
    pub fn cow_update_full() {
        let mut app = APP.minimal().run_headless(false);

        let base = var(0);
        let cow = base.cow();

        let base_values = Arc::new(Mutex::new(vec![]));
        let cow_values = Arc::new(Mutex::new(vec![]));
        base.trace_value(clmv!(base_values, |v| base_values.lock().push(*v.value()))).perm();
        cow.trace_value(clmv!(cow_values, |v| cow_values.lock().push(*v.value()))).perm();

        base.set(1);
        app.update(false).assert_wait();

        assert_eq!(1, base.get());
        assert_eq!(1, cow.get());

        cow.set(2);
        app.update(false).assert_wait();

        assert_eq!(1, base.get());
        assert_eq!(2, cow.get());

        assert_eq!(&base_values.lock()[..], &[0, 1]);
        assert_eq!(&cow_values.lock()[..], &[0, 1, 2]);

        base.set(3);
        app.update(false).assert_wait();
        base.set(4);
        app.update(false).assert_wait();
        assert_eq!(&base_values.lock()[..], &[0, 1, 3, 4]);
        assert_eq!(&cow_values.lock()[..], &[0, 1, 2]);
    }
}

mod multi {
    use std::sync::Arc;

    use zng::{prelude::*, task::parking_lot::Mutex};

    #[test]
    fn multi_bidi() {
        let mut app = APP.minimal().run_headless(false);

        let a = var(false);
        let b = a.map_bidi(
            |&a| if a { 1i32 } else { 0 },
            |&b| match b {
                0 => false,
                1 => true,
                n => panic!("invalid test {n}"),
            },
        );

        let a_values = Arc::new(Mutex::new(vec![]));
        let b_values = Arc::new(Mutex::new(vec![]));
        a.trace_value(clmv!(a_values, |v| a_values.lock().push(*v.value()))).perm();
        b.trace_value(clmv!(b_values, |v| b_values.lock().push(*v.value()))).perm();

        assert!(!a.get());
        assert_eq!(b.get(), 0);

        a.set(true);
        app.update(false).assert_wait();
        assert!(a.get());
        assert_eq!(b.get(), 1);

        assert_eq!(&a_values.lock()[..], &[false, true]);
        assert_eq!(&b_values.lock()[..], &[0, 1]);
    }
}

mod threads {
    use zng::prelude::*;

    #[test]
    fn set_from_other_thread_once() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(1);

        task::spawn(async_clmv!(test, {
            test.set(2);
        }));

        let test = async move {
            while test.get() != 2 {
                test.wait_update().await;
            }
        };
        app.run_task(task::with_deadline(test, 20.secs())).unwrap().unwrap();
    }

    #[test]
    fn set_from_other_thread_many() {
        let mut app = APP.minimal().run_headless(false);

        let test = var(1);

        task::spawn(async_clmv!(test, {
            for i in 2..=100 {
                test.set(i);
                if i % 10 == 0 {
                    task::deadline(2.ms()).await;
                }
            }
        }));

        let mut prev = 0;
        let test = async move {
            loop {
                let new = test.get();
                assert!(prev < new, "{prev} < {new}");
                if new == 100 {
                    break;
                }
                prev = new;
                test.wait_update().await;
            }
        };

        app.run_task(task::with_deadline(test, 40.secs())).unwrap().unwrap();
    }
}

mod contextualized {
    use zng::{
        prelude::*,
        var::{ContextInitHandle, contextual_var},
    };

    #[test]
    fn nested_contextualized_vars() {
        let mut app = APP.defaults().run_headless(false);

        let var = var(0u32);
        let source = contextual_var(move || var.clone());
        let mapped = source.map(|n| n + 1);
        let mapped2 = mapped.map(|n| n - 1);
        let mapped2_copy = mapped2.clone();

        // init, same effect as subscribe in widgets, the last to init breaks the other.
        assert_eq!(0, mapped2.get());
        assert_eq!(0, mapped2_copy.get());

        source.set(10u32);
        let mut updated = false;
        app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(10), mapped2.get_new());
                    assert_eq!(Some(10), mapped2_copy.get_new());
                }
            },
            false,
        )
        .assert_wait();

        assert!(updated);
    }

    #[test]
    fn nested_contextualized_vars_diff_contexts() {
        let mut app = APP.defaults().run_headless(false);

        let var = var(0u32);
        let source = contextual_var(move || var.clone());
        let mapped = source.map(|n| n + 1);
        let mapped2 = mapped.map(|n| n - 1);
        let mapped2_copy = mapped2.clone();

        // init, same effect as subscribe in widgets, the last to init breaks the other.
        assert_eq!(0, mapped2.get());
        let other_ctx = ContextInitHandle::new();
        other_ctx.with_context(|| {
            assert_eq!(0, mapped2_copy.get());
        });

        source.set(10u32);
        let mut updated = false;
        app.update_observe(
            || {
                if !updated {
                    updated = true;
                    assert_eq!(Some(10), mapped2.get_new());
                    other_ctx.with_context(|| {
                        assert_eq!(Some(10), mapped2_copy.get_new());
                    });
                }
            },
            false,
        )
        .assert_wait();

        assert!(updated);
    }
}

mod vec {
    use zng::{
        prelude::*,
        var::{ObservableVec, VecChange},
    };

    #[test]
    fn basic_usage() {
        let mut app = APP.minimal().run_headless(false);

        let list = var(ObservableVec::<u32>::new());

        list.modify(|a| {
            a.push(32);
        });
        app.update_observe(
            || {
                assert!(list.is_new());

                list.with_new(|l| {
                    assert_eq!(&[32], &l[..]);
                    assert_eq!(&[VecChange::Insert { index: 0, count: 1 }], l.changes());
                });
            },
            false,
        )
        .assert_wait();

        list.modify(|a| {
            a.push(33);
        });
        app.update_observe(
            || {
                assert!(list.is_new());

                list.with_new(|l| {
                    assert_eq!(&[32, 33], &l[..]);
                    assert_eq!(&[VecChange::Insert { index: 1, count: 1 }], l.changes());
                });
            },
            false,
        )
        .assert_wait();
    }
}

mod response {
    use zng::prelude::*;

    #[test]
    fn race_condition() {
        let mut app = APP.minimal().run_headless(false);

        for _ in 0..10 {
            let a = task::respond(async {
                task::deadline(1.ms()).await;
                'a'
            });
            let b = task::respond(async {
                task::deadline(1.ms()).await;
                'b'
            });
            let ab = task::respond(async {
                let mut r = String::new();
                for v in [a, b] {
                    r.push(v.await);
                }
                r
            });

            let ab = app.run_task(async { task::with_deadline(ab, 20.secs()).await }).unwrap().unwrap();

            assert_eq!(ab, "ab");
        }
    }
}
