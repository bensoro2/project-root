use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;
use std::io::Write;
use rayon::prelude::*;
use serde_json::Value;

use backend::embed::Embedder;
#[cfg(feature = "spfresh")]
use spfresh::Index as VectorIndex;
#[cfg(not(feature = "spfresh"))]
use backend::storage::vector_store::VectorStore as VectorIndex;

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest_dir.join("data");
    let input_path = data_dir.join("reviews.jsonl");
    if !input_path.exists() {
        eprintln!("Input file {:?} not found", input_path);
        std::process::exit(1);
    }
    let total_lines: usize = {
        let f = File::open(&input_path)?;
        let reader = BufReader::new(f);
        reader.lines().count()
    };
    println!("Initializing embedder...");
    let embedder = Embedder::new()?;
    println!("Resetting index directory/file ...");
    let index_path = {
        #[cfg(feature = "spfresh")]
        { data_dir.join("reviews") }
        #[cfg(not(feature = "spfresh"))]
        { data_dir.join("reviews.vectors") }
    };
    if index_path.exists() {
        if index_path.is_dir() {
            std::fs::remove_dir_all(&index_path)?;
        } else {
            std::fs::remove_file(&index_path)?;
        }
    }
    #[cfg(feature = "spfresh")]
    let index_path = data_dir.join("reviews");
    #[cfg(not(feature = "spfresh"))]
    let index_path = data_dir.join("reviews.vectors");
    let mut index = VectorIndex::open_or_create(index_path)?;
    const BATCH: usize = 200;

    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let mut buffer = Vec::with_capacity(BATCH);
    let mut processed: usize = 0;
    for (_idx, line) in reader.lines().enumerate() {
                let line = line?;
                buffer.push(line);
        if buffer.len() == BATCH {
            process_batch(&buffer, &embedder, &mut index)?;
            processed += buffer.len();
            print!("Processed {} / {}\r", processed, total_lines);
            std::io::stdout().flush()?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        process_batch(&buffer, &embedder, &mut index)?;
        processed += buffer.len();
    }
    #[cfg(feature = "spfresh")]
    println!("Index build completed. Total vectors: {}", index.len());
    #[cfg(not(feature = "spfresh"))]
    println!("Index build completed. Total vectors: {}", index.len()?);
    Ok(())
}

fn extract_text(v: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(title) = v.get("review_title").and_then(|x| x.as_str()) {
        if !title.trim().is_empty() { parts.push(title.trim()); }
    }
    if let Some(body) = v.get("review_body").and_then(|x| x.as_str()) {
        if !body.trim().is_empty() { parts.push(body.trim()); }
    }
    if parts.is_empty() { v.to_string() } else { parts.join(" ") }
}

fn process_batch(lines: &[String], embedder: &Embedder, index: &mut VectorIndex) -> Result<()> {
    let embeddings: Vec<Vec<f32>> = lines
        .par_iter()
        .map(|l| {
            match serde_json::from_str::<Value>(l) {
                Ok(v) => embedder.embed_default(&extract_text(&v)),
                Err(_) => {
                    embedder.embed_default(l)
                }
            }
        })
        .collect();
    for (i, embedding) in embeddings.iter().enumerate() {
        index.append(embedding)?;
        if i % 1000 == 0 {
            println!("Appended {} vectors", i);
        }
    }
    Ok(())
}
