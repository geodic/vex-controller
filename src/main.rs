use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::time::Duration;
use tracing::{info, error};

mod protocol;
mod gamepad;
mod serial;
mod transport;
#[cfg(target_os = "windows")]
mod device_monitor;

use crate::protocol::{ControllerState, VexController};
use crate::gamepad::GamepadHandler;
#[cfg(target_os = "windows")]
use crate::device_monitor::wait_for_device_change;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Serial port to use (optional, will auto-detect if not provided)
    #[arg(short, long)]
    port: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Calibrate the controller
    Calibrate {
        /// Abort calibration
        #[arg(long)]
        abort: bool,
    },
    /// Get controller info
    Info,
    /// Get current status (battery, joystick values)
    Status {
        /// Monitor status continuously
        #[arg(long)]
        monitor: bool,
    },
    /// Start the virtual gamepad daemon
    Daemon,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // Helper to get controller
    let get_controller = || -> Result<VexController> {
        let transport = serial::SerialTransport::new(args.port.clone())?;
        Ok(VexController::new(Box::new(transport)))
    };

    if let Some(cmd) = args.command {
        let mut controller = get_controller()?;
        match cmd {
            Commands::Calibrate { abort } => {
                if abort {
                    info!("Sending abort calibration command...");
                    match controller.abort_calibration() {
                        Ok(_) => {
                            println!("Calibration aborted.");
                        }
                        Err(e) => error!("Failed to send command: {}", e),
                    }
                } else {
                    info!("Starting calibration...");
                    if let Err(e) = run_calibration(&mut controller) {
                        error!("Calibration failed: {}", e);
                    }
                }
            }
            Commands::Info => {
                info!("Getting controller info...");
                match controller.get_versions() {
                    Ok(v) => println!("Version String: {}", v),
                    Err(e) => error!("Failed to get versions: {}", e),
                }
                
                match controller.get_pair_id() {
                    Ok(id) => println!("Pair ID: {}", id),
                    Err(e) => error!("Failed to get pair ID: {}", e),
                }
            }
            Commands::Status { monitor } => {
                if monitor {
                    info!("Monitoring controller status...");
                    loop {
                        match controller.get_state() {
                            Ok(state) => {
                                print_controller_state(&state);
                            }
                            Err(_) => {}
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                } else {
                    match controller.get_state() {
                        Ok(state) => {
                            print_controller_state(&state);
                            println!(); // Add newline since print_controller_state uses \r
                        }
                        Err(e) => error!("Failed to get status: {}", e),
                    }
                }
            }
            Commands::Daemon => {
                info!("Initializing virtual gamepad...");
                let mut gamepad_handler = match GamepadHandler::new() {
                    Ok(h) => Some(h),
                    Err(e) => {
                        error!("Failed to initialize virtual gamepad: {}", e);
                        None
                    }
                };

                info!("Starting VEX Controller (Serial)...");
                // Re-get controller here or reuse? Reuse is fine but we need to move it or clone.
                // Since we are in a match arm, we own controller.
                
                info!("Listening for controller data...");

                loop {
                    match controller.get_state() {
                        Ok(state) => {
                            print_controller_state(&state);
                            if let Some(handler) = &mut gamepad_handler {
                                if let Err(e) = handler.update(&state) {
                                    error!("Error updating gamepad: {}", e);
                                }
                            }
                        }
                        Err(_) => {}
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
        return Ok(());
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

fn run_calibration(controller: &mut VexController) -> Result<()> {
    // Send StartJsCal
    match controller.start_calibration() {
        Ok(_) => info!("Calibration command sent"),
        Err(e) => tracing::warn!("Calibration command warning: {}", e),
    }
    
    println!("Calibration started.");
    println!("Please rotate BOTH joysticks 360 degrees.");

    let mut last_cal_active = false;
    let mut waiting_for_confirm = false;

    loop {
        let state = controller.get_state()?;
        
        // Refresh status line
        if !waiting_for_confirm {
            print!("\rLeft: [{}] Right: [{}]   ", 
                if state.cal_left { "DONE" } else { "    " },
                if state.cal_right { "DONE" } else { "    " }
            );
            io::stdout().flush()?;
        }

        if state.cal_left && state.cal_right {
            if !waiting_for_confirm {
                println!("\nBoth joysticks calibrated. Press 'E Up' button to confirm.");
                waiting_for_confirm = true;
            }
        }

        // Check if calibration finished (active goes false)
        if last_cal_active && !state.cal_active {
             println!("\nCalibration complete!");
             return Ok(());
        }
        
        last_cal_active = state.cal_active;

        std::thread::sleep(Duration::from_millis(20));
    }
}
