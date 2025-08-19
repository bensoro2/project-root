use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use backend::embed::Embedder;
use reqwest::Client;
use serde_json::{json, Value};

#[cfg(feature = "spfresh")]
use spfresh::Index as VectorIndex;
#[cfg(not(feature = "spfresh"))]
use backend::storage::vector_store::VectorStore as VectorIndex;

const SAMPLE_QUERIES: usize = 1000;
const TOP_K: usize = 100;

#[tokio::main]
async fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest_dir.join("data");
    let input_path = data_dir.join("reviews.jsonl");
    let index_path = {
        #[cfg(feature = "spfresh")]
        {
            data_dir.join("reviews")
        }
        #[cfg(not(feature = "spfresh"))]
        {
            data_dir.join("reviews.vectors")
        }
    };

    let mut index = VectorIndex::open_or_create(index_path)?;
    let embedder = Embedder::new()?;
    let client = Client::new();
    let qdrant_url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
    let collection = std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "reviews".to_string());

    // sample queries
    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let mut queries: Vec<String> = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        if i >= SAMPLE_QUERIES {
            break;
        }
        let l = line?;
        let v: Value = serde_json::from_str(&l)?;
        queries.push(extract_text(&v));
    }

    let mut sp_times: Vec<u128> = Vec::with_capacity(SAMPLE_QUERIES);
    let mut q_times: Vec<u128> = Vec::with_capacity(SAMPLE_QUERIES);
    let mut recalls: Vec<f32> = Vec::with_capacity(SAMPLE_QUERIES);

    for (idx, q) in queries.iter().enumerate() {
        let emb = embedder.embed_default(q);

        // spfresh search
        let t0 = Instant::now();
        let sp_results = index.search(&emb, TOP_K)?; // Vec<(usize, f32)>
        let sp_ids: Vec<usize> = sp_results.iter().map(|(id, _)| *id).collect();
        sp_times.push(t0.elapsed().as_micros());

        // qdrant search
        let url = format!("{}/collections/{}/points/search", qdrant_url, collection);
        let body = json!({"vector": emb, "top": TOP_K});
        let t1 = Instant::now();
        let resp = client.post(&url).json(&body).send().await?;
        let result_json: Value = resp.json().await?;
        let q_ids: Vec<usize> = result_json["result"].as_array().unwrap_or(&Vec::new())
            .iter()
            .filter_map(|p| p["id"].as_u64())
            .map(|v| v as usize)
            .collect();
        q_times.push(t1.elapsed().as_micros());

        // recall@k
        let overlap = sp_ids.iter().filter(|id| q_ids.contains(id)).count() as f32;
        recalls.push(overlap / TOP_K as f32);

        if idx % 20 == 0 {
            println!("Processed {} / {} queries", idx + 1, SAMPLE_QUERIES);
        }
    }

    let avg_sp = average(&sp_times);
    let avg_q = average(&q_times);
    let avg_recall = recalls.iter().sum::<f32>() / recalls.len() as f32;

    println!("\n=== Benchmark Results ===");
    println!("Avg latency (µs)  - spfresh: {:.2}, qdrant: {:.2}", avg_sp, avg_q);
    println!("Avg recall@{}      : {:.3}", TOP_K, avg_recall);

    Ok(())
}

fn average(xs: &[u128]) -> f64 {
    if xs.is_empty() { return 0.0; }
    let sum: u128 = xs.iter().sum();
    (sum as f64) / (xs.len() as f64)
}

fn extract_text(v: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(title) = v.get("review_title").and_then(|x| x.as_str()) {
        if !title.trim().is_empty() {
            parts.push(title.trim());
        }
    }
    if let Some(body) = v.get("review_body").and_then(|x| x.as_str()) {
        if !body.trim().is_empty() {
            parts.push(body.trim());
        }
    }
    if parts.is_empty() {
        v.to_string()
    } else {
        parts.join(" ")
    }
}
