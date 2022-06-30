use crate::{units::*, widget_info::*, window::WindowId};

use super::*;

use pretty_assertions::assert_eq;

trait WidgetInfoBuilderExt {
    fn push_test_widget<F>(&mut self, name: &'static str, focus: FocusInfo, offset: PxVector, inner: F)
    where
        F: FnMut(&mut Self);
}
impl WidgetInfoBuilderExt for WidgetInfoBuilder {
    fn push_test_widget<F>(&mut self, name: &'static str, focus: FocusInfo, offset: PxVector, mut inner: F)
    where
        F: FnMut(&mut Self),
    {
        self.push_widget(
            WidgetId::named(name),
            WidgetBoundsInfo::new_test(PxRect::new(offset.to_point(), PxSize::new(Px(1), Px(1))), None),
            WidgetBorderInfo::new(),
            WidgetRenderInfo::new_test(None, None),
            |builder| {
                let meta = builder.meta().entry(FocusInfoKey).or_default();
                match focus {
                    FocusInfo::NotFocusable => {}
                    FocusInfo::Focusable {
                        tab_index,
                        skip_directional,
                    } => {
                        meta.tab_index = Some(tab_index);
                        meta.skip_directional = Some(skip_directional);
                    }
                    FocusInfo::FocusScope {
                        tab_index,
                        skip_directional,
                        tab_nav,
                        directional_nav,
                        on_focus,
                        alt,
                    } => {
                        meta.scope = Some(true);
                        meta.tab_index = Some(tab_index);
                        meta.skip_directional = Some(skip_directional);
                        meta.tab_nav = Some(tab_nav);
                        meta.directional_nav = Some(directional_nav);
                        meta.on_focus = on_focus;
                        meta.alt_scope = alt;
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

fn scope(tab_nav: TabNav, directional_nav: DirectionalNav, mut offset: impl FnMut() -> PxVector) -> WidgetInfoTree {
    let mut builder = WidgetInfoBuilder::new(
        WindowId::named("w"),
        WidgetId::named("w"),
        WidgetBoundsInfo::new_test(PxRect::new(PxPoint::zero(), PxSize::new(Px(20), Px(20))), None),
        WidgetBorderInfo::new(),
        WidgetRenderInfo::new_test(None, None),
        None,
    );
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
        PxVector::zero(),
        |builder| {
            builder.push_test_widget(
                "item-0",
                FocusInfo::Focusable {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                },
                offset(),
                |_| {},
            );
            builder.push_test_widget(
                "item-1",
                FocusInfo::Focusable {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                },
                offset(),
                |_| {},
            );
            builder.push_test_widget(
                "item-2",
                FocusInfo::Focusable {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                },
                offset(),
                |_| {},
            );
        },
    );
    builder.finalize().0
}

fn horizontal_offsets() -> impl FnMut() -> PxVector {
    let mut v = PxVector::zero();
    move || {
        v.x += Px(1);
        v
    }
}

fn vertical_offsets() -> impl FnMut() -> PxVector {
    let mut v = PxVector::zero();
    move || {
        v.x += Px(1);
        v
    }
}

#[test]
pub fn enabled_nav_cycle_0() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_0_vertical() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, vertical_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_1() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_1_vertical() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, vertical_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_2() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_cycle_2_vertical() {
    let tree = scope(TabNav::Cycle, DirectionalNav::Cycle, vertical_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_0() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_0_vertical() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, vertical_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_1() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_1_vertical() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, vertical_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_2() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_contained_2_vertical() {
    let tree = scope(TabNav::Contained, DirectionalNav::Contained, vertical_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_none_0() {
    let tree = scope(TabNav::None, DirectionalNav::None, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-0").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_none_1() {
    let tree = scope(TabNav::None, DirectionalNav::None, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-1").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}

#[test]
pub fn enabled_nav_none_2() {
    let tree = scope(TabNav::None, DirectionalNav::None, horizontal_offsets());
    let tree = FocusInfoTree::new(&tree, true);

    let item = tree.find("item-2").unwrap();

    let result = item.enabled_nav();
    let actual = item.actual_enabled_nav();

    assert_eq!(result, actual);
}
