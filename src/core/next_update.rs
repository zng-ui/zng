use super::{
    ColorF, FocusRequest, FontCache, FontInstance, LayoutSize, NewWindow, Ui, UiItemId, ValueMut, ValueMutCommit,
};
use webrender::api::RenderApiSender;

pub struct NextUpdate {
    pub(crate) fonts: FontCache,

    pub(crate) windows: Vec<NewWindow>,

    pub(crate) update_layout: bool,
    pub(crate) render_frame: bool,
    pub(crate) focus_request: Option<FocusRequest>,
    pub(crate) var_changes: Vec<Box<dyn ValueMutCommit>>,

    pub(crate) mouse_capture_request: Option<EventCaptureRequest>,

    _request_close: bool,

    pub(crate) has_update: bool,
}

pub(crate) enum EventCaptureRequest {
    Capture(UiItemId),
    Release(UiItemId),
}

impl NextUpdate {
    pub fn new(sender: RenderApiSender) -> Self {
        NextUpdate {
            fonts: FontCache::new(sender),
            windows: vec![],

            update_layout: true,
            render_frame: true,
            var_changes: vec![],
            focus_request: None,
            mouse_capture_request: None,
            _request_close: false,

            has_update: true,
        }
    }

    pub fn create_window<C: Ui>(
        &mut self,
        clear_color: ColorF,
        inner_size: LayoutSize,
        content: impl FnOnce(&mut NextUpdate) -> C + 'static,
    ) {
        self.windows.push(NewWindow {
            content: Box::new(move |c| content(c).into_box()),
            clear_color,
            inner_size,
        });
        self.has_update = true;
    }

    pub fn update_layout(&mut self) {
        self.update_layout = true;
        self.has_update = true;
    }
    pub fn render_frame(&mut self) {
        self.render_frame = true;
        self.has_update = true;
    }

    pub fn focus(&mut self, request: FocusRequest) {
        self.focus_request = Some(request);
        self.has_update = true;
    }

    /// On next update tries to capture mouse events. When captured
    /// only mouse events inside `item` are invoked and mouse_move event is
    /// received even when the mouse is not over the item.
    pub fn capture_mouse(&mut self, item: UiItemId) {
        self.mouse_capture_request = Some(EventCaptureRequest::Capture(item));
    }

    /// On next update releases mouse capture if it is captured by the same `item`.
    pub fn release_mouse(&mut self, item: UiItemId) {
        self.mouse_capture_request = Some(EventCaptureRequest::Release(item));
    }

    pub fn set<T: 'static>(&mut self, value: &impl ValueMut<T>, new_value: T) {
        self.change(value, |v| *v = new_value);
    }

    pub fn change<T: 'static>(&mut self, value: &impl ValueMut<T>, change: impl FnOnce(&mut T) + 'static) {
        value.change_value(change);
        self.var_changes.push(Box::new(value.clone()));
        self.has_update = true;
    }

    pub fn font(&mut self, family: &str, size: u32) -> FontInstance {
        self.fonts.get(family, size)
    }

    //-------idea---------
    //
    //pub fn close_app(&mut self) {
    //    self.close = Some(CloseRequest::App);
    //}

    //pub fn cancel_close(&mut self) {
    //    self.cancel_close = true;
    //}

    //pub fn set_window_title(&mut self, title: String) {
    //    self.new_window_title = Some(title);
    //}

    //pub fn start_work(&mut self, work: impl FnOnce() + 'static) -> WorkKey {
    //    let key = self.next_work_key;
    //    self.new_work.push((key, Box::new(work)));
    //    self.next_work_key = WorkKey(key.0.wrapping_add(1));
    //    key
    //}

    //pub fn cancel_work(&mut self, work_key: WorkKey) {
    //    self.cancel_work.push(work_key)
    //}
}
