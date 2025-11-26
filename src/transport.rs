use anyhow::Result;

pub trait Transport: Send {
    fn send_bytes(&mut self, data: &[u8]) -> Result<()>;
    fn receive_bytes(&mut self, buffer: &mut [u8]) -> Result<usize>;
    fn clear_buffer(&mut self) -> Result<()>;
}
