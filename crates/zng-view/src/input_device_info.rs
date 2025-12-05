use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Devices::HumanInterfaceDevice::GUID_DEVINTERFACE_HID;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError, HANDLE};
use windows::Win32::UI::Input::*;
use zng_txt::{Txt, formatx};
use zng_view_api::raw_input::{InputDeviceCapability, InputDeviceInfo};

fn device_name(h_device: HANDLE) -> Option<String> {
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getrawinputdeviceinfow

    let mut size = 0u32;
    // SAFETY: Function is called according to the documentation.
    let r = unsafe { GetRawInputDeviceInfoW(Some(h_device), RIDI_DEVICENAME, None, &mut size) };
    if r == 0 && size > 0 {
        let mut buffer: Vec<u16> = vec![0; size as usize];
        // SAFETY: Function is called according to the documentation.
        let r = unsafe { GetRawInputDeviceInfoW(Some(h_device), RIDI_DEVICENAME, Some(buffer.as_mut_ptr() as *mut _), &mut size) };
        if r > 0 {
            if let Some(pos) = buffer.iter().position(|&c| c == 0) {
                buffer.truncate(pos);
            }
            return Some(String::from_utf16_lossy(&buffer));
        }
    }
    None
}

fn device_info_by_handle(h_device: HANDLE) -> Option<RID_DEVICE_INFO> {
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getrawinputdeviceinfow

    let mut device_info = RID_DEVICE_INFO {
        cbSize: std::mem::size_of::<RID_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    let mut cb_size = device_info.cbSize;
    // SAFETY: Function is called according to the documentation.
    let r = unsafe {
        GetRawInputDeviceInfoW(
            Some(h_device),
            RIDI_DEVICEINFO,
            Some(&mut device_info as *mut _ as *mut _),
            &mut cb_size,
        )
    };
    if r > 0 { Some(device_info) } else { None }
}

fn device_info(device_path: &str) -> Option<RID_DEVICE_INFO> {
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getrawinputdevicelist

    let mut num_devices = 5u32;
    let mut devices: Vec<RAWINPUTDEVICELIST> = vec![Default::default(); num_devices as usize];
    for _ in 0..5 {
        // SAFETY: Function is called according to the documentation.
        let r = unsafe {
            GetRawInputDeviceList(
                Some(devices.as_mut_ptr()),
                &mut num_devices,
                std::mem::size_of::<RAWINPUTDEVICELIST>() as u32,
            )
        };

        if r == -1i32 as u32 {
            //  SAFETY: GetLastError is safe to call here
            let e = unsafe { GetLastError() };
            if e != ERROR_INSUFFICIENT_BUFFER {
                tracing::error!("GetRawInputDeviceList error, {}", e.to_hresult());
                continue; // retry
            }

            // if there are more devices them can fit on the buffer:
            // "the function returns the actual number of devices in this variable and fails with ERROR_INSUFFICIENT_BUFFER"
            devices.resize(num_devices as usize, Default::default());
            continue;
        } else if r > 0 {
            if r > num_devices {
                tracing::error!("GetRawInputDeviceList unexpected return, {r} > num_devices");
                continue; // retry
            }
            // if there are less devices them the buffer:
            // "the number of devices is reported as the return value"
            num_devices = r;
            devices.truncate(num_devices as usize);
            break;
        } else {
            break;
        }
    }

    for dev in devices {
        let h_device = dev.hDevice;
        if let Some(name) = device_name(h_device)
            && name.eq_ignore_ascii_case(device_path)
        {
            return device_info_by_handle(h_device);
        }
    }

    None
}

pub fn get(device_path: &str) -> InputDeviceInfo {
    if let Some(info) = device_info(device_path) {
        match info.dwType {
            RIM_TYPEKEYBOARD => {
                let name = if device_path.starts_with(r"\\?\HID#")
                    && let Some(name) = hid_description(device_path)
                {
                    name
                } else {
                    Txt::from_static("Keyboard")
                };

                return InputDeviceInfo::new(name, InputDeviceCapability::KEY);
            }
            RIM_TYPEMOUSE => {
                let mut cap = InputDeviceCapability::POINTER_MOTION;
                // winit generates axis motion events for mouse move
                cap |= InputDeviceCapability::AXIS_MOTION;
                // Windows API just assumes mouses have vertical scroll wheels.
                cap |= InputDeviceCapability::SCROLL_MOTION;

                // SAFETY: dwType defines the union field
                let mouse = unsafe { info.Anonymous.mouse };
                if mouse.dwNumberOfButtons > 0 {
                    cap |= InputDeviceCapability::BUTTON;
                }

                let name = if device_path.starts_with(r"\\?\HID#")
                    && let Some(name) = hid_description(device_path)
                {
                    name
                } else {
                    Txt::from_static("Mouse")
                };

                return InputDeviceInfo::new(name, cap);
            }
            RIM_TYPEHID => {
                // SAFETY: dwType defines the union field
                let hid = unsafe { info.Anonymous.hid };
                let usage_page = hid.usUsagePage;
                let usage = hid.usUsage;

                let name = if let Some(name) = hid_description(device_path) {
                    name
                } else {
                    formatx!("HID-0x{usage_page:X}:0x{usage:X}")
                };
                return InputDeviceInfo::new(name, hid_capabilities(usage_page, usage));
            }
            _ => {}
        };
    }
    InputDeviceInfo::new("Unknown Device", InputDeviceCapability::empty())
}

/// device_path is a "\\?\HID#.." path.
fn hid_description(device_path: &str) -> Option<Txt> {
    // https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsw
    // SAFETY: Function is called according to the docs, the list is destroyed later with no early returns in between.
    let h_dev_info = match unsafe { SetupDiGetClassDevsW(Some(&GUID_DEVINTERFACE_HID), None, None, DIGCF_DEVICEINTERFACE | DIGCF_PRESENT) }
    {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("`SetupDiGetClassDevsW` error, {e}");
            return None;
        }
    };

    let mut index = 0;
    let mut dev_data = SP_DEVICE_INTERFACE_DATA {
        cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
        ..Default::default()
    };

    let mut out = None::<Txt>;

    // https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf--setupdienumdeviceinterfaces
    // SAFETY: Function is called according to the documentation.
    while unsafe { SetupDiEnumDeviceInterfaces(h_dev_info, None, &GUID_DEVINTERFACE_HID, index, &mut dev_data) }.is_ok() {
        let mut required_size = 0;
        // https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetdeviceinterfacedetailw
        // SAFETY: Function is called according to the documentation.
        unsafe { SetupDiGetDeviceInterfaceDetailW(h_dev_info, &dev_data, None, 0, Some(&mut required_size), None) }.ok();

        if required_size == 0 {
            index += 1;
            continue;
        }

        let mut detail_data_buffer = vec![0u8; required_size as usize];
        let detail_data = detail_data_buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;

        // SAFETY: The pointer is valid.
        unsafe {
            (*detail_data).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }

        let mut dev_info_data = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };

        // SAFETY: Function is called according to the documentation.
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                h_dev_info,
                &dev_data,
                Some(detail_data),
                required_size,
                None,
                Some(&mut dev_info_data),
            )
        }
        .is_err()
        {
            index += 1;
            continue;
        }

        // SAFETY: The `DevicePath` points to the start of a null terminated UTF-16 string
        let current_device_path_wide = unsafe {
            let path_ptr = std::ptr::addr_of!((*detail_data).DevicePath) as *const u16;
            let mut len = 0;
            while *path_ptr.add(len) != 0 {
                len += 1;
            }
            std::slice::from_raw_parts(path_ptr, len)
        };
        let current_device_path = String::from_utf16_lossy(current_device_path_wide);

        if current_device_path.eq_ignore_ascii_case(device_path) {
            // found it!

            let mut prop_required_size = 0;
            // https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetdeviceregistrypropertyw
            // SAFETY: Function is called according to the documentation.
            unsafe {
                SetupDiGetDeviceRegistryPropertyW(
                    h_dev_info,
                    &dev_info_data,
                    SPDRP_DEVICEDESC,
                    None,
                    None,
                    Some(&mut prop_required_size),
                )
            }
            .ok();

            if prop_required_size > 0 {
                let mut prop_buffer = vec![0u8; prop_required_size as usize];

                // SAFETY: Function is called according to the documentation.
                if unsafe {
                    SetupDiGetDeviceRegistryPropertyW(h_dev_info, &dev_info_data, SPDRP_DEVICEDESC, None, Some(&mut prop_buffer), None)
                }
                .is_ok()
                {
                    // SAFETY: The buffer is valid and contains UTF-16 data.
                    let desc_wide =
                        unsafe { std::slice::from_raw_parts(prop_buffer.as_ptr() as *const u16, (prop_required_size / 2) as usize) };
                    let description = String::from_utf16_lossy(desc_wide).trim_end_matches('\0').to_string();
                    out = Some(description.into());
                    break;
                }
            } else {
                // No description available
                break;
            }
        }

        index += 1;
    }

    // https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdidestroydeviceinfolist
    // SAFETY: Function is called according to the docs
    unsafe { SetupDiDestroyDeviceInfoList(h_dev_info) }.ok();
    out
}

