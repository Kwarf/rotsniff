use std::{
    fmt::Display,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use blake2::{digest::generic_array::GenericArray, Blake2b512, Digest};
use digest::crypto_common::BlockSizeUser;
use regex::Regex;
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};

pub fn blake2s(path: impl AsRef<Path>) -> Result<Hash, io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Blake2b512::new();
    let mut buffer = vec![0u8; Blake2b512::block_size()];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break Ok(Hash::Blake2b512(hasher.finalize())),
            Ok(n) => hasher.update(&buffer[..n]),
            Err(err) => break Err(err),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hash {
    Blake2b512(GenericArray<u8, blake2::digest::consts::U64>),
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hash::Blake2b512(bytes) => write!(f, "blake2b:{:X}", bytes),
        }
    }
}

struct HashVisitor;

impl<'de> Visitor<'de> for HashVisitor {
    type Value = Hash;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string matching \"blake2b:[a-fA-F0-9]{128}\"")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let regex = Regex::new("blake2b:([a-fA-F0-9]{128})").unwrap();

        if let Some(captures) = regex.captures(s) {
            let hex = captures.get(1).unwrap().as_str();
            let mut decoded = [0u8; 64];
            hex::decode_to_slice(hex, &mut decoded).unwrap();
            Ok(Hash::Blake2b512(decoded.into()))
        } else {
            Err(de::Error::invalid_value(de::Unexpected::Str(s), &self))
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(HashVisitor)
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
