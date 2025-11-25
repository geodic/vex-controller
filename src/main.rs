use std::time::Duration;
use std::io::{self, Write};
use clap::Parser;

mod crc;
mod protocol;
mod gamepad;
#[cfg(target_os = "windows")]
mod device_monitor;

use protocol::{Protocol, ControllerState};
use gamepad::GamepadHandler;
#[cfg(target_os = "windows")]
use device_monitor::wait_for_device_change;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Serial port to use (optional, will auto-detect if not provided)
    #[arg(short, long)]
    port: Option<String>,

    /// Enable virtual gamepad
    #[arg(long)]
    daemon: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let mut gamepad_handler = if args.daemon {
        println!("Initializing virtual gamepad...");
        match GamepadHandler::new() {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("Failed to initialize virtual gamepad: {}", e);
                None
            }
        }
    } else {
        None
    };

    #[cfg(target_os = "windows")]
    if args.daemon {
        loop {
            // Try to run. If it fails (device not found or disconnected), we wait for an event.
            if let Err(e) = run_controller_loop(&args, &mut gamepad_handler) {
                eprintln!("Controller loop ended: {}", e);
            }
            
            println!("Waiting for device connection...");
            wait_for_device_change();
            // Give a small delay for the device to be fully ready after insertion event
            std::thread::sleep(Duration::from_millis(500));
        }
    } else {
        return run_controller_loop(&args, &mut gamepad_handler);
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Linux, we rely on udev/systemd to start us when the device is present.
        // If we fail or disconnect, we exit. Systemd will NOT restart us immediately if we configure it so,
        // but udev will trigger the service again on insertion.
        return run_controller_loop(&args, &mut gamepad_handler);
    }

    Ok(())
}

fn run_controller_loop(args: &Args, gamepad_handler: &mut Option<GamepadHandler>) -> anyhow::Result<()> {
    let port_name = if let Some(port) = &args.port {
        port.clone()
    } else {
        let ports = serialport::available_ports()?;
        let vex_port = ports.iter().find(|p| {
            if let serialport::SerialPortType::UsbPort(info) = &p.port_type {
                info.vid == 10376 && info.pid == 528
            } else {
                false
            }
        });

        if let Some(port) = vex_port {
            println!("Found VEX IQ Gen 2 Controller at {}", port.port_name);
            port.port_name.clone()
        } else {
            anyhow::bail!("No VEX IQ Gen 2 Controller found");
        }
    };

    let mut port = serialport::new(&port_name, 115200)
        .timeout(Duration::from_millis(100))
        .open()?;

    let protocol = Protocol::new();
    let mut buffer = vec![0u8; 1024];
    let mut packet_buffer = Vec::new();

    println!("Connected to {}", port_name);

    // Set DTR and RTS, just in case
    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(true)?;

    loop {
        // Send CNTR_GET_STATE command
        // Cmd1: 0x58 (88), Cmd2: 0x60 (96)
        let command = protocol.encode_command(0x58, 0x60, &[]);
        // println!("Sending: {:02X?}", command);
        if let Err(e) = port.write_all(&command) {
             return Err(e.into());
        }

        // Read response
        match port.read(&mut buffer) {
            Ok(n) if n > 0 => {
                // println!("Received {} bytes: {:02X?}", n, &buffer[..n]);
                packet_buffer.extend_from_slice(&buffer[..n]);
                
                // Try to find a valid packet in packet_buffer
                // Header is AA 55
                while packet_buffer.len() >= 6 {
                    if let Some(start) = packet_buffer.windows(2).position(|w| w == [0xAA, 0x55]) {
                        if start > 0 {
                            packet_buffer.drain(0..start);
                        }
                        
                        // Now buffer starts with AA 55
                        if packet_buffer.len() < 5 {
                            break; // Need more data for length
                        }

                        // Length parsing (at index 3)
                        let (len, header_size) = if (packet_buffer[3] & 0x80) != 0 {
                            let len = ((packet_buffer[3] & 0x7F) as usize) << 8 | (packet_buffer[4] as usize);
                            (len, 5)
                        } else {
                            (packet_buffer[3] as usize, 4)
                        };

                        // len includes Cmd + Payload + CRC
                        let packet_len = header_size + len;
                        
                        if packet_buffer.len() >= packet_len {
                            let packet = &packet_buffer[..packet_len];
                            if let Some(payload) = protocol.decode_response(packet) {
                                if let Some(state) = Protocol::parse_controller_state(&payload) {
                                    print_controller_state(&state);
                                    if let Some(handler) = gamepad_handler {
                                        if let Err(e) = handler.update(&state) {
                                            eprintln!("Error updating gamepad: {}", e);
                                        }
                                    }
                                    io::stdout().flush().unwrap();
                                }
                                packet_buffer.drain(0..packet_len);
                            } else {
                                // Invalid CRC or packet, skip header and try again
                                packet_buffer.drain(0..2);
                            }
                        } else {
                            break; // Need more data
                        }
                    } else {
                        // No header found, clear buffer
                        packet_buffer.clear();
                        break;
                    }
                }
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {}
            Err(e) => return Err(e.into()),
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

fn print_controller_state(state: &ControllerState) {
    print!("\rLX: {:3} LY: {:3} RX: {:3} RY: {:3} | L: {}{} R: {}{} E: {}{} F: {}{} | L3: {} R3: {} | Bat: {:3}%   ",
        state.left_x, state.left_y, state.right_x, state.right_y,
        if state.l_up { "^" } else { " " }, if state.l_down { "v" } else { " " },
        if state.r_up { "^" } else { " " }, if state.r_down { "v" } else { " " },
        if state.e_up { "^" } else { " " }, if state.e_down { "v" } else { " " },
        if state.f_up { "^" } else { " " }, if state.f_down { "v" } else { " " },
        if state.l3 { "X" } else { " " }, if state.r3 { "X" } else { " " },
        state.battery
    );
    io::stdout().flush().unwrap();
}
