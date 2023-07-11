use std::{
    collections::HashMap,
    error::Error,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
};

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};

use crate::hash::{self, Hash};

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    path: PathBuf,
    hash: hash::Hash,
}

pub struct Database {
    entries: HashMap<PathBuf, Hash>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Database, Box<dyn Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let mut entries = HashMap::new();
        if file.metadata()?.len() > 0 {
            let decoder = GzDecoder::new(file);
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(decoder);

            for entry in reader.deserialize() {
                let entry: Record = entry?;
                entries.insert(entry.path, entry.hash);
            }
        }

        Ok(Database { entries })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), io::Error> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;

        let encoder = GzEncoder::new(file, Compression::default());
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(encoder);

        for entry in &self.entries {
            writer.serialize(Record {
                path: entry.0.clone(),
                hash: entry.1.clone(),
            })?;
        }

        Ok(())
    }

    pub fn get(&self, path: impl AsRef<Path>) -> Option<&Hash> {
        self.entries.get(path.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &Hash)> {
        self.entries.iter()
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = (PathBuf, Hash)>) {
        self.entries.extend(iter)
    }

    pub fn retain(&mut self, f: impl Fn(&PathBuf) -> bool) {
        self.entries.retain(|k, _| f(k))
    }
}

#[cfg(test)]
mod tests {
    use blake2::{Blake2b512, Digest};

    use super::*;
    use std::io::Cursor;

    #[test]
    fn can_write_record() {
        let mut hasher = Blake2b512::new();
        hasher.update(b"hello world");

        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(Cursor::new(Vec::new()));

        writer
            .serialize(
                &(Record {
                    path: PathBuf::from(r"/some/path"),
                    hash: hash::Hash::Blake2b512(hasher.finalize()),
                }),
            )
            .unwrap();

        assert_eq!(
            "/some/path,blake2b:021CED8799296CECA557832AB941A50B4A11F83478CF141F51F933F653AB9FBCC05A037CDDBED06E309BF334942C4E58CDF1A46E237911CCD7FCF9787CBC7FD0\n",
            std::str::from_utf8(&writer.into_inner().unwrap().into_inner()).unwrap()
        );
    }

    #[test]
    fn can_read_record() {
        let data =
            "/some/path,blake2b:021CED8799296CECA557832AB941A50B4A11F83478CF141F51F933F653AB9FBCC05A037CDDBED06E309BF334942C4E58CDF1A46E237911CCD7FCF9787CBC7FD0\n"
                .as_bytes();

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(Cursor::new(data));

        let record: Record = reader.deserialize().next().unwrap().unwrap();
        assert_eq!(PathBuf::from(r"/some/path"), record.path);

        let mut hasher = Blake2b512::new();
        hasher.update(b"hello world");
        assert_eq!(hash::Hash::Blake2b512(hasher.finalize()), record.hash);
    }
}
