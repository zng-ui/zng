use super::*;
use crate::app::Focused;
use crate::properties::{FocusScope, FocusScopeConfig};
use webrender::api::*;

/// Root of an [Ui] tree. This is usually only used internally,
/// but can be used directly to implement a custom render target or
/// for testing.
pub struct UiRoot {
    api: RenderApi,

    pipeline_id: PipelineId,
    document_id: DocumentId,

    size: LayoutSize,
    dpi_factor: f32,

    content: FocusScope<Box<dyn Ui>>,
    // size of last render
    content_size: LayoutSize,

    next_update: NextUpdate,
    ui_values: UiValues,
    focus_map: FocusMap,

    mouse_pos: LayoutPoint,
    key_down: Option<ScanCode>,
    cursor: CursorIcon,

    set_cursor: Option<CursorIcon>,

    prev_frame_data_len: usize,

    focused: Option<FocusKey>,
    // `focused` in the new frame. If it does not exist
    // closest sibling or parent.
    focused_coerced: Option<FocusKey>,

    latest_frame_id: Epoch,
}

impl UiRoot {
    #[inline]
    pub fn new(
        api: RenderApi,
        api_sender: RenderApiSender,
        initial_size: LayoutSize,
        initial_dpi_factor: f32,
        init: Box<dyn FnOnce(&mut NextUpdate) -> Box<dyn Ui>>,
    ) -> Self {
        let device_size = {
            let size: LayoutSize = initial_size * euclid::TypedScale::new(initial_dpi_factor);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };

        let document_id = api.add_document(device_size, 0);

        let mut ui_values = UiValues::new(UiItemId::new_unique(), FocusKey::new_unique(), None);
        let mut next_update = NextUpdate::new(api_sender);

        let content = init(&mut next_update);

        let mut content = FocusScope::new(
            content,
            FocusScopeConfig::new()
                .tab_nav_cycle()
                .directional_nav_cycle()
                .remember_focus(true)
                .key(ui_values.window_focus_key()),
        );

        content.init(&mut ui_values, &mut next_update);

        UiRoot {
            api,
            pipeline_id: PipelineId(PipelineNamespace::new_unique().get() as u32, 0),
            document_id,
            size: initial_size,
            dpi_factor: initial_dpi_factor,
            content,
            content_size: LayoutSize::default(),
            ui_values,
            next_update,
            focus_map: FocusMap::new(),
            mouse_pos: LayoutPoint::new(-1., -1.),
            key_down: None,
            cursor: CursorIcon::Default,
            prev_frame_data_len: 0,
            focused: None,
            focused_coerced: None,
            latest_frame_id: Epoch(0),
            set_cursor: None,
        }
    }

    #[inline]
    pub fn cursor(&self) -> CursorIcon {
        self.cursor
    }

    #[inline]
    pub fn resize(&mut self, new_size: LayoutSize) {
        if self.size != new_size {
            self.size = new_size;
            self.next_update.update_layout();
        }
    }

    #[inline]
    pub fn set_dpi_factor(&mut self, new_dpi_factor: f32) {
        if (self.dpi_factor - new_dpi_factor).abs() > 0.01 {
            self.dpi_factor = new_dpi_factor;
            self.next_update.update_layout();
        }
    }

