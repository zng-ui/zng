#![cfg(test)]

use super::*;

mod any {
    use super::*;

    #[test]
    fn downcast_ref_rc() {
        let any_var = var(true).boxed_any();
        assert!(any_var.as_any().downcast_ref::<ArcVar<bool>>().is_some())
    }

    #[test]
    fn downcast_ref_boxed() {
        let any_var = var(true).boxed().boxed_any();
        assert!(any_var.as_any().downcast_ref::<ArcVar<bool>>().is_some())
    }

    #[test]
    fn downcast_ref_context_var() {
        context_var! {
            static FOO_VAR: bool = true;
        }
        let any_var = FOO_VAR.boxed_any();
        assert!(any_var.as_any().downcast_ref::<ContextVar<bool>>().is_some());
    }

    #[test]
    fn downcast_double_boxed() {
        let any_var = var(true).boxed_any().double_boxed_any();
        assert!(any_var.downcast_ref::<BoxedVar<bool>>().is_some())
    }

    #[test]
    fn downcast_rc() {
        let any_var = var(true).boxed_any();
        let any_box = any_var.as_any();
        assert!(any_box.downcast_ref::<ArcVar<bool>>().is_some());
    }

    #[test]
    fn downcast_boxed() {
        let any_var = var(true).boxed().boxed_any();
        let any_box = any_var.as_any();
        assert!(any_box.downcast_ref::<ArcVar<bool>>().is_some());
    }
}

mod bindings {
    use super::*;
    use crate::app::App;
    use crate::text::ToText;

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_map(&b, |a| a.to_text()).perm();

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
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert!(updated);

        a.set(13);

        updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(13i32), a.get_new());
                assert_eq!(Some("13".to_text()), b.get_new());
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn two_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_map_bidi(&b, |a| a.to_text(), |b| b.parse().unwrap()).perm();

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
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert!(updated);

        b.set("55");

        updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some("55".to_text()), b.get_new());
                assert_eq!(Some(55i32), a.get_new());
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn one_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_filter_map(&b, |a| if *a == 13 { None } else { Some(a.to_text()) }).perm();

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
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert!(updated);

        a.set(13);

        updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(13i32), a.get_new());
                assert_eq!("20".to_text(), b.get());
                assert!(!b.is_new());
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn two_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::minimal().run_headless(false);

        a.bind_filter_map_bidi(&b, |a| Some(a.to_text()), |b| b.parse().ok()).perm();

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
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(20i32), a.get_new());
                assert_eq!(Some("20".to_text()), b.get_new());
            },
            false,
        );
        assert!(updated);

        b.set("55");

        updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some("55".to_text()), b.get_new());
                assert_eq!(Some(55i32), a.get_new());
            },
            false,
        );
        assert!(updated);

        b.set("not a i32");

        updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some("not a i32".to_text()), b.get_new());
                assert_eq!(55i32, a.get());
                assert!(!a.is_new());
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

        let mut app = App::minimal().run_headless(false);

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
                assert!(!updated, "expected one update");
                updated = true;

                assert_eq!(Some(20), a.get_new());
                assert_eq!(Some(21), b.get_new());
                assert_eq!(Some(22), c.get_new());
                assert_eq!(Some(23), d.get_new());
            },
            false,
        );
        assert!(updated);

        a.set(30);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;

                assert_eq!(Some(30), a.get_new());
                assert_eq!(Some(31), b.get_new());
                assert_eq!(Some(32), c.get_new());
                assert_eq!(Some(33), d.get_new());
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

        let mut app = App::minimal().run_headless(false);

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
                assert!(!updated, "expected one update");
                updated = true;

                assert_eq!(Some(20), a.get_new());
                assert_eq!(Some(20), b.get_new());
                assert_eq!(Some(20), c.get_new());
                assert_eq!(Some(20), d.get_new());
            },
            false,
        );
        assert!(updated);

        d.set(30);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;

                assert_eq!(Some(30), a.get_new());
                assert_eq!(Some(30), b.get_new());
                assert_eq!(Some(30), c.get_new());
                assert_eq!(Some(30), d.get_new());
            },
            false,
        );
        assert!(updated);
    }

    #[test]
    fn binding_drop() {
        let a = var(1);
        let b = var(1);

        let mut app = App::minimal().run_headless(false);

        let handle = a.bind_map(&b, |i| *i + 1);

        a.set(10);

        let mut updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;

                assert_eq!(Some(10), a.get_new());
                assert_eq!(Some(11), b.get_new());
            },
            false,
        );
        assert!(updated);

        drop(handle);

        a.set(100);

        updated = false;
        let _ = app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;

                assert_eq!(Some(100), a.get_new());
                assert!(!b.is_new());
                assert_eq!(11, b.get());
            },
            false,
        );
        assert!(updated);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());
    }

    #[test]
    fn binding_bidi_set_both() {
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

        let a = var(0);
        let b = var(0);

        a.set(1);
        b.set_from(&a);
        a.bind(&b).perm();

        app.update(false).assert_wait();

        assert_eq!(1, b.get());
    }
}

mod context {
    use when::AnyWhenVarBuilder;

