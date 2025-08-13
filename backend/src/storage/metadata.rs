use anyhow::Result;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct MetadataStore {
    path: PathBuf,
}

impl MetadataStore {
    pub fn open_or_create(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            File::create(&path)?;
        }
        Ok(Self { path })
    }

    pub fn append<T: Serialize>(&mut self, item: &T) -> Result<()> {
        let file = OpenOptions::new().append(true).open(&self.path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, item)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    pub fn get_by_index<T: for<'de> serde::Deserialize<'de>>(&self, index: usize) -> Result<T> {
        use std::io::{BufRead, BufReader};
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let line = reader
            .lines()
            .nth(index)
            .ok_or_else(|| anyhow::anyhow!("index out of bounds"))??;
        let value = serde_json::from_str::<T>(&line)?;
        Ok(value)
    }
}
