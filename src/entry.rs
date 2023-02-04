use super::{ext::ReadExt, Compression, Version};
use byteorder::{ReadBytesExt, LE};
use std::io;

#[derive(Debug)]
struct Block {
    start: u64,
    end: u64,
}

impl Block {
    pub fn new<R: io::Read>(reader: &mut R) -> Result<Self, super::Error> {
        Ok(Self {
            start: reader.read_u64::<LE>()?,
            end: reader.read_u64::<LE>()?,
        })
    }
}

#[derive(Debug)]
pub struct Entry {
    offset: u64,
    compressed: u64,
    compression: Compression,
    blocks: Option<Vec<Block>>,
    encrypted: bool,
}

impl Entry {
    pub fn new<R: io::Read>(reader: &mut R, version: super::Version) -> Result<Self, super::Error> {
        // since i need the compression flags, i have to store these as variables which is mildly annoying
        let offset = reader.read_u64::<LE>()?;
        let compressed = reader.read_u64::<LE>()?;
        // uncompressed
        reader.read_u64::<LE>()?;
        let compression = match reader.read_u32::<LE>()? {
            0x01 | 0x10 | 0x20 => Compression::Zlib,
            _ => Compression::None,
        };
        // timestamp
        if version == Version::Initial {
            reader.read_u64::<LE>()?;
        }
        // hash
        reader.read_guid()?;
        let blocks =
            match version >= Version::CompressionEncryption && compression != Compression::None {
                true => Some(reader.read_array(Block::new)?),
                false => None,
            };
        let encrypted = version >= Version::CompressionEncryption && reader.read_bool()?;
        // block uncompressed
        if version >= Version::CompressionEncryption {
            reader.read_u32::<LE>()?;
        }
        Ok(Self {
            offset,
            compressed,
            compression,
            blocks,
            encrypted,
        })
    }

    pub fn from_encoded<R: io::Read>(reader: &mut R) -> Result<Self, super::Error> {
        let bitfield = reader.read_u32::<LE>()?;
        let offset = match bitfield & (1 << 31) != 0 {
            true => reader.read_u32::<LE>()? as u64,
            false => reader.read_u64::<LE>()?,
        };
        let uncompressed = match bitfield & (1 << 30) != 0 {
            true => reader.read_u32::<LE>()? as u64,
            false => reader.read_u64::<LE>()?,
        };
        let compression = match (bitfield >> 23) & 0x3F {
            0x01 | 0x10 | 0x20 => Compression::Zlib,
            _ => Compression::None,
        };
        let compressed = match compression != Compression::None {
            true => match bitfield & (1 << 29) != 0 {
                true => reader.read_u32::<LE>()? as u64,
                false => reader.read_u64::<LE>()?,
            },
            false => uncompressed,
        };
        let encrypted = (bitfield & (1 << 22)) != 0;
        let mut blocks = Vec::with_capacity(((bitfield >> 6) & 0xFFFF) as usize);
        // all versions with an encoded record a header size of 53
        let mut start = 53;
        if compression != Compression::None {
            start += 16 * blocks.capacity() as u64 + 4
        }
        for _ in 0..blocks.capacity() {
            let size = reader.read_u32::<LE>()?;
            blocks.push(Block {
                start,
                end: start + size as u64,
            });
            start += match encrypted {
                true => (size + 15) & !15,
                false => size,
            } as u64;
        }
        let blocks = Some(blocks);
        Ok(Self {
            offset,
            compressed,
            compression,
            blocks,
            encrypted,
        })
    }

    pub fn read<R: io::Read + io::Seek, W: io::Write>(
        &self,
        reader: &mut R,
        version: super::Version,
        key: Option<&aes::Aes256Dec>,
        buf: &mut W,
    ) -> Result<(), super::Error> {
        reader.seek(io::SeekFrom::Start(self.offset))?;
        Entry::new(reader, version)?;
        let data_offset = reader.stream_position()?;
        let mut data = reader.read_len(match self.encrypted {
            // add alignment (aes block size: 16) then zero out alignment bits
            true => (self.compressed + 15) & !15,
            false => self.compressed,
        } as usize)?;
        if self.encrypted {
            let Some(key) = key else {
                return Err(super::Error::Encrypted);
            };
            use aes::cipher::BlockDecrypt;
            for block in data.chunks_mut(16) {
                key.decrypt_block(aes::Block::from_mut_slice(block))
            }
            data.truncate(self.compressed as usize);
        }
        macro_rules! decompress {
            ($decompressor: ty) => {
                match &self.blocks {
                    Some(blocks) => {
                        for block in blocks {
                            io::copy(
                                &mut <$decompressor>::new(
                                    &data[match version >= Version::RelativeChunkOffsets {
                                        true => {
                                            (block.start - (data_offset - self.offset)) as usize
                                                ..(block.end - (data_offset - self.offset)) as usize
                                        }
                                        false => {
                                            (block.start - data_offset) as usize
                                                ..(block.end - data_offset) as usize
                                        }
                                    }],
                                ),
                                buf,
                            )?;
                        }
                    }
                    None => {
                        io::copy(&mut <$decompressor>::new(data.as_slice()), buf)?;
                    }
                }
            };
        }
        match self.compression {
            Compression::None => buf.write_all(&data)?,
            Compression::Zlib => decompress!(flate2::read::ZlibDecoder<&[u8]>),
            Compression::Gzip => decompress!(flate2::read::GzDecoder<&[u8]>),
            Compression::Oodle => todo!(),
        }
        buf.flush()?;
        Ok(())
    }
}
