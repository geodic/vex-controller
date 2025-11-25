#[cfg(target_os = "windows")]
mod device_monitor {
    use windows::{
        core::*,
        Win32::Foundation::*,
        Win32::System::LibraryLoader::GetModuleHandleA,
        Win32::UI::WindowsAndMessaging::*,
        Win32::UI::Input::KeyboardAndMouse::*,
        Win32::Devices::DeviceAndDriverInstallation::*,
    };
    use std::ptr;

    pub fn wait_for_device_change() {
        unsafe {
            let instance = GetModuleHandleA(None).unwrap();
            let class_name = s!("DeviceMonitorClass");

            let wnd_class = WNDCLASSA {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance,
                lpszClassName: class_name,
                ..Default::default()
            };

            RegisterClassA(&wnd_class);

            let hwnd = CreateWindowExA(
                WINDOW_EX_STYLE(0),
                class_name,
                s!("Device Monitor"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                instance,
                None,
            );

            // Register for device notifications
            let mut notification_filter = DEV_BROADCAST_DEVICEINTERFACE_A {
                dbcc_size: std::mem::size_of::<DEV_BROADCAST_DEVICEINTERFACE_A>() as u32,
                dbcc_devicetype: DBT_DEVTYP_DEVICEINTERFACE,
                // GUID_DEVINTERFACE_USB_DEVICE: {A5DCBF10-6530-11D2-901F-00C04FB951ED}
                dbcc_classguid: windows::core::GUID::from_u128(0xA5DCBF10_6530_11D2_901F_00C04FB951ED), 
                ..Default::default()
            };

            let notify_handle = RegisterDeviceNotificationA(
                HANDLE(hwnd.0),
                &mut notification_filter as *mut _ as *mut _,
                DEVICE_NOTIFY_WINDOW_HANDLE,
            );

            let mut msg = MSG::default();
            // Wait for message. GetMessage blocks.
            // We only want to wait until we get a device change, then return.
            // But GetMessage waits for ANY message.
            // We can loop until we get WM_DEVICECHANGE.
            
            loop {
                let res = GetMessageA(&mut msg, hwnd, 0, 0);
                if res.0 == 0 || res.0 == -1 {
                    break;
                }
                
                if msg.message == WM_DEVICECHANGE {
                    // Device change detected!
                    // We could check wParam to see if it is arrival (DBT_DEVICEARRIVAL = 0x8000)
                    // But for now, any change is a good reason to check for our device.
                    break;
                }

                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
            
            if !notify_handle.is_invalid() {
                UnregisterDeviceNotification(notify_handle);
            }
            DestroyWindow(hwnd);
        }
    }

    unsafe extern "system" fn wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcA(window, message, wparam, lparam)
    }
}

#[cfg(target_os = "windows")]
pub use device_monitor::wait_for_device_change;

#[cfg(not(target_os = "windows"))]
pub fn wait_for_device_change() {
    // On non-Windows, we don't have a specific event wait implementation here yet.
    // But the main loop logic will handle it differently.
    std::thread::sleep(std::time::Duration::from_secs(1));
}
