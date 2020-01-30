use zero_ui::core::app::*;
use zero_ui::core::context::*;
use zero_ui::core::types::*;

struct PrintDeviceKeyPresses;

impl AppExtension for PrintDeviceKeyPresses {
    fn on_device_event(&mut self, _: DeviceId, event: &DeviceEvent, _: &mut AppContext) {
        if let DeviceEvent::Key(i) = event {
            if i.virtual_keycode == Some(VirtualKeyCode::Escape) {
                std::process::exit(0)
            }
            if i.state == ElementState::Pressed {
                println!("scancode: {:?} key: {:?}", i.scancode, i.virtual_keycode);
            }
        }
    }
}

fn main() {
    App::empty().extend(PrintDeviceKeyPresses).run(|_| {});
}
