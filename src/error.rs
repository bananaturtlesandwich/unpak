/// the error type used by unpak
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // external crate errors
    /// key hash is an incorrect length
    #[error("key hash is an incorrect length")]
    Aes,

    // standard library errors
    /// std::io error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// error converting from utf8
    #[error("utf8 conversion: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// error converting from utf16
    #[error("utf16 conversion: {0}")]
    Utf16(#[from] std::string::FromUtf16Error),
    /// error dereferencing bufwriter
    #[error("bufwriter dereference: {0}")]
    IntoInner(#[from] std::io::IntoInnerError<std::io::BufWriter<Vec<u8>>>),

    // crate feature errors
    /// re-enable the encryption feature to read encrypted paks
    #[error("re-enable the encryption feature to read encrypted paks")]
    Encryption,
    /// re-enable the compression feature to read compressed paks
    #[error("re-enable the compression feature to read compressed paks")]
    Compression,

    // internal crate errors
    /// failed to convert to boolean - normally a result of parsing with wrong version
    #[error("found {0} instead of a boolean")]
    Bool(u8),
    /// read bad magic - normally a result of parsing with wrong version
    #[error("found magic of {0:#x} instead of {:#x}", super::MAGIC)]
    Magic(u32),
    /// pak is encrypted but no valid key was provided
    #[error("pak is encrypted but no valid key was provided")]
    Encrypted,
    /// pak could not be parsed with any version - make a github issue
    #[error("pak could not be parsed with any version")]
    Parse,
    /// no entry found at the specified path
    #[error("no entry could be found at {0}")]
    Missing(String),
    /// parsing with wrong version - convert error to string to get correct version
    #[error("wrong version - try using v{0}")]
    Version(u32),
    /// oodle compression is not currently supported
    #[error("oodle compression is not currently supported")]
    Oodle,
    /// any other error if you're too lazy to have a function return custom error
    #[error("{0}")]
    Other(String),
}
