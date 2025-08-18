use anyhow::Result;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct MetadataStore {
    path: PathBuf,
}

impl MetadataStore {
    pub fn open_or_create(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            File::create(&path)
                .map_err(|e| anyhow::anyhow!("Failed to create metadata store file: {}", e))?;
        }
        Ok(Self { path })
    }

    pub fn append<T: Serialize>(&mut self, item: &T) -> Result<()> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)
            .map_err(|e| anyhow::anyhow!("Failed to open metadata store file: {}", e))?;
        let mut writer = BufWriter::new(file);
        
        serde_json::to_writer(&mut writer, item)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metadata item: {}", e))?;
        writer.write_all(b"\n")
            .map_err(|e| anyhow::anyhow!("Failed to write newline to metadata store: {}", e))?;
        writer.flush()
            .map_err(|e| anyhow::anyhow!("Failed to flush metadata store writer: {}", e))?;
        Ok(())
    }

    pub fn get_by_index<T: for<'de> serde::Deserialize<'de>>(&self, index: usize) -> Result<T> {
        let file = File::open(&self.path)
            .map_err(|e| anyhow::anyhow!("Failed to open metadata store file: {}", e))?;
        let reader = BufReader::new(file);
        
        let line = reader
            .lines()
            .nth(index)
            .ok_or_else(|| anyhow::anyhow!("index out of bounds"))?
            .map_err(|e| anyhow::anyhow!("Failed to read line from metadata store: {}", e))?;
            
        let value = serde_json::from_str::<T>(&line)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON from metadata store: {}", e))?;
            
        Ok(value)
    }
}