    #[inline]
    pub fn keyboard_input(
        &mut self,
        scancode: ScanCode,
        state: ElementState,
        virtual_keycode: Option<VirtualKeyCode>,
        modifiers: ModifiersState,
    ) {
        let is_pressed = state == ElementState::Pressed;
        // check if is auto repeat.
        let mut repeat = false;
        if is_pressed {
            if self.key_down != Some(scancode) {
                self.key_down = Some(scancode);
            } else {
                repeat = true;
            }
        } else {
            self.key_down = None;
        }
        let keyboard_input = KeyboardInput {
            scancode,
            state,
            virtual_keycode,
            modifiers,
            repeat,
        };

        // notify content
        self.content
            .keyboard_input(&keyboard_input, &mut self.ui_values, &mut self.next_update);

        // do default focus navigation
        if is_pressed && self.next_update.focus_request.is_none() && self.ui_values.child(*FOCUS_HANDLED).is_none() {
            static SHIFT_ONLY: ModifiersState = ModifiersState {
                shift: true,
                alt: false,
                ctrl: false,
                logo: false,
            };

            let request = if modifiers == ModifiersState::default() {
                match virtual_keycode {
                    Some(VirtualKeyCode::Tab) => Some(FocusRequest::Next),
                    Some(VirtualKeyCode::Left) => Some(FocusRequest::Left),
                    Some(VirtualKeyCode::Right) => Some(FocusRequest::Right),
                    Some(VirtualKeyCode::Up) => Some(FocusRequest::Up),
                    Some(VirtualKeyCode::Down) => Some(FocusRequest::Down),
                    _ => None,
                }
            } else if modifiers == SHIFT_ONLY {
                match virtual_keycode {
                    Some(VirtualKeyCode::Tab) => Some(FocusRequest::Prev),
                    _ => None,
                }
            } else {
                None
            };

            if let Some(request) = request {
                self.next_update.focus(request);
            }
        }

        // clear all child values
        self.ui_values.clear_child_values();
    }

    #[inline]
    pub fn mouse_move(&mut self, position: LayoutPoint, modifiers: ModifiersState) {
        if self.mouse_pos != position {
            let hit = self.hit_test(self.mouse_pos);
            self.mouse_pos = position;
            self.set_cursor(hit.cursor());
            self.content.mouse_move(
                &UiMouseMove { position, modifiers },
                &hit,
                &mut self.ui_values,
                &mut self.next_update,
            );

            self.ui_values.clear_child_values();
        }
    }

    #[inline]
    pub fn mouse_entered(&mut self) {
        self.content.mouse_entered(&mut self.ui_values, &mut self.next_update);
    }

    #[inline]
    pub fn mouse_left(&mut self) {
        self.set_cursor(CursorIcon::Default);
        self.content.mouse_left(&mut self.ui_values, &mut self.next_update);
    }

    #[inline]
    pub fn mouse_input(&mut self, state: ElementState, button: MouseButton, modifiers: ModifiersState) {
        self.content.mouse_input(
            &MouseInput {
                state,
                button,
                modifiers,
                position: self.mouse_pos,
            },
            &self.hit_test(self.mouse_pos),
            &mut self.ui_values,
            &mut self.next_update,
        );
        self.ui_values.clear_child_values()
    }

    #[inline]
    pub fn window_focused(&mut self, focused: bool) {
        self.content
            .window_focused(focused, &mut self.ui_values, &mut self.next_update);

        if focused {
            if self.next_update.focus_request.is_none() {
                self.next_update
                    .focus(FocusRequest::Direct(self.ui_values.window_focus_key()));
            }
        } else {
            self.key_down = None;
        }

        self.ui_values.clear_child_values();
    }

    #[inline]
    pub fn hit_test(&self, point: LayoutPoint) -> Hits {
        Hits::new(self.api.hit_test(
            self.document_id,
            Some(self.pipeline_id),
            WorldPoint::new(point.x, point.y),
            HitTestFlags::FIND_ALL,
        ))
    }

    #[inline]
    pub fn has_update(&self) -> bool {
        self.next_update.has_update || self.set_cursor.is_some()
    }

    #[inline]
    pub fn take_new_window_requests(&mut self) -> Vec<NewWindow> {
        std::mem::replace(&mut self.next_update.windows, vec![])
    }

    #[inline]
    pub fn take_var_changes(&mut self) -> Vec<Box<dyn ValueMutCommit>> {
        std::mem::replace(&mut self.next_update.var_changes, vec![])
    }

    #[inline]
    pub fn take_switch_changes(&mut self) -> Vec<Box<dyn SwitchCommit>> {
        std::mem::replace(&mut self.next_update.switch_changes, vec![])
    }

    #[inline]
    pub fn take_set_cursor(&mut self) -> Option<CursorIcon> {
        if let Some(cursor) = self.set_cursor.take() {
            self.cursor = cursor;
            Some(cursor)
        } else {
            None
        }
    }