    use crate::{app::*, context::*, text::*, var::*, widget_instance::*, *};

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
    fn on_init(child: impl UiNode, mut handler: impl handler::WidgetHandler<()>) -> impl UiNode {
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

    #[widget($crate::var::tests::context::TestWgt)]
    struct TestWgt(crate::widget_base::WidgetBase);
    impl TestWgt {
        fn widget_intrinsic(&mut self) {
            self.widget_builder().push_build_action(|wgt| {
                if let Some(child) = wgt.capture_ui_node(property_id!(child)) {
                    wgt.set_child(child);
                }
            });
        }
    }
    use widget_base::child;

    fn test_app(app: AppExtended<impl AppExtension>, root: impl UiNode) -> HeadlessApp {
        test_log();

        use crate::window::*;
        let mut app = app.run_headless(false);
        WINDOWS.open(async move { crate::window::WindowRoot::new_test(root) });
        let _ = app.update(false);
        app
    }

    #[test]
    fn context_var_basic() {
        let _test = test_app(
            App::default(),
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
            App::default(),
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
        let app = App::default();

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
        let app = App::default();
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
        let app = App::default();

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
        let _app = App::minimal();

        let mapped = TEST_VAR.map(|t| formatx!("map {t}"));
        let a = TEST_VAR.with_context_var(ContextInitHandle::new(), "A", || mapped.get());

        let b = TEST_VAR.with_context_var(ContextInitHandle::new(), "B", || mapped.get());

        assert_ne!(a, b);
    }

    #[test]
    fn context_var_cloned_same_widget() {
        let app = App::default();

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
        let mut app = test_app(App::default(), NilUiNode);

        let backing_var = var(Txt::from(""));

        TEST_VAR.with_context_var(ContextInitHandle::new(), backing_var.clone(), || {
            let t = TEST_VAR;
            assert!(t.capabilities().contains(VarCapabilities::MODIFY));
            t.set("set!").unwrap();
        });

        let _ = app.update(false);
        assert_eq!(backing_var.get(), "set!");
    }

    #[test]
    fn context_var_binding() {
        let app = App::default();

        let input_var = var("Input!".to_text());
        let other_var = var(".".to_text());

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
        let _scope = App::minimal();

        let var = when_var! {
            false => var("hello".to_text()),
            _ => TEST_VAR,
        };

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_when2() {
        let _scope = App::minimal();

        let var = when_var! {
            true => TEST_VAR,
            _ => var("hello".to_text()),
        };

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_issue_when_any() {
        let _scope = App::minimal();

        let mut var = AnyWhenVarBuilder::new(TEST_VAR);
        var.push(self::var(false), self::var("hello".to_text()));
        let var = var.contextualized_build().unwrap();

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }

    #[test]
    fn context_var_recursion_merge() {
        let _scope = App::minimal();

        let var = merge_var!(TEST_VAR, var(true), |t, _| t.clone());

        let r = TEST_VAR.with_context_var(ContextInitHandle::new(), var.clone(), || var.get());

        assert_eq!("", r);
    }
}

mod flat_map {
    use crate::{app::App, var::*};
    use std::fmt;

    #[derive(Clone)]
    pub struct Foo {
        pub bar: bool,
        pub var: ArcVar<usize>,
    }
    impl PartialEq for Foo {
        fn eq(&self, other: &Self) -> bool {
            self.bar == other.bar && self.var.var_ptr() == other.var.var_ptr()
        }
    }
    impl fmt::Debug for Foo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Foo").field("bar", &self.bar).finish_non_exhaustive()
        }
    }

    #[test]
    pub fn flat_map() {
        let mut app = App::minimal().run_headless(false);

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
    use crate::{app::App, text::Txt, var::*};

    #[test]
    pub fn set_same_importance() {
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

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
        let mut app = App::minimal().run_headless(false);

        let test = var(Txt::from_static("v0"));
        let ease = var(0i32);
        test.hook(Box::new(clmv!(ease, |_| {
            ease.ease(100, 10.ms(), easing::linear).perm();
            false // once
        })))
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
    use crate::app::App;

    use super::*;

    #[test]
    pub fn cow_base_update() {
        let mut app = App::minimal().run_headless(false);

        let base = var(false);
        let cow = base.cow();

        base.set(true);
        app.update(false).assert_wait();

        assert!(base.get());
        assert!(cow.get());
    }

    #[test]
    pub fn cow_update() {
        let mut app = App::minimal().run_headless(false);

        let base = var(false);
        let cow = base.cow();

        cow.set(true);
        app.update(false).assert_wait();

        assert!(!base.get());
        assert!(cow.get());
    }

    #[test]
    pub fn cow_update_full() {
        let mut app = App::minimal().run_headless(false);

        let base = var(false);
        let cow = base.cow();

        let base_values = Arc::new(Mutex::new(vec![]));
        let cow_values = Arc::new(Mutex::new(vec![]));
        base.trace_value(clmv!(base_values, |v| base_values.lock().push(v.value))).perm();
        cow.trace_value(clmv!(cow_values, |v| cow_values.lock().push(v.value))).perm();

        base.set(true);
        app.update(false).assert_wait();

        assert!(base.get());
        assert!(cow.get());

        cow.set(false);
        app.update(false).assert_wait();

        assert!(base.get());
        assert!(!cow.get());

        assert_eq!(&base_values.lock()[..], &[false, true]);
        assert_eq!(&cow_values.lock()[..], &[false, true, false]);

        base.set(true);
        app.update(false).assert_wait();
        base.set(false);
        app.update(false).assert_wait();
        assert_eq!(&base_values.lock()[..], &[false, true, true, false]);
        assert_eq!(&cow_values.lock()[..], &[false, true, false]);
    }
}

mod multi {
    use crate::app::App;

    use super::*;

    #[test]
    fn multi_bidi() {
        let mut app = App::minimal().run_headless(false);

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
        a.trace_value(clmv!(a_values, |v| a_values.lock().push(v.value))).perm();
        b.trace_value(clmv!(b_values, |v| b_values.lock().push(v.value))).perm();

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
