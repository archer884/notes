use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use flate2::{
    read::{GzDecoder as Decoder, GzEncoder as Encoder},
    Compression,
};
use serde::{Deserialize, Serialize};

pub fn zip<T: Serialize>(s: T) -> io::Result<Vec<u8>> {
    let s = bincode::serialize(&s).map_err(io::Error::other)?;
    let mut buf = Vec::with_capacity(s.len());
    let mut encoder = Encoder::new(&*s, Compression::fast());
    encoder.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn unzip<T>(path: &Path) -> io::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let d = fs::read(path)?;
    let mut buf = Vec::new();
    let mut decoder = Decoder::new(&*d);
    decoder.read_to_end(&mut buf)?;
    bincode::deserialize(&buf).map_err(io::Error::other)
}
