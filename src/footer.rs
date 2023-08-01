use std::str::FromStr;

use super::{ext::ReadExt, Compression, Version};
use byteorder::{ReadBytesExt, LE};

#[derive(Debug)]
pub struct Footer {
    pub encrypted: bool,
    pub index_offset: u64,
    pub index_size: u64,
    pub compression: Vec<Compression>,
}

impl Footer {
    pub fn new(reader: &mut std::fs::File, version: Version) -> Result<Self, super::Error> {
        // encryption key uuid
        if version >= Version::EncryptionKeyUuid {
            reader.read_u128::<LE>()?;
        }
        let encrypted = version >= Version::IndexEncryption && reader.read_bool()?;
        let magic = reader.read_u32::<LE>()?;
        if magic != super::MAGIC {
            return Err(super::Error::Magic(magic));
        }
        // version won't always be the given
        let pak_ver = reader.read_u32::<LE>()?;
        if version.as_u32() != pak_ver {
            return Err(super::Error::Version(pak_ver));
        }
        let index_offset = reader.read_u64::<LE>()?;
        let index_size = reader.read_u64::<LE>()?;
        // hash
        reader.read_guid()?;
        // frozen index
        if version == Version::FrozenIndex {
            reader.read_bool()?;
        }
        let mut compression = Vec::with_capacity(match version {
            ver if ver < Version::FNameBasedCompression => 0,
            ver if ver == Version::FNameBasedCompression => 4,
            _ => 5,
        });
        for _ in 0..compression.capacity() {
            compression.push(
                Compression::from_str(
                    &reader
                        .read_len(32)?
                        .iter()
                        // filter out whitespace and convert to char
                        .filter_map(|&ch| (ch != 0).then_some(ch as char))
                        .collect::<String>(),
                )
                .unwrap_or_default(),
            )
        }
        if version < Version::FNameBasedCompression {
            compression.push(Compression::Zlib);
            compression.push(Compression::Gzip);
            compression.push(Compression::Oodle);
        }
        compression.dedup();
        Ok(Self {
            encrypted,
            index_offset,
            index_size,
            compression,
        })
    }
}
