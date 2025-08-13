use std::fs::File;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Tweet {
    pub id: String,
    pub user: String,
    pub fullname: String,
    pub url: Option<String>,
    pub timestamp: String,
    pub replies: Option<i32>,
    pub likes: Option<i32>,
    pub retweets: Option<i32>,
    pub text: String,
}

pub async fn insert_bitcoin_tweets() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("tweets.csv")?;
    let mut csv_reader = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(file);
    
    let client = reqwest::Client::new();
    let url = "http://localhost:8000/reviews/bulk";
    
    let mut batch = Vec::new();
    let batch_size = 4000;
    let mut count = 0;
    
    for result in csv_reader.deserialize() {
        let tweet: Tweet = result?;
        
        let review = crate::handlers::Review {
            review_title: format!("Tweet by {}", tweet.user),
            review_body: tweet.text,
            product_id: tweet.id.clone(),
            review_rating: 3,
        };
        
        batch.push(review);
        count += 1;
        
        if batch.len() >= batch_size {
            let response = client
                .post(url)
                .json(&batch)
                .send()
                .await?;
                
            if response.status().is_success() {
                println!("Successfully inserted batch of {} tweets", batch_size);
            } else {
                println!("Failed to insert batch: {}", response.status());
            }
            
            batch.clear();
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
    
    if !batch.is_empty() {
        let response = client
            .post(url)
            .json(&batch)
            .send()
            .await?;
            
        if response.status().is_success() {
            println!("Successfully inserted final batch of {} tweets", batch.len());
        } else {
            println!("Failed to insert final batch: {}", response.status());
        }
    }
    
    println!("Completed processing {} tweets", count);
    Ok(())
}