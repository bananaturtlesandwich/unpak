use super::Version;
use std::io;

/// the pak file with all the goodies
#[derive(Debug)]
pub struct Pak {
    version: Version,
    path: std::path::PathBuf,
    mount_point: String,
    #[cfg(feature = "encryption")]
    key: Option<aes::Aes256Dec>,
    entries: hashbrown::HashMap<String, super::entry::Entry>,
}

impl Pak {
    /// reads a pak file with a known version
    pub fn new(
        path: impl AsRef<std::path::Path>,
        version: super::Version,
        #[cfg(feature = "encryption")] key_hash: Option<&[u8]>,
    ) -> Result<Self, super::Error> {
        use super::ext::ReadExt;
        use byteorder::{ReadBytesExt, LE};
        use io::Seek;
        let mut reader = std::fs::File::open(&path)?;
        // read footer to get index, encryption & compression info
        reader.seek(io::SeekFrom::End(-version.footer_size()))?;
        let footer = super::footer::Footer::new(&mut reader, version)?;
        // read index to get all the entry info
        reader.seek(io::SeekFrom::Start(footer.index_offset))?;
        #[allow(unused_mut)]
        let mut index = reader.read_len(footer.index_size as usize)?;
        #[cfg(feature = "encryption")]
        let mut key = None;
        // decrypt index if needed
        if footer.encrypted {
            #[cfg(feature = "encryption")]
            {
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
            #[cfg(not(feature = "encryption"))]
            return Err(super::Error::Encryption);
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
                #[allow(unused_mut)]
                let mut full_dir = reader.read_len(size as usize)?;
                if footer.encrypted {
                    #[cfg(feature = "encryption")]
                    super::decrypt(key.as_ref(), &mut full_dir)?;
                    #[cfg(not(feature = "encryption"))]
                    return Err(super::Error::Encryption);
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
            path: path.as_ref().to_path_buf(),
            mount_point,
            #[cfg(feature = "encryption")]
            key,
            entries,
        })
    }

    /// reads a pak file with a guessed version
    pub fn new_any(
        path: impl AsRef<std::path::Path>,
        #[cfg(feature = "encryption")] key: Option<&[u8]>,
    ) -> Result<Pak, super::Error> {
        for ver in Version::iter().rev() {
            match Pak::new(
                &path,
                ver,
                #[cfg(feature = "encryption")]
                key,
            ) {
                Ok(pak) => return Ok(pak),
                Err(e) => match e {
                    crate::Error::Io(io) => match io.kind() {
                        io::ErrorKind::NotFound
                        | io::ErrorKind::PermissionDenied
                        | io::ErrorKind::AlreadyExists
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::InvalidInput
                        | io::ErrorKind::InvalidData
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::WriteZero
                        | io::ErrorKind::Interrupted
                        | io::ErrorKind::Unsupported => return Err(io.into()),
                        // eof or out of memory would indicate a wrong version
                        _ => continue,
                    },
                    crate::Error::Aes
                    | crate::Error::IntoInner(_)
                    | crate::Error::Encryption
                    | crate::Error::Compression
                    | crate::Error::Encrypted => return Err(e),
                    _ => continue,
                },
            }
        }
        Err(super::Error::Parse)
    }

    pub fn version(&self) -> super::Version {
        self.version
    }

    pub fn mount_point(&self) -> &str {
        &self.mount_point
    }

    /// reads the entry into any writer
    pub fn read<W: io::Write>(&self, entry: &str, writer: &mut W) -> Result<(), super::Error> {
        match self.entries.get(entry) {
            Some(entry) => entry.read(
                &self.path,
                self.version,
                #[cfg(feature = "encryption")]
                self.key.as_ref(),
                writer,
            ),
            None => Err(super::Error::Missing(entry.to_string())),
        }
    }

    /// reads the entry to the given path
    pub fn read_to_file(
        &self,
        entry: &str,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), super::Error> {
        match self.entries.get(entry) {
            Some(entry) => entry.read(
                &self.path,
                self.version,
                #[cfg(feature = "encryption")]
                self.key.as_ref(),
                &mut std::fs::File::create(path)?,
            ),
            None => Err(super::Error::Missing(entry.to_string())),
        }
    }

    /// gets the entry as a vector of bytes
    pub fn get(&self, entry: &str) -> Result<Vec<u8>, super::Error> {
        let mut data = Vec::new();
        self.read(entry, &mut data)?;
        Ok(data)
    }

    /// gets the names of all entries
    pub fn entries(&self) -> Vec<String> {
        self.entries.keys().cloned().collect::<Vec<String>>()
    }
}
