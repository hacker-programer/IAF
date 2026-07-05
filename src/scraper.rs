use std::sync::{Arc, Mutex};
use std::time::Duration;
use reqwest::Client;
use regex::Regex;
use crate::state::CaptchaRequest;

pub async fn perform_search(
    query: &str,
    pending_captcha: Arc<Mutex<Option<CaptchaRequest>>>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build()?;

    let url = format!("https://www.google.com/search?q={}", urlencoding::encode(query));
    let response = client.get(&url).send().await?;
    let body = response.text().await?;

    if body.contains("detected unusual traffic") || body.contains("captcha") || body.contains("g-recaptcha") || body.contains("not a robot") {
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut cap = pending_captcha.lock().unwrap();
            *cap = Some(CaptchaRequest {
                id: id.clone(),
                sitekey: String::new(),
                url: url.clone(),
                solved_content: None,
            });
        }

        // Wait up to 60 seconds for user to solve captcha in browser
            tokio::time::sleep(Duration::from_millis(500)).await;
            let current = pending_captcha.lock().unwrap();
            if let Some(ref req) = *current {
                if req.id == id && req.solved_content.is_some() {
                    return Ok(req.solved_content.as_ref().unwrap().clone());
                }
            } else {
                break; // Cleared
            }
        }
        return Err("Google CAPTCHA was triggered and not solved in time by the user".into());
    }

    // Basic extraction of text results
    let mut results = Vec::new();
    let re = Regex::new(r#"<h3[^>]*>(.*?)</h3>"#)?;
    for cap in re.captures_iter(&body) {
        let title = cap[1].replace("href=\"", "").replace("</a>", "");
        let clean_title = scraper_clean_tags(&title);
        results.push(clean_title);
    }

    if results.is_empty() {
        Ok("No se encontraron resultados o el formato de Google ha cambiado.".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

pub fn scraper_clean_tags(html: &str) -> String {
    let re = Regex::new(r#"<[^>]*>"#).unwrap();
    re.replace_all(html, "").to_string()
}
