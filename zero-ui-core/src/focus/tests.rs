use crate::{units::*, widget_info::*, widget_instance::ZIndex, window::WindowId};

use super::*;

use pretty_assertions::assert_eq;

trait WidgetInfoBuilderExt {
    fn push_test_widget<F>(&mut self, name: &'static str, focus: FocusInfo, rect: PxRect, inner: F)
    where
        F: FnMut(&mut Self);
}
impl WidgetInfoBuilderExt for WidgetInfoBuilder {
    fn push_test_widget<F>(&mut self, name: &'static str, focus: FocusInfo, rect: PxRect, mut inner: F)
    where
        F: FnMut(&mut Self),
    {
        self.push_widget(
            WidgetId::named(name),
            WidgetBoundsInfo::new_test(
                rect,
                None,
                None,
                Some(rect.origin.to_vector().into()),
                Some(WidgetRenderInfo {
                    visible: true,
                    back: ZIndex::BACK,
                    front: ZIndex::BACK,
                }),
            ),
            WidgetBorderInfo::new(),
            |builder| {
                let mut meta = FocusInfoBuilder::get(builder);
                match focus {
                    FocusInfo::NotFocusable => {}
                    FocusInfo::Focusable {
                        tab_index,
                        skip_directional,
                    } => {
                        meta.tab_index(tab_index);
                        meta.skip_directional(skip_directional);
                    }
                    FocusInfo::FocusScope {
                        tab_index,
                        skip_directional,
                        tab_nav,
                        directional_nav,
                        on_focus,
                        alt,
                    } => {
                        meta.scope(true);
                        meta.tab_index(tab_index);
                        meta.skip_directional(skip_directional);
                        meta.tab_nav(tab_nav);
                        meta.directional_nav(directional_nav);
                        meta.on_focus(on_focus);
                        meta.alt_scope(alt);
                    }
                }
                inner(builder);
            },
        )
    }
}

trait WidgetFocusInfoExt {
    fn test_name(self) -> &'static str;

    fn actual_enabled_nav(self) -> FocusNavAction;
}
impl<'a> WidgetFocusInfoExt for WidgetFocusInfo<'a> {
    fn test_name(self) -> &'static str {
        self.info
            .widget_id()
            .name()
            .as_static_str()
            .expect("use with `push_test_widget` only")
    }

    fn actual_enabled_nav(self) -> FocusNavAction {
        let mut nav = FocusNavAction::all();

        nav.set(FocusNavAction::EXIT, self.parent().is_some() || self.is_alt_scope());
        nav.set(FocusNavAction::ENTER, self.descendants().next().is_some());

        nav.set(FocusNavAction::NEXT, self.next_tab(false).is_some());
        nav.set(FocusNavAction::PREV, self.prev_tab(false).is_some());

        nav.set(FocusNavAction::UP, self.next_up().is_some());
        nav.set(FocusNavAction::RIGHT, self.next_right().is_some());
        nav.set(FocusNavAction::DOWN, self.next_down().is_some());
        nav.set(FocusNavAction::LEFT, self.next_left().is_some());

        nav.set(FocusNavAction::ALT, self.in_alt_scope() || self.alt_scope().is_some());

        nav
    }
}

fn scope(tab_nav: TabNav, directional_nav: DirectionalNav, horizontal: bool) -> WidgetInfoTree {
    let mut builder = WidgetInfoBuilder::new(
        WindowId::named("w"),
        WidgetId::named("w"),
        WidgetBoundsInfo::new_test(
            PxRect::from_size(PxSize::new(Px(800), Px(600))),
            None,
            None,
            None,
            Some(WidgetRenderInfo {
                visible: true,
                back: ZIndex::BACK,
                front: ZIndex::BACK,
            }),
        ),
        WidgetBorderInfo::new(),
        1.fct(),
        None,
    );
    FocusInfoBuilder::get(&mut builder)
        .scope(true)
        .tab_nav(TabNav::Cycle)
        .directional_nav(DirectionalNav::Cycle);

    let mut v = PxVector::zero();
    let mut rect = move || {
        let point = v.to_point();
        if horizontal {
            v.x += Px(25);
        } else {
            v.y += Px(25);
        }
        PxRect::new(point, PxSize::new(Px(20), Px(20)))
    };

    builder.push_test_widget(
        "scope",
        FocusInfo::FocusScope {
            tab_index: TabIndex::AUTO,
            skip_directional: false,
            tab_nav,
            directional_nav,
            on_focus: FocusScopeOnFocus::Widget,
            alt: false,
        },
        PxRect::new(
            PxPoint::new(Px(100), Px(100)),
            if horizontal {
                PxSize::new(Px(25 * 3), Px(20))
            } else {
                PxSize::new(Px(20), Px(25 * 3))
            },
        ),
        |builder| {
            builder.push_test_widget(
                "item-0",
                FocusInfo::Focusable {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                },
                rect(),
                |_| {},
            );
            builder.push_test_widget(
                "item-1",
                FocusInfo::Focusable {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                },
                rect(),
                |_| {},
            );
            builder.push_test_widget(
                "item-2",
                FocusInfo::Focusable {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                },
                rect(),
                |_| {},
            );
        },
    );
    builder.finalize().0
}

#[test]
pub fn enabled_nav_cycle_0_horizontal() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_0_vertical() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, false);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_1_horizontal() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_1_vertical() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, false);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_2_horizontal() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_2_vertical() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, false);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_0_horizontal() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_0_vertical() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, false);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_1_horizontal() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_1_vertical() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, false);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_2_horizontal() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_2_vertical() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, false);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_none_0() {
    let tree = scope(TabNav::None, DirectionalNav::None, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_none_1() {
    let tree = scope(TabNav::None, DirectionalNav::None, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_none_2() {
    let tree = scope(TabNav::None, DirectionalNav::None, true);
    let tree = FocusInfoTree::new(&tree, true, true);

    let item = tree.get("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}
