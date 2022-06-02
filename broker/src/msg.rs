use vls_protocol::serde_bolt::{self, Read, Write};

pub struct MsgDriver(Vec<u8>);

impl MsgDriver {
    pub fn new(raw: Vec<u8>) -> Self {
        Self(raw)
    }
    pub fn new_empty() -> Self {
        Self(Vec::new())
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.0.clone()
    }
}

impl Read for MsgDriver {
    type Error = serde_bolt::Error;

    // input: buf to be written. Should already be the right size
    fn read(&mut self, mut buf: &mut [u8]) -> serde_bolt::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = self.0.len();
        Ok(len)
    }

    fn peek(&mut self) -> serde_bolt::Result<Option<u8>> {
        Ok(Some(0))
    }
}

impl Write for MsgDriver {
    type Error = serde_bolt::Error;

    fn write_all(&mut self, buf: &[u8]) -> serde_bolt::Result<()> {
        self.0.extend(buf.iter().cloned());
        Ok(())
    }
}
