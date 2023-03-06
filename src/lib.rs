#![allow(dead_code)]
mod entry;
mod error;
mod ext;
mod footer;
mod pak;

pub use {error::*, pak::*};

/// the magic used to identify a pak
pub const MAGIC: u32 = 0x5A6F12E1;

/// the possible versions that a pak file can be
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug, strum::Display, strum::EnumIter)]
pub enum Version {
    /// initial specification
    Initial,
    /// timestamps removed    
    NoTimestamps,
    /// compression and encryption support
    CompressionEncryption,
    /// index encryption support
    IndexEncryption,
    /// offsets now relative to header
    RelativeChunkOffsets,
    /// record deletion support
    DeleteRecords,
    /// include key UUID
    EncryptionKeyUuid,
    /// include compression names
    FNameBasedCompression,
    /// adds another compression name
    FNameBasedCompression2,
    /// include frozen index byte
    FrozenIndex,
    /// index format overhauled
    PathHashIndex,
    /// idk what this changed
    Fnv64BugFix,
}

impl Version {
    /// gets an iterator over the versions
    pub fn iter() -> VersionIter {
        <Version as strum::IntoEnumIterator>::iter()
    }

    fn from_repr(byte: u32) -> Self {
        match byte {
            1 => Self::Initial,
            2 => Self::NoTimestamps,
            3 => Self::CompressionEncryption,
            4 => Self::IndexEncryption,
            5 => Self::RelativeChunkOffsets,
            6 => Self::DeleteRecords,
            7 => Self::EncryptionKeyUuid,
            8 => Self::FNameBasedCompression,
            9 => Self::FrozenIndex,
            10 => Self::PathHashIndex,
            11 => Self::Fnv64BugFix,
            _ => unimplemented!(),
        }
    }

    fn footer_size(self) -> i64 {
        // (magic + version): u32 + (offset + size): u64 + hash: [u8; 20]
        let mut size = 4 + 4 + 8 + 8 + 20;
        if self >= Version::EncryptionKeyUuid {
            // encryption uuid: u128
            size += 16;
        }
        if self >= Version::IndexEncryption {
            // encrypted: bool
            size += 1;
        }
        if self == Version::FrozenIndex {
            // frozen index: bool
            size += 1;
        }
        if self >= Version::FNameBasedCompression {
            // compression names: [[u8; 32]; 4]
            size += 32 * 4;
        }
        if self >= Version::FNameBasedCompression2 {
            // extra compression name: [u8; 32]
            size += 32
        }
        size
    }
}

/// the possible compression types
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, strum::Display, strum::EnumString)]
pub enum Compression {
    #[default]
    None,
    Zlib,
    Gzip,
    Oodle,
}

fn decrypt(key: Option<&aes::Aes256Dec>, bytes: &mut [u8]) -> Result<(), Error> {
    match key {
        Some(key) => {
            use aes::cipher::BlockDecrypt;
            for chunk in bytes.chunks_mut(16) {
                key.decrypt_block(aes::Block::from_mut_slice(chunk))
            }
            Ok(())
        }
        None => Err(Error::Encrypted),
    }
}
