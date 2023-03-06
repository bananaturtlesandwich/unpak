/// the error type used by unpak
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // dependency errors
    #[error("enum conversion: {0}")]
    Strum(#[from] strum::ParseError),
    #[error("key hash is an incorrect length")]
    Aes,
    // std errors
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 conversion: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("utf16 conversion: {0}")]
    Utf16(#[from] std::string::FromUtf16Error),
    #[error("bufwriter dereference: {0}")]
    IntoInner(#[from] std::io::IntoInnerError<std::io::BufWriter<Vec<u8>>>),
    // feature errors
    #[error("re-enable the encryption feature to read encrypted paks")]
    Encryption,
    // crate errors
    #[error("found {0} instead of a boolean")]
    Bool(u8),
    #[error("found magic of {0:#x} instead of {:#x}", super::MAGIC)]
    Magic(u32),
    #[error("pak is encrypted but no key was provided")]
    Encrypted,
    #[error("pak could not be parsed with any version")]
    Parse,
    #[error("no entry could be found at {0}")]
    Missing(String),
    #[error("{0}")]
    Other(String),
}

impl From<&mut std::io::Error> for Error {
    fn from(value: &mut std::io::Error) -> Self {
        value.into()
    }
}
