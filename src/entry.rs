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
    uncompressed: u64,
    compression: Option<usize>,
    blocks: Option<Vec<Block>>,
    encrypted: bool,
    block_uncompressed: u64,
}

impl Entry {
    pub fn new<R: io::Read + io::Seek>(
        reader: &mut R,
        version: super::Version,
    ) -> Result<Self, super::Error> {
        // since i need the compression flags, i have to store these as variables which is mildly annoying
        let offset = reader.read_u64::<LE>()?;
        let compressed = reader.read_u64::<LE>()?;
        let uncompressed = reader.read_u64::<LE>()?;
        let compression = match match version == Version::FNameBasedCompression {
            true => reader.read_u8()? as u32,
            false => reader.read_u32::<LE>()?,
        } {
            0 => None,
            i => Some(i as usize - 1),
        };
        // timestamp
        if version == Version::Initial {
            reader.read_u64::<LE>()?;
        }
        // hash
        reader.read_guid()?;
        let blocks = match version >= Version::CompressionEncryption && compression.is_some() {
            true => Some(reader.read_array(Block::new)?),
            false => None,
        };
        let encrypted = version >= Version::CompressionEncryption && reader.read_bool()?;
        let block_uncompressed = match version >= Version::CompressionEncryption {
            true => reader.read_u32::<LE>()? as u64,
            false => uncompressed,
        };
        Ok(Self {
            offset,
            compressed,
            uncompressed,
            compression,
            blocks,
            encrypted,
            block_uncompressed,
        })
    }

    pub fn from_encoded<R: io::Read>(reader: &mut R) -> Result<Self, super::Error> {
        let bitfield = reader.read_u32::<LE>()?;
        let compression = match (bitfield >> 23) & 0x3F {
            0 => None,
            i => Some(i as usize - 1),
        };
        let encrypted = (bitfield & (1 << 22)) != 0;
        let block_uncompressed = match (bitfield & 0x3F) == 0x3F {
            true => reader.read_u32::<LE>()?,
            false => (bitfield & 0x3F) << 11,
        } as u64;
        let mut flag = |bit: u32| -> Result<u64, super::Error> {
            Ok(match bitfield & (1 << bit) != 0 {
                true => reader.read_u32::<LE>()? as u64,
                false => reader.read_u64::<LE>()?,
            })
        };
        let offset = flag(31)?;
        let uncompressed = flag(30)?;
        let compressed = match compression.is_some() {
            true => flag(29)?,
            false => uncompressed,
        };
        let block_count: u32 = (bitfield >> 6) & 0xffff;
        // all versions with an encoded record have a header size of 53
        let mut start = 53;
        if compression.is_some() {
            start += 4 + 16 * block_count as u64
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
            uncompressed,
            compression,
            blocks,
            encrypted,
            block_uncompressed,
        })
    }

    pub fn read<W: io::Write>(
        &self,
        path: impl AsRef<std::path::Path>,
        version: super::Version,
        compression: &[super::Compression],
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
        let blocks = match &self.blocks {
            Some(blocks) => blocks
                .iter()
                .map(|block| match version >= Version::RelativeChunkOffsets {
                    true => {
                        (block.start - (data_offset - self.offset)) as usize
                            ..(block.end - (data_offset - self.offset)) as usize
                    }
                    false => {
                        (block.start - data_offset) as usize..(block.end - data_offset) as usize
                    }
                })
                .collect(),
            None => vec![0..data.len()],
        };
        match self.compression.and_then(|i| compression.get(i)) {
            None | Some(Compression::None) => {
                for block in blocks {
                    buf.write_all(&data[block])?
                }
            }
            #[cfg(feature = "compression")]
            Some(Compression::Zlib) => {
                for block in blocks {
                    io::copy(&mut flate2::read::ZlibDecoder::new(&data[block]), buf)?;
                }
            }
            #[cfg(feature = "compression")]
            Some(Compression::Gzip) => {
                for block in blocks {
                    io::copy(&mut flate2::read::GzDecoder::new(&data[block]), buf)?;
                }
            }
            #[cfg(feature = "oodle")]
            Some(Compression::Oodle) => {
                let block_count = blocks.len();
                for (i, block) in blocks.into_iter().enumerate() {
                    let data = &data[block];
                    let mut scratch = vec![
                        0;
                        match block_count == 1 {
                            true => self.uncompressed,
                            false => self
                                .block_uncompressed
                                .min(self.uncompressed - i as u64 * self.block_uncompressed),
                        } as usize
                    ];
                    if unsafe {
                        OodleLZ_Decompress(
                            data.as_ptr(),
                            data.len(),
                            scratch.as_mut_ptr(),
                            scratch.len(),
                            1,
                            1,
                            0,
                            0,
                            0,
                            0,
                            0,
                            std::ptr::null_mut(),
                            0,
                            3,
                        ) == 0
                    } {
                        return Err(super::Error::OodleLZ_Decompress);
                    }
                    buf.write_all(scratch.as_slice())?;
                }
            }
            #[cfg(all(feature = "compression", not(feature = "oodle")))]
            Some(Compression::Oodle) => return Err(crate::Error::Oodle),
            #[allow(unreachable_patterns)]
            _ => return Err(super::Error::Compression),
        }
        buf.flush()?;
        Ok(())
    }
}

#[cfg(feature = "oodle")]
#[cfg_attr(target_os = "windows", link(name = "oo2core_win64", kind = "static"))]
#[cfg_attr(target_os = "macos", link(name = "liboo2coremac64", kind = "static"))]
#[cfg_attr(
    all(target_os = "linux", target_arch = "x86_64"),
    link(name = "liboo2corelinux64", kind = "static")
)]
#[cfg_attr(
    all(target_os = "linux", target_arch = "arm"),
    link(name = "liboo2corelinuxarm64", kind = "static")
)]
extern "C" {
    fn OodleLZ_Decompress(
        compBuf: *const u8,
        compBufSize: usize,
        rawBuf: *mut u8,
        rawLen: usize,
        fuzzSafe: u32,
        checkCRC: u32,
        verbosity: u32,
        decBufBase: u64,
        decBufSize: usize,
        fpCallback: u64,
        callbackUserData: u64,
        decoderMemory: *mut u8,
        decoderMemorySize: usize,
        threadPhase: u32,
    ) -> i32;
}
