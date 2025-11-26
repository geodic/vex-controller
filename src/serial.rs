use crate::transport::Transport;
use anyhow::{Context, Result};
use serialport::SerialPort;
use std::io::{self, Write};
use std::time::Duration;
use tracing::info;

pub struct SerialTransport {
    port: Box<dyn SerialPort>,
}

impl SerialTransport {
    pub fn new(port_name: Option<String>) -> Result<Self> {
        let name = find_port(port_name)?;
        let mut port = serialport::new(&name, 115200)
            .timeout(Duration::from_millis(100)) // Short timeout for non-blocking feel
            .open()
            .context("Failed to open serial port")?;

        port.write_data_terminal_ready(true)?;
        port.write_request_to_send(true)?;

        info!("Connected to {}", name);

        Ok(Self { port })
    }
}

impl Transport for SerialTransport {
    fn send_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.port.write_all(data)?;
        Ok(())
    }

    fn receive_bytes(&mut self, buffer: &mut [u8]) -> Result<usize> {
        match self.port.read(buffer) {
            Ok(n) => Ok(n),
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    fn clear_buffer(&mut self) -> Result<()> {
        self.port.clear(serialport::ClearBuffer::All)?;
        Ok(())
    }
}

fn find_port(port_name: Option<String>) -> Result<String> {
    if let Some(port) = port_name {
        Ok(port)
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
            Ok(port.port_name.clone())
        } else {
            anyhow::bail!("No VEX IQ Gen 2 Controller found");
        }
    }
}
