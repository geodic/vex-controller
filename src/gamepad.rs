use crate::protocol::ControllerState;

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use evdev::{
        uinput::{VirtualDevice, VirtualDeviceBuilder},
        AttributeSet, InputEvent, EventType, Key, AbsoluteAxisType, UinputAbsSetup, AbsInfo,
    };

    pub struct GamepadHandler {
        device: VirtualDevice,
    }

    impl GamepadHandler {
        pub fn new() -> anyhow::Result<Self> {
            let mut keys = AttributeSet::<Key>::new();
            keys.insert(Key::BTN_TL);
            keys.insert(Key::BTN_TL2);
            keys.insert(Key::BTN_TR);
            keys.insert(Key::BTN_TR2);
            keys.insert(Key::BTN_THUMBL);
            keys.insert(Key::BTN_THUMBR);
            
            keys.insert(Key::BTN_SOUTH);
            keys.insert(Key::BTN_EAST);
            keys.insert(Key::BTN_NORTH);
            keys.insert(Key::BTN_WEST);

            let device = VirtualDeviceBuilder::new()?
                .name("VEX IQ Gen 2 Controller")
                .with_keys(&keys)?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisType::ABS_X,
                    AbsInfo::new(127, 0, 255, 0, 0, 0),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisType::ABS_Y,
                    AbsInfo::new(127, 0, 255, 0, 0, 0),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisType::ABS_RX,
                    AbsInfo::new(127, 0, 255, 0, 0, 0),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisType::ABS_RY,
                    AbsInfo::new(127, 0, 255, 0, 0, 0),
                ))?
                .build()?;

            Ok(Self { device })
        }

        pub fn update(&mut self, state: &ControllerState) -> anyhow::Result<()> {
            let mut events = Vec::new();

            // Axes
            // VEX: 0-255, 127 center.
            // Linux ABS_X/Y: We defined 0-255.
            // Invert Y axes to match standard gamepad (Up is negative)
            // VEX: Up is 255 (usually), Down is 0.
            // Standard gamepad: Up is min, Down is max.
            // So we need to invert Y axes: 255 - value.
            // Also, user reported Left and Right joysticks are switched.
            
            events.push(InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_X.0, state.right_x as i32));
            events.push(InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_Y.0, 255 - state.right_y as i32));
            events.push(InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_RX.0, state.left_x as i32));
            events.push(InputEvent::new(EventType::ABSOLUTE, AbsoluteAxisType::ABS_RY.0, 255 - state.left_y as i32));

            // Buttons
            events.push(InputEvent::new(EventType::KEY, Key::BTN_TL.0, if state.l_up { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_TL2.0, if state.l_down { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_TR.0, if state.r_up { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_TR2.0, if state.r_down { 1 } else { 0 }));
            
            events.push(InputEvent::new(EventType::KEY, Key::BTN_THUMBL.0, if state.l3 { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_THUMBR.0, if state.r3 { 1 } else { 0 }));

            // Action Buttons (Diamond)
            // E Up -> Y (North)
            // E Down -> A (South)
            // F Up -> X (West)
            // F Down -> B (East)
            events.push(InputEvent::new(EventType::KEY, Key::BTN_NORTH.0, if state.e_up { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_SOUTH.0, if state.e_down { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_WEST.0, if state.f_up { 1 } else { 0 }));
            events.push(InputEvent::new(EventType::KEY, Key::BTN_EAST.0, if state.f_down { 1 } else { 0 }));

            self.device.emit(&events)?;
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::GamepadHandler;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use vigem_client::{Client, X360Controller, TargetId, XButtons, XGamepad};

    pub struct GamepadHandler {
        target: X360Controller,
    }

    impl GamepadHandler {
        pub fn new() -> anyhow::Result<Self> {
            let client = Client::connect().map_err(|e| anyhow::anyhow!("Failed to connect to ViGEmBus: {:?}", e))?;
            let mut target = X360Controller::new(client, TargetId::XBOX360_WIRED);
            target.plugin().map_err(|e| anyhow::anyhow!("Failed to plugin virtual controller: {:?}", e))?;
            Ok(Self { target })
        }

        pub fn update(&mut self, state: &ControllerState) -> anyhow::Result<()> {
            let mut report = XGamepad::default();

            // Map buttons
            // L Up -> LB
            // L Down -> LT (Trigger)
            // R Up -> RB
            // R Down -> RT (Trigger)
            
            if state.l_up { report.buttons.raw |= XButtons::LB.raw; }
            if state.r_up { report.buttons.raw |= XButtons::RB.raw; }
            
            if state.l_down { report.left_trigger = 255; }
            if state.r_down { report.right_trigger = 255; }

            if state.l3 { report.buttons.raw |= XButtons::LTHUMB.raw; }
            if state.r3 { report.buttons.raw |= XButtons::RTHUMB.raw; }

            // Diamond
            // E Up -> Y
            // E Down -> A
            // F Up -> X
            // F Down -> B
            if state.e_up { report.buttons.raw |= XButtons::Y.raw; }
            if state.e_down { report.buttons.raw |= XButtons::A.raw; }
            if state.f_up { report.buttons.raw |= XButtons::X.raw; }
            if state.f_down { report.buttons.raw |= XButtons::B.raw; }

            // Joysticks
            // VEX: 0-255, 127 center.
            // XInput: -32768 to 32767.
            // Formula: (val - 127) * 256 roughly.
            
            // Swapped: Left stick controls Right stick on gamepad, and vice versa.
            // Inverted Y: VEX Up is 255. XInput Up is positive.
            // So VEX 255 -> 32767. VEX 0 -> -32768.
            // (val as i16 - 127) * 256
            
            fn map_axis(val: u8) -> i16 {
                ((val as i32 - 127) * 256) as i16
            }

            report.thumb_lx = map_axis(state.right_x);
            report.thumb_ly = map_axis(state.right_y);
            report.thumb_rx = map_axis(state.left_x);
            report.thumb_ry = map_axis(state.left_y);

            self.target.update(&report).map_err(|e| anyhow::anyhow!("Failed to update controller: {:?}", e))?;
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::GamepadHandler;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub struct GamepadHandler;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl GamepadHandler {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
    pub fn update(&mut self, _state: &ControllerState) -> anyhow::Result<()> {
        Ok(())
    }
}