    /// Applies all Ui updates and send a new frame request.
    ///
    /// If during the update process before layout and render
    /// more `NextUpdate` changes are requested the function does
    /// not do layout and render and returns [UiUpdateResult::CausedMoreUpdates].
    #[inline]
    pub fn update(&mut self, values_changed: bool, focused: Focused, first_draw: bool) -> UiUpdateResult {
        if self.next_update.has_update || values_changed {
            self.next_update.has_update = false;

            if values_changed {
                self.content.value_changed(&mut self.ui_values, &mut self.next_update);
            }
            if !first_draw {
                self.update_focus(focused);
            }

            if self.next_update.has_update {
                return UiUpdateResult::CausedMoreUpdates;
            }

            self.update_layout();
            self.send_render_frame();
        }

        UiUpdateResult::Completed
    }

    #[inline]
    pub fn device_size(&self) -> DeviceIntSize {
        let size: LayoutSize = self.size * euclid::TypedScale::new(self.dpi_factor);
        DeviceIntSize::new(size.width as i32, size.height as i32)
    }

    fn set_cursor(&mut self, cursor: CursorIcon) {
        if self.cursor != cursor {
            self.set_cursor = Some(cursor);
        }
    }

    /// Updates the content layout and flags `render_frame`.
    fn update_layout(&mut self) {
        if !self.next_update.update_layout {
            return;
        }
        self.next_update.update_layout = false;

        let device_size = self.device_size();

        self.api.set_window_parameters(
            self.document_id,
            device_size,
            DeviceIntRect::from_size(device_size),
            self.dpi_factor,
        );

        self.content_size = self.content.measure(self.size).min(self.size);
        self.content.arrange(self.content_size);

        self.next_update.render_frame();
    }

    fn update_focus(&mut self, focused: Focused) {
        if let Some(request) = self.next_update.focus_request.take() {
            let new_focused = self.focus_map.focus(focused.get(), request);
            self.focused = new_focused;
            self.focused_coerced = new_focused;

            if new_focused != focused.get() {
                self.content.focus_changed(
                    &FocusChange::new(focused.get(), new_focused),
                    &mut self.ui_values,
                    &mut self.next_update,
                );
                focused.set(new_focused);
            }
        } else if self.focused.is_some() && focused.get() == self.focused {
            // if window has focused element and global focused is that element.

            if self.focused != self.focused_coerced {
                // but the window new frame no longer contains that element.

                // update to closest sibling or parent.

                self.content.focus_changed(
                    &FocusChange::new(self.focused, self.focused_coerced),
                    &mut self.ui_values,
                    &mut self.next_update,
                );

                focused.set(self.focused_coerced);
                self.focused = self.focused_coerced;
            }
        }
    }

    /// Generates window content display list and sends a new frame request to webrender.
    /// Webrender will request a redraw when the frame is done.
    fn send_render_frame(&mut self) {
        if !self.next_update.render_frame {
            return;
        }
        self.next_update.render_frame = false;

        let mut txn = Transaction::new();

        let mut frame = NextFrame::new(
            DisplayListBuilder::with_capacity(self.pipeline_id, self.size, self.prev_frame_data_len),
            SpatialId::root_reference_frame(self.pipeline_id),
            self.content_size,
            FocusMap::with_capacity(self.focus_map.len()),
        );

        self.content.render(&mut frame);

        self.latest_frame_id = Epoch({
            let mut next = self.latest_frame_id.0.wrapping_add(1);
            if next == Epoch::invalid().0 {
                next = next.wrapping_add(1);
            }
            next
        });

        let (display_list_data, focus_map) = frame.finalize();
        self.prev_frame_data_len = display_list_data.2.data().len();

        if let Some(f) = self.focused {
            self.focused_coerced = self.focus_map.closest_existing(f, &focus_map);
        }
        self.focus_map = focus_map;

        txn.set_display_list(self.latest_frame_id, None, self.size, display_list_data, true);
        txn.set_root_pipeline(self.pipeline_id);
        txn.generate_frame();
        self.api.send_transaction(self.document_id, txn);
    }
}

/// Result of the [UiRoot::update] operation.
pub enum UiUpdateResult {
    /// Updated completed, a new frame request was made.
    Completed,
    /// Update not completed, no frame request was made, update
    /// must be called again.
    CausedMoreUpdates,
}

uid! {
    struct PipelineNamespace(_);
}
