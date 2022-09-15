#[repr(u8)]
#[derive(PartialEq, Debug, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum ErrorCode {
    Unidentified = 0,
    Vls = 1,
    Control = 2,
    Proxy = 3,
}

impl From<u8> for ErrorCode {
    fn from(item: u8) -> Self {
        match item {
            0 => ErrorCode::Unidentified,
            1 => ErrorCode::Vls,
            _ => ErrorCode::Unidentified,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    pub fn new(code: u8, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.to_string(),
        }
    }
    pub fn from_slice(slice: &[u8]) -> Self {
        let mut v = slice.to_vec();
        let code = v.pop().unwrap_or_default();
        Self {
            code: code.into(),
            message: String::from_utf8_lossy(&v[..]).to_string(),
        }
    }
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = self.message.as_bytes().to_vec();
        v.extend_from_slice(&[self.code.clone() as u8]);
        v
    }
}

#[cfg(test)]
mod tests {
    use crate::error::*;

    #[test]
    fn test_error() {
        let e = Error::new(1, "bad msg");
        let v = e.to_vec();
        let e2 = Error::from_slice(&v);
        assert_eq!(e.code, e2.code);
        assert_eq!(e.message, e2.message);
        // println!("=> e2.message: {}", e2.message);
    }
}
