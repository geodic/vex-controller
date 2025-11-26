use anyhow::{Result, Context};
use futures::stream::Stream;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task;
use tracing::{info, error, warn};
use std::io::{self, Write};

use crate::protocol::{Protocol, ControllerState};

pub fn create_serial_stream(port_name: Option<String>) -> Result<impl Stream<Item = ControllerState>> {
    let (tx, rx) = mpsc::channel(32);

    // Spawn a blocking task to handle the serial port
    task::spawn_blocking(move || {
        if let Err(e) = run_serial_thread(port_name, tx) {
            error!("Serial thread failed: {}", e);
        }
    });

    // Convert receiver to stream
    Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
}

fn run_serial_thread(port_name: Option<String>, tx: mpsc::Sender<ControllerState>) -> Result<()> {
    let port_name = if let Some(port) = port_name {
        port
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
            info!("Found VEX IQ Gen 2 Controller at {}", port.port_name);
            port.port_name.clone()
        } else {
            anyhow::bail!("No VEX IQ Gen 2 Controller found");
        }
    };

    let mut port = serialport::new(&port_name, 115200)
        .timeout(Duration::from_millis(100))
        .open()
        .context("Failed to open serial port")?;

    let protocol = Protocol::new();
    let mut buffer = vec![0u8; 1024];
    let mut packet_buffer = Vec::new();

    info!("Connected to {}", port_name);

    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(true)?;

    loop {
        // Send CNTR_GET_STATE command
        let command = protocol.encode_command(0x58, 0x60, &[]);
        if let Err(e) = port.write_all(&command) {
             warn!("Failed to write to serial port: {}", e);
             // Maybe break or retry?
             std::thread::sleep(Duration::from_secs(1));
             continue;
        }

        match port.read(&mut buffer) {
            Ok(n) if n > 0 => {
                packet_buffer.extend_from_slice(&buffer[..n]);
                
                while packet_buffer.len() >= 6 {
                    if let Some(start) = packet_buffer.windows(2).position(|w| w == [0xAA, 0x55]) {
                        if start > 0 {
                            packet_buffer.drain(0..start);
                        }
                        
                        if packet_buffer.len() < 5 {
                            break; 
                        }

                        let (len, header_size) = if (packet_buffer[3] & 0x80) != 0 {
                            let len = ((packet_buffer[3] & 0x7F) as usize) << 8 | (packet_buffer[4] as usize);
                            (len, 5)
                        } else {
                            (packet_buffer[3] as usize, 4)
                        };

                        let packet_len = header_size + len;
                        
                        if packet_buffer.len() >= packet_len {
                            let packet = &packet_buffer[..packet_len];
                            if let Some(payload) = protocol.decode_response(packet) {
                                if let Some(state) = Protocol::parse_controller_state(&payload) {
                                    if let Err(_) = tx.blocking_send(state) {
                                        // Receiver dropped, exit loop
                                        return Ok(());
                                    }
                                }
                                packet_buffer.drain(0..packet_len);
                            } else {
                                packet_buffer.drain(0..2);
                            }
                        } else {
                            break; 
                        }
                    } else {
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
