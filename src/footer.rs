use super::{ext::ReadExt, Version};
use byteorder::{ReadBytesExt, LE};

#[derive(Debug)]
pub struct Footer {
    pub encrypted: bool,
    pub index_offset: u64,
    pub index_size: u64,
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
        // no point to read hash, frozen index or compression names
        Ok(Self {
            encrypted,
            index_offset,
            index_size,
        })
    }
}
