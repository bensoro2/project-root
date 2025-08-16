use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;

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
    let start_time = Instant::now();
    println!("Starting index builder...");
    // Locate data directory relative to crate root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest_dir.join("data");
    let input_path = data_dir.join("reviews.jsonl");
    println!("Looking for input file at: {:?}", input_path);
    if !input_path.exists() {
        eprintln!("Input file {:?} not found", input_path);
        std::process::exit(1);
    }
    println!("Counting total lines...");
    let total_lines: usize = {
        let f = File::open(&input_path)?;
        let reader = BufReader::new(f);
        reader.lines().count()
    };
    println!("Total lines: {}", total_lines);
    println!("Found input file");

    // Instantiate embedder & vector index
    println!("Initializing embedder...");
    let embedder = Embedder::new()?;
    println!("Embedder initialized");

    println!("Opening or creating index...");
    #[cfg(feature = "spfresh")]
    let mut index = VectorIndex::open_or_create(data_dir.join("reviews"))?;
    #[cfg(not(feature = "spfresh"))]
    let mut index = VectorIndex::open_or_create(data_dir.join("reviews.vectors"))?;
    println!("Index opened or created");

    // Stream JSONL file in batches
    const BATCH: usize = 1000; // Reduce batch size to improve responsiveness
    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let mut buffer = Vec::with_capacity(BATCH);
    let mut processed: usize = 0;

    println!("Starting to process lines...");
    for line in reader.lines() {
        let line = line?;
        buffer.push(line);
        if buffer.len() == BATCH {
            println!("Processing batch of {} lines...", buffer.len());
            process_batch(&buffer, &embedder, &mut index)?;
            processed += buffer.len();
            let pct = (processed as f64 * 100.0) / (total_lines as f64);
            print!("Processed {} / {} ({:.1}%)\r", processed, total_lines, pct);
            std::io::stdout().flush()?;
            std::io::stdout().flush()?;
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        println!("Processing final batch of {} lines...", buffer.len());
        process_batch(&buffer, &embedder, &mut index)?;
        processed += buffer.len();
        let pct = (processed as f64 * 100.0) / (total_lines as f64);
            print!("Processed {} / {} ({:.1}%)\r", processed, total_lines, pct);
            std::io::stdout().flush()?;
        std::io::stdout().flush()?;
    }

    #[cfg(feature = "spfresh")]
    println!("Index build completed. Total vectors: {}", index.len());
    #[cfg(not(feature = "spfresh"))]
    println!("Index build completed. Total vectors: {}", index.len()?);
    let duration = start_time.elapsed();
    println!("Program finished in {:?}", duration);
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
    println!("Processing batch of {} lines", lines.len());
    // Parse and embed in parallel
    let embeddings: Vec<Vec<f32>> = lines
        .par_iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| embedder.embed(&extract_text(&v)))
        .collect();
    println!("Generated {} embeddings", embeddings.len());

    // Append sequentially to preserve order
    for (i, embedding) in embeddings.iter().enumerate() {
        index.append(embedding)?;
        if i % 1000 == 0 {
            println!("Appended {} vectors", i);
        }
    }
    println!("Batch processing completed");
    Ok(())
}
