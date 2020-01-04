use super::*;
pub use glutin::window::CursorIcon;
use webrender::api::*;

pub struct FrameBuilder {
    widget_id: WidgetId,
    cursor: CursorIcon,
}

impl FrameBuilder {
    pub fn new(root_id: WidgetId) -> Self {
        FrameBuilder {
            widget_id: root_id,
            cursor: CursorIcon::default(),
        }
    }

    fn item_tag(&self) -> ItemTag {
        (self.widget_id.get(), self.cursor as u16)
    }

    pub(crate) fn push_widget(&mut self, id: WidgetId, content: &impl UiNode) {
        let widget_hit = (id, u16::max_value());
        // self.push_hit_rect(widget_hit);

        let parent = std::mem::replace(&mut self.widget_id, id);
        content.render(self);
        self.widget_id = parent;
    }

    pub fn push_cursor(&mut self, cursor: CursorIcon, node: &impl UiNode) {
        let parent_cursor = std::mem::replace(&mut self.cursor, cursor);
        node.render(self);
        self.cursor = parent_cursor;
    }
}

fn is_widget(raw: u16) -> bool {
    raw == u16::max_value()
}

fn unpack_cursor(raw: u16) -> CursorIcon {
    debug_assert!(raw <= CursorIcon::RowResize as u16);

    if raw <= CursorIcon::RowResize as u16 {
        unsafe { std::mem::transmute(raw as u8) }
    } else {
        CursorIcon::Default
    }
}

pub struct Hit {
    pub widget_id: WidgetId,
    pub point: LayoutPoint,
}

pub struct Hits {
    hits: Vec<Hit>,
    cursor: CursorIcon,
}

impl Hits {
    #[inline]
    pub fn new(hits: HitTestResult) -> Self {
        // TODO solve: using the same WidgetId in multiple properties
        // will result in repeated entries here with potentially different
        // hit points, that don't match with the widget area.
        todo!()
    }

    #[inline]
    pub fn cursor(&self) -> CursorIcon {
        self.cursor
    }

    #[inline]
    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    #[inline]
    pub fn hit(&self, widget_id: WidgetId) -> &Hit {
        todo!()
    }
}
