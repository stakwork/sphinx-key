pub mod control;
pub mod error;
pub mod topics;

use serde::ser;
use std::cmp::min;
use std::io;
use vls_protocol::msgs::{self, DeBolt, Message};
use vls_protocol::serde_bolt::{Error, Read, Result, Write};

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
    type Error = Error;

    // input: buf to be written. Should already be the right size
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let (mut content, remaining) = self.0.split_at(min(buf.len(), self.0.len()));
        let bytes = &mut content;
        match io::copy(bytes, &mut buf) {
            Ok(len) => {
                self.0 = remaining.to_vec();
                Ok(len as usize)
            }
            Err(_) => Ok(0),
        }
    }

    fn peek(&mut self) -> Result<Option<u8>> {
        Ok(if let Some(u) = self.0.get(0) {
            Some(u.clone())
        } else {
            None
        })
    }
}

impl Write for MsgDriver {
    type Error = Error;

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.0.extend(buf.iter().cloned());
        Ok(())
    }
}

pub fn raw_request_from_bytes(
    message: Vec<u8>,
    sequence: u16,
    dbid: u64,
) -> vls_protocol::Result<Vec<u8>> {
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_request_header(&mut md, sequence, dbid)?;
    msgs::write_vec(&mut md, message)?;
    Ok(md.bytes())
}

pub fn request_from_msg<T: ser::Serialize + DeBolt>(
    msg: T,
    sequence: u16,
    dbid: u64,
) -> vls_protocol::Result<Vec<u8>> {
    let mut md = MsgDriver::new_empty();
    msgs::write_serial_request_header(&mut md, sequence, dbid)?;
    msgs::write(&mut md, msg)?;
    Ok(md.bytes())
}

pub fn raw_response_from_msg<T: ser::Serialize + DeBolt>(
    msg: T,
    sequence: u16,
) -> vls_protocol::Result<Vec<u8>> {
    let mut m = MsgDriver::new_empty();
    msgs::write_serial_response_header(&mut m, sequence)?;
    msgs::write(&mut m, msg)?;
    Ok(m.bytes())
}

pub fn request_from_bytes<T: DeBolt>(msg: Vec<u8>) -> vls_protocol::Result<(T, u16, u64)> {
    let mut m = MsgDriver::new(msg);
    let (sequence, dbid) = msgs::read_serial_request_header(&mut m)?;
    let reply: T = msgs::read_message(&mut m)?;
    Ok((reply, sequence, dbid))
}

pub fn raw_response_from_bytes(
    res: Vec<u8>,
    expected_sequence: u16,
) -> vls_protocol::Result<Vec<u8>> {
    let mut m = MsgDriver::new(res);
    msgs::read_serial_response_header(&mut m, expected_sequence)?;
    Ok(msgs::read_raw(&mut m)?)
}

pub fn response_from_bytes(res: Vec<u8>, expected_sequence: u16) -> vls_protocol::Result<Message> {
    let mut m = MsgDriver::new(res);
    msgs::read_serial_response_header(&mut m, expected_sequence)?;
    Ok(msgs::read(&mut m)?)
}

#[cfg(test)]
mod tests {
    use crate::MsgDriver;
    use vls_protocol::msgs;
    use vls_protocol::serde_bolt::WireString;

    #[test]
    fn test_parser() {
        let msg = "hello";
        let ping = msgs::Ping {
            id: 0,
            message: WireString(msg.as_bytes().to_vec()),
        };
        let mut md = MsgDriver::new_empty();
        msgs::write_serial_request_header(&mut md, 0, 0)
            .expect("failed to write_serial_request_header");
        msgs::write(&mut md, ping).expect("failed to serial write");
        let mut m = MsgDriver::new(md.bytes());
        let (_sequence, _dbid) =
            msgs::read_serial_request_header(&mut m).expect("read ping header");
        let parsed_ping: msgs::Ping =
            msgs::read_message(&mut m).expect("failed to read ping message");
        assert_eq!(parsed_ping.id, 0);
        assert_eq!(
            String::from_utf8(parsed_ping.message.0).unwrap(),
            msg.to_string()
        );
    }
}
