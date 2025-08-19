use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;
use serde::Serialize;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use backend::embed::Embedder;

#[derive(Serialize)]
struct Point<'a> {
    id: u64,
    vector: &'a [f32],
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<&'a Value>,
}

#[derive(Serialize)]
struct PointsBatch<'a> {
    points: Vec<Point<'a>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest_dir.join("data");
    let input_path = data_dir.join("reviews.jsonl");
    if !input_path.exists() {
        eprintln!("reviews.jsonl not found in {:?}", input_path);
        std::process::exit(1);
    }
    // QDRANT endpoint; override via env var
    let qdrant_url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
    let collection = std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "reviews".to_string());

    let client = Client::new();
    let embedder = Embedder::new()?;

    const BATCH: usize = 64;
    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);

    let mut batch_points: Vec<Vec<f32>> = Vec::with_capacity(BATCH);
    let mut batch_payloads: Vec<Value> = Vec::with_capacity(BATCH);
    let mut total: u64 = 0;
    for line in reader.lines() {
        let line = line?;
        let v: Value = serde_json::from_str(&line)?;
        let text = extract_text(&v);
        let embedding = embedder.embed_default(&text);
        batch_points.push(embedding);
        batch_payloads.push(v);

        if batch_points.len() == BATCH {
            upload_batch(&client, &qdrant_url, &collection, total, &batch_points, &batch_payloads).await?;
            total += batch_points.len() as u64;
            batch_points.clear();
            batch_payloads.clear();
        }
    }
    if !batch_points.is_empty() {
        upload_batch(&client, &qdrant_url, &collection, total, &batch_points, &batch_payloads).await?;
    }
    println!("Finished uploading to Qdrant. Total points: {}", total + batch_points.len() as u64);
    Ok(())
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

async fn upload_batch(
    client: &Client,
    base_url: &str,
    collection: &str,
    start_id: u64,
    vectors: &[Vec<f32>],
    payloads: &[Value],
) -> Result<()> {
    let points: Vec<_> = vectors
        .iter()
        .enumerate()
        .map(|(i, vec)| Point {
            id: start_id + i as u64,
            vector: vec,
            payload: Some(&payloads[i]),
        })
        .collect();

    let body = serde_json::json!({ "points": points });

    let url = format!("{}/collections/{}/points?wait=true", base_url, collection);
    let resp = client.put(url).json(&body).send().await?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Qdrant error: {}", text);
    }
    // small delay to avoid overloading
    sleep(Duration::from_millis(100)).await;
    Ok(())
}
