use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;
use rayon::prelude::*;
use serde_json::Value;

use backend::embed::Embedder;
#[cfg(feature = "spfresh")]
use spfresh::Index as VectorIndex;
#[cfg(not(feature = "spfresh"))]
use backend::storage::vector_store::VectorStore as VectorIndex;

fn main() -> Result<()> {
    // Locate data directory relative to crate root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest_dir.join("data");
    let input_path = data_dir.join("reviews.jsonl");
    if !input_path.exists() {
        eprintln!("Input file {:?} not found", input_path);
        std::process::exit(1);
    }

    // Instantiate embedder & vector index
    let embedder = Embedder::new()?;

    #[cfg(feature = "spfresh")]
    let mut index = VectorIndex::open_or_create(data_dir.join("reviews"))?;
    #[cfg(not(feature = "spfresh"))]
    let mut index = VectorIndex::open_or_create(data_dir.join("reviews.vectors"))?;

    // Stream JSONL file in batches
    const BATCH: usize = 2000;
    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let mut buffer = Vec::with_capacity(BATCH);

    for line in reader.lines() {
        let line = line?;
        buffer.push(line);
        if buffer.len() == BATCH {
            process_batch(&buffer, &embedder, &mut index)?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        process_batch(&buffer, &embedder, &mut index)?;
    }

    println!("Index build completed. Total vectors: {}", index.len());
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
    // Parse and embed in parallel
    let embeddings: Vec<Vec<f32>> = lines
        .par_iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| embedder.embed(&extract_text(&v)))
        .collect();

    // Append sequentially to preserve order
    for embedding in embeddings.iter() {
        index.append(embedding)?;
    }
    Ok(())
}
