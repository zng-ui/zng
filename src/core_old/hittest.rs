use super::CursorIcon;
use super::LayoutPoint;
use fnv::FnvHashMap;
use std::num::NonZeroU64;
use webrender::api::HitTestResult;

uid! {
    /// Hit-test tag.
    pub struct HitTag(_);
}

/// Hit test results.
#[derive(Default)]
pub struct Hits {
    points: FnvHashMap<HitTag, LayoutPoint>,
    cursor: CursorIcon,
}

impl Hits {
    pub fn new(hits: HitTestResult) -> Self {
        let cursor = hits
            .items
            .first()
            .map(|h| {
                if h.tag.1 <= CursorIcon::RowResize as u16 {
                    unsafe { std::mem::transmute(h.tag.1 as u8) }
                } else {
                    CursorIcon::Default
                }
            })
            .unwrap_or(CursorIcon::Default);

        Hits {
            points: hits
                .items
                .into_iter()
                .map(|h| {
                    (
                        HitTag(NonZeroU64::new(h.tag.0).expect("Invalid tag: 0")),
                        h.point_relative_to_item,
                    )
                })
                .collect(),
            cursor,
        }
    }

    pub fn point_over(&self, tag: HitTag) -> Option<LayoutPoint> {
        self.points.get(&tag).cloned()
    }

    pub fn cursor(&self) -> CursorIcon {
        self.cursor
    }
}
