use anyhow::Result;
use clap::Parser;
use futures::stream::StreamExt;
use std::io::{self, Write};
use tracing::{info, error};

mod protocol;
mod crc;
mod gamepad;
mod serial;
#[cfg(target_os = "windows")]
mod device_monitor;

use crate::protocol::ControllerState;
use crate::gamepad::GamepadHandler;
#[cfg(target_os = "windows")]
use crate::device_monitor::wait_for_device_change;

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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let mut gamepad_handler = if args.daemon {
        info!("Initializing virtual gamepad...");
        match GamepadHandler::new() {
            Ok(h) => Some(h),
            Err(e) => {
                error!("Failed to initialize virtual gamepad: {}", e);
                None
            }
        }
    } else {
        None
    };

    info!("Starting VEX Controller (Serial)...");
    let s = serial::create_serial_stream(args.port.clone())?;
    let mut stream = Box::pin(s);

    info!("Listening for controller data...");

    while let Some(state) = stream.next().await {
        print_controller_state(&state);
        if let Some(handler) = &mut gamepad_handler {
            if let Err(e) = handler.update(&state) {
                error!("Error updating gamepad: {}", e);
            }
        }
    }

    Ok(())
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
