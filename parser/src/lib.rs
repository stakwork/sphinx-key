use vls_protocol::serde_bolt::{self, Read, Write};
use std::io;
use std::cmp::min;

pub struct MsgDriver(Vec<u8>);

impl MsgDriver {
    pub fn new(raw: Vec<u8>) -> Self {
        Self(raw)
    }
    pub fn new_empty() -> Self {
        Self(Vec::new())
    }
    pub fn as_ref(&self) -> &Vec<u8> {
        &self.0
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
        let (mut content, remaining) = self.0.split_at(
            min(buf.len(), self.0.len())
        );
        let bytes = &mut content;
        match io::copy(bytes, &mut buf) {
            Ok(len) => {
                self.0 = remaining.to_vec();
                Ok(len as usize)
            },
            Err(_) => Ok(0)
        }
    }

    fn peek(&mut self) -> serde_bolt::Result<Option<u8>> {
        Ok(if let Some(u) = self.0.get(0) { Some(u.clone()) } else { None})
    }
}

impl Write for MsgDriver {
    type Error = serde_bolt::Error;

    fn write_all(&mut self, buf: &[u8]) -> serde_bolt::Result<()> {
        self.0.extend(buf.iter().cloned());
        Ok(())
    }
}
