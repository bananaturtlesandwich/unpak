use super::Version;
use std::io;

#[derive(Debug)]
pub struct Pak<R: io::Read + io::Seek> {
    version: Version,
    mount_point: String,
    key: Option<aes::Aes256Dec>,
    entries: hashbrown::HashMap<String, super::entry::Entry>,
    reader: R,
}

impl<R: io::Read + io::Seek> Pak<R> {
    pub fn new(
        mut reader: R,
        version: super::Version,
        key_hash: Option<&[u8]>,
    ) -> Result<Self, super::Error> {
        use super::ext::ReadExt;
        use byteorder::{ReadBytesExt, LE};
        // read footer to get index, encryption & compression info
        reader.seek(io::SeekFrom::End(-version.footer_size()))?;
        let footer = super::footer::Footer::new(&mut reader, version)?;
        // read index to get all the entry info
        reader.seek(io::SeekFrom::Start(footer.index_offset))?;
        let mut index = reader.read_len(footer.index_size as usize)?;
        let mut key = None;
        // decrypt index if needed
        if footer.encrypted {
            let Some(hash) = key_hash else {
                return Err(super::Error::Encrypted);
            };
            use aes::cipher::KeyInit;
            use base64::Engine;
            let Ok(dec)= aes::Aes256Dec::new_from_slice(
                &base64::engine::general_purpose::STANDARD.decode(hash)?
            ) else {
                return Err(super::Error::Aes)
            };
            key = Some(dec);
            super::decrypt(key.as_ref(), &mut index)?;
        }
        let mut index = io::Cursor::new(index);
        let mount_point = index.read_string()?;
        // with_capacity doesn't set capacity exactly
        let mut entries = hashbrown::HashMap::new();
        if version >= Version::PathHashIndex {
            // entry count
            index.read_u32::<LE>()?;
            // path hash seed
            index.read_u64::<LE>()?;
            // path hash
            if index.read_u32::<LE>()? != 0 {
                // offset
                index.read_u64::<LE>()?;
                // size
                index.read_u64::<LE>()?;
                // hash
                index.read_guid()?;
                // no need to look at the path hash information
            }
            let mut files = Vec::new();
            // full directory index
            if index.read_u32::<LE>()? != 0 {
                let offset = index.read_u64::<LE>()?;
                let size = index.read_u64::<LE>()?;
                // hash
                index.read_guid()?;
                reader.seek(io::SeekFrom::Start(offset))?;
                let mut full_dir = reader.read_len(size as usize)?;
                if footer.encrypted {
                    super::decrypt(key.as_ref(), &mut full_dir)?;
                }
                let mut full_dir = io::Cursor::new(full_dir);
                for _ in 0..full_dir.read_u32::<LE>()? {
                    let dir = full_dir.read_name()?;
                    for _ in 0..full_dir.read_u32::<LE>()? {
                        files.push((
                            dir.clone() + &full_dir.read_string()?,
                            full_dir.read_u32::<LE>()?,
                        ));
                    }
                }
            }
            let size = index.read_u32::<LE>()? as usize;
            let mut encoded = io::Cursor::new(index.read_len(size)?);
            for (file, offset) in files {
                use io::Seek;
                encoded.seek(io::SeekFrom::Start(offset as u64))?;
                entries.insert(file, super::entry::Entry::from_encoded(&mut encoded)?);
            }
        }
        for _ in 0..index.read_u32::<LE>()? as usize {
            entries.insert(
                index.read_name()?,
                super::entry::Entry::new(&mut index, version)?,
            );
        }

        Ok(Self {
            version,
            mount_point,
            key,
            entries,
            reader,
        })
    }

    pub fn version(&self) -> super::Version {
        self.version
    }

    pub fn mount_point(&self) -> &str {
        &self.mount_point
    }

    pub fn get(&mut self, path: &str) -> Result<Vec<u8>, super::Error> {
        let mut data = Vec::new();
        self.read(path, &mut data)?;
        Ok(data)
    }

    pub fn read<W: io::Write>(&mut self, path: &str, writer: &mut W) -> Result<(), super::Error> {
        match self.entries.get(path) {
            Some(entry) => entry.read(&mut self.reader, self.version, self.key.as_ref(), writer),
            None => Err(super::Error::Other("no file found at given path")),
        }
    }

    pub fn files(&self) -> std::vec::IntoIter<String> {
        self.entries
            .keys()
            .cloned()
            .collect::<Vec<String>>()
            .into_iter()
    }
}
