use super::{ColorF, FocusRequest, FontInstance, FontInstances, LayoutSize, NewWindow, Ui, Var, VarChange};
use app_units::Au;
use fnv::FnvHashMap;
use font_loader::system_fonts;
use webrender::api::{DocumentId, RenderApi, Transaction};

pub struct NextUpdate {
    pub(crate) api: RenderApi,
    pub(crate) document_id: DocumentId,
    fonts: FnvHashMap<String, FontInstances>,
    pub(crate) windows: Vec<NewWindow>,

    pub(crate) update_layout: bool,
    pub(crate) render_frame: bool,
    pub(crate) focus_request: Option<FocusRequest>,
    pub(crate) value_changes: Vec<Box<dyn VarChange>>,
    _request_close: bool,

    pub(crate) has_update: bool,
}
impl NextUpdate {
    pub fn new(api: RenderApi, document_id: DocumentId) -> Self {
        NextUpdate {
            api,
            document_id,
            fonts: FnvHashMap::default(),
            windows: vec![],

            update_layout: true,
            render_frame: true,
            value_changes: vec![],
            focus_request: None,
            _request_close: false,

            has_update: false,
        }
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

    pub fn font(&mut self, family: &str, size: u32) -> FontInstance {
        let mut uncached_font = true;

        if let Some(font) = self.fonts.get(family) {
            if let Some(&instance_key) = font.instances.get(&size) {
                return FontInstance {
                    font_key: font.font_key,
                    instance_key,
                    size,
                };
            }
            uncached_font = false;
        }

        let mut txn = Transaction::new();

        if uncached_font {
            let property = system_fonts::FontPropertyBuilder::new().family(family).build();
            let (font, _) = system_fonts::get(&property).unwrap();

            let font_key = self.api.generate_font_key();
            txn.add_raw_font(font_key, font, 0);

            self.fonts.insert(
                family.to_owned(),
                FontInstances {
                    font_key,
                    instances: FnvHashMap::default(),
                },
            );
        }

        let f = self.fonts.get_mut(family).unwrap();

        let instance_key = self.api.generate_font_instance_key();
        txn.add_font_instance(
            instance_key,
            f.font_key,
            Au::from_px(size as i32),
            None,
            None,
            Vec::new(),
        );
        f.instances.insert(size, instance_key);

        self.api.send_transaction(self.document_id, txn);

        FontInstance {
            font_key: f.font_key,
            instance_key,
            size,
        }
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
