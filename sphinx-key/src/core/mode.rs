use std::fmt;

pub enum Mode {
    Test,
    Signer,
}

impl fmt::Debug for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Test => write!(f, "Test"),
            Mode::Signer => write!(f, "Signer"),
        }
    }
}

impl Mode {
    pub fn from_env(mode: Option<&'static str>) -> Self {
        match mode {
            Some(m) => {
                match m {
                    "TEST" => Self::Test,
                    "test" => Self::Test,
                    "SIGNER" => Self::Signer,
                    "signer" => Self::Signer,
                    _ => Self::Signer,
                }
            },
            None => Self::Signer,
        }
    }
}