fn hid_capabilities(usage_page: u16, usage: u16) -> InputDeviceCapability {
    let mut out = InputDeviceCapability::empty();

    match (usage_page, usage) {
        (0x01, 0x02) => {
            // Mouse
            out |= InputDeviceCapability::BUTTON;
            out |= InputDeviceCapability::AXIS_MOTION;
            out |= InputDeviceCapability::SCROLL_MOTION;
            out |= InputDeviceCapability::POINTER_MOTION;
        }
        (0x01, 0x06) => {
            // Keyboard
            out = InputDeviceCapability::KEY;
        }
        (0x01, 0x04) => {
            // Joystick
            out |= InputDeviceCapability::BUTTON;
            out |= InputDeviceCapability::AXIS_MOTION;
        }
        (0x01, 0x05) => {
            // Game Pad
            out |= InputDeviceCapability::BUTTON;
            out |= InputDeviceCapability::AXIS_MOTION;
        }
        (0x0C, 0x01) => {
            // Consumer Control (e.g. media keys)
            out = InputDeviceCapability::KEY;
        }
        (0x0D, _) => {
            // Digitizer Page
            out = InputDeviceCapability::POINTER_MOTION;
        }
        (0x0F, _) => {
            // PID Page (force feedback)
            out = InputDeviceCapability::BUTTON;
        }
        (0x01, 0x80) => {
            // System Control (power, sleep, etc.)
            out = InputDeviceCapability::KEY;
        }
        _ => {}
    }

    out
}
