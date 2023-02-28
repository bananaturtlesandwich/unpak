use super::Version;
use std::io;

/// the pak file with all the goodies
#[derive(Debug)]
pub struct Pak {
    version: Version,
    mount_point: String,
    key: Option<aes::Aes256Dec>,
    entries: hashbrown::HashMap<String, super::entry::Entry>,
}

impl Pak {
    /// reads a pak file from the provided reader with a known version and optional key
    pub fn new<R: io::Read + io::Seek>(
        reader: &mut R,
        version: super::Version,
        key_hash: Option<&[u8]>,
    ) -> Result<Self, super::Error> {
        use super::ext::ReadExt;
        use byteorder::{ReadBytesExt, LE};
        // read footer to get index, encryption & compression info
        reader.seek(io::SeekFrom::End(-version.footer_size()))?;
        let footer = super::footer::Footer::new(reader, version)?;
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
            let Ok(dec) = aes::Aes256Dec::new_from_slice(hash) else {
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
        })
    }

    /// reads a pak file from the provided path with a known version and optional key
    pub fn new_from_path(
        path: impl AsRef<std::path::Path>,
        version: super::Version,
        key: Option<&[u8]>,
    ) -> Result<Pak, super::Error> {
        Pak::new(&mut std::fs::File::open(path)?, version, key)
    }

    /// reads a pak file from the provided reader with a guessed version and optional key
    pub fn new_any<R: io::Read + io::Seek>(
        reader: &mut R,
        key: Option<&[u8]>,
    ) -> Result<Pak, super::Error> {
        for ver in Version::iter().rev() {
            if let Ok(pak) = Pak::new(reader, ver, key) {
                return Ok(pak);
            }
        }
        Err(super::Error::Parse)
    }

    /// reads a pak file from the provided path with a guessed version and optional key
    pub fn new_any_from_path(
        path: impl AsRef<std::path::Path>,
        key: Option<&[u8]>,
    ) -> Result<Pak, super::Error> {
        Pak::new_any(&mut std::fs::File::open(path)?, key)
    }

    pub fn version(&self) -> super::Version {
        self.version
    }

    pub fn mount_point(&self) -> &str {
        &self.mount_point
    }

    /// gets the entry as a vector of bytes from the reader corresponding to the pak
    pub fn get<R: io::Read + io::Seek>(
        &self,
        entry: &str,
        reader: &mut R,
    ) -> Result<Vec<u8>, super::Error> {
        let mut data = Vec::new();
        self.read(entry, reader, &mut data)?;
        Ok(data)
    }

    /// gets the entry as a vector of bytes from the path corresponding to the pak
    pub fn get_from_path(
        &self,
        entry: &str,
        reader: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, super::Error> {
        let mut data = Vec::new();
        self.read(entry, &mut std::fs::File::open(reader)?, &mut data)?;
        Ok(data)
    }

    /// reads the entry into any writer from the reader corresponding to the pak
    pub fn read<R: io::Read + io::Seek, W: io::Write>(
        &self,
        entry: &str,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<(), super::Error> {
        match self.entries.get(entry) {
            Some(entry) => entry.read(reader, self.version, self.key.as_ref(), writer),
            None => Err(super::Error::Missing(entry.to_string())),
        }
    }

    /// reads the entry into the given file from the reader corresponding to the pak
    pub fn read_to_file<R: io::Read + io::Seek>(
        &self,
        entry: &str,
        reader: &mut R,
        writer: impl AsRef<std::path::Path>,
    ) -> Result<(), super::Error> {
        match self.entries.get(entry) {
            Some(entry) => entry.read(
                reader,
                self.version,
                self.key.as_ref(),
                &mut std::fs::File::create(writer)?,
            ),
            None => Err(super::Error::Missing(entry.to_string())),
        }
    }

    /// reads the entry into any writer from the path corresponding to the pak
    pub fn read_from_path<W: io::Write>(
        &self,
        entry: &str,
        reader: impl AsRef<std::path::Path>,
        writer: &mut W,
    ) -> Result<(), super::Error> {
        match self.entries.get(entry) {
            Some(entry) => entry.read(
                &mut std::fs::File::open(reader)?,
                self.version,
                self.key.as_ref(),
                writer,
            ),
            None => Err(super::Error::Missing(entry.to_string())),
        }
    }

    /// reads the entry into the given file from the path corresponding to the pak
    pub fn read_from_path_to_file(
        &self,
        entry: &str,
        reader: impl AsRef<std::path::Path>,
        writer: impl AsRef<std::path::Path>,
    ) -> Result<(), super::Error> {
        match self.entries.get(entry) {
            Some(entry) => entry.read(
                &mut std::fs::File::open(reader)?,
                self.version,
                self.key.as_ref(),
                &mut std::fs::File::create(writer)?,
            ),
            None => Err(super::Error::Missing(entry.to_string())),
        }
    }

    /// gets an iterator over the names of each entry
    pub fn entries(&self) -> std::vec::IntoIter<String> {
        self.entries
            .keys()
            .cloned()
            .collect::<Vec<String>>()
            .into_iter()
    }

    /// gets a parallel iterator over the names of each entry
    #[cfg(feature = "rayon")]
    pub fn par_entries(&self) -> rayon::vec::IntoIter<String> {
        use rayon::prelude::IntoParallelIterator;
        self.entries
            .keys()
            .cloned()
            .collect::<Vec<String>>()
            .into_par_iter()
    }
}
