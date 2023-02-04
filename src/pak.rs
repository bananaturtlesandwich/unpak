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
        reader.seek(io::SeekFrom::End(-version.size()))?;
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
            use aes::cipher::{BlockDecrypt, KeyInit};
            use base64::Engine;
            let Ok(decrypter)= aes::Aes256Dec::new_from_slice(
                &base64::engine::general_purpose::STANDARD.decode(hash)?
            ) else {
                return Err(super::Error::Aes)
            };
            for chunk in index.chunks_mut(16) {
                decrypter.decrypt_block(aes::Block::from_mut_slice(chunk))
            }
            key = Some(decrypter);
        }
        let mut index = io::Cursor::new(index);
        let mount_point = index.read_string()?;
        let len = index.read_u32::<LE>()? as usize;
        let mut entries = hashbrown::HashMap::with_capacity(len);
        for _ in 0..len {
            entries.insert(
                {
                    let mut path = index.read_string()?;
                    if let Some(pos) = path.find("Content") {
                        path.replace_range(0..pos + 7, "Game");
                    } else if let Some(pos) = path.find("Config") {
                        path.drain(0..pos);
                    } else if let Some(pos) = path.find("Plugins") {
                        path.drain(0..pos);
                    }
                    format!("/{path}")
                },
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
