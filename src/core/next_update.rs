use super::{ColorF, FocusRequest, FontCache, FontInstanceRef, LayoutSize, NewWindow, Ui, Var, VarChange};

pub struct NextUpdate {
    pub(crate) fonts: FontCache,

    pub(crate) windows: Vec<NewWindow>,

    pub(crate) update_layout: bool,
    pub(crate) render_frame: bool,
    pub(crate) focus_request: Option<FocusRequest>,
    pub(crate) value_changes: Vec<Box<dyn VarChange>>,
    _request_close: bool,

    pub(crate) has_update: bool,
}

impl Default for NextUpdate {
    fn default() -> Self {
        NextUpdate {
            fonts: FontCache::default(),
            windows: vec![],

            update_layout: true,
            render_frame: true,
            value_changes: vec![],
            focus_request: None,
            _request_close: false,

            has_update: true,
        }
    }
}

impl NextUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_window<C: Ui + 'static>(
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

    pub fn set<T: 'static>(&mut self, value: &Var<T>, new_value: T) {
        self.change(value, |v| *v = new_value);
    }

    pub fn change<T: 'static>(&mut self, value: &Var<T>, change: impl FnOnce(&mut T) + 'static) {
        value.change_value(change);
        self.value_changes.push(Box::new(value.clone()));
        self.has_update = true;
    }

    pub fn font(&mut self, family: &str, size: u32) -> FontInstanceRef {
        let font = self.fonts.get(family, size);
        self.has_update |= self.fonts.has_load_requests();
        font
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
