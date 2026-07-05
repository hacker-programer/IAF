use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::error::Error;

#[allow(dead_code)]
pub async fn get_voyage_embeddings(text: &str, api_key: &str) -> Result<Vec<f32>, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let response = client
        .post("https://api.voyageai.com/v1/embeddings")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&json!({
            "input": [text],
            "model": "voyage-code-2"
        }))
        .send()
        .await?;

    let resp_json: Value = response.json().await?;
    let embedding = resp_json["data"][0]["embedding"]
        .as_array()
        .ok_or("Invalid response from VoyageAI")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    Ok(embedding)
}

#[allow(dead_code)]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
