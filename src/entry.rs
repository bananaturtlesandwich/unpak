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
    pub fn new<R: io::Read + io::Seek>(
        reader: &mut R,
        version: super::Version,
    ) -> Result<Self, super::Error> {
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
        let compression = match (bitfield >> 23) & 0x3F {
            0x01 | 0x10 | 0x20 => Compression::Zlib,
            _ => Compression::None,
        };
        let encrypted = (bitfield & (1 << 22)) != 0;
        // uncompressed
        if (bitfield & 0x3F) == 0x3F {
            reader.read_u32::<LE>()?;
        }
        let mut flag = |bit: u32| -> Result<u64, super::Error> {
            Ok(match bitfield & (1 << bit) != 0 {
                true => reader.read_u32::<LE>()? as u64,
                false => reader.read_u64::<LE>()?,
            })
        };
        let offset = flag(31)?;
        let uncompressed = flag(30)?;
        let compressed = match compression != Compression::None {
            true => flag(29)?,
            false => uncompressed,
        };
        let block_count: u32 = (bitfield >> 6) & 0xffff;
        // all versions with an encoded record a header size of 53
        let mut start = 53;
        if compression != Compression::None {
            start += 16 * block_count as u64 + 4
        }
        let blocks = match block_count {
            0 => None,
            1 if !encrypted => Some(vec![Block {
                start,
                end: start + compressed,
            }]),
            block_count => {
                let mut blocks = Vec::with_capacity(block_count as usize);
                for _ in 0..block_count {
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
                Some(blocks)
            }
        };
        Ok(Self {
            offset,
            compressed,
            compression,
            blocks,
            encrypted,
        })
    }

    pub fn read<W: io::Write>(
        &self,
        path: impl AsRef<std::path::Path>,
        version: super::Version,
        #[cfg(feature = "encryption")] key: Option<&aes::Aes256Dec>,
        buf: &mut W,
    ) -> Result<(), super::Error> {
        use io::Seek;
        let mut reader = std::fs::File::open(&path)?;
        reader.seek(io::SeekFrom::Start(self.offset))?;
        Entry::new(&mut reader, version)?;
        let data_offset = reader.stream_position()?;
        #[allow(unused_mut)]
        let mut data = reader.read_len(match self.encrypted {
            // add alignment (aes block size: 16) then zero out alignment bits
            true => (self.compressed + 15) & !15,
            false => self.compressed,
        } as usize)?;
        if self.encrypted {
            #[cfg(feature = "encryption")]
            {
                super::decrypt(key, &mut data)?;
                data.truncate(self.compressed as usize);
            }
            #[cfg(not(feature = "encryption"))]
            return Err(super::Error::Encryption);
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
