use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use reqwest::Client;
use regex::Regex;
use crate::state::CaptchaRequest;

/// Regex estática para limpiar tags HTML. Se compila una sola vez (LUT de regex).
static TAG_CLEAN_REGEX: OnceLock<Regex> = OnceLock::new();

/// Realiza una búsqueda web usando DuckDuckGo Lite (HTML simple, no requiere API key).
/// DuckDuckGo es mucho más amigable con scrapers que Google, que bloquea
/// agresivamente las peticiones automatizadas.
///
/// Si DuckDuckGo falla, intenta Google como fallback (poco probable que funcione).
pub async fn perform_search(
    query: &str,
    pending_captcha: Arc<Mutex<Option<CaptchaRequest>>>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Intentar DuckDuckGo Lite primero (mucho más fiable que Google)
    match search_duckduckgo(query).await {
        Ok(results) if !results.is_empty() && results != "No se encontraron resultados." => {
            return Ok(results);
        }
        _ => {
            // Fallback: intentar Google (probablemente falle, pero por si acaso)
            match search_google(query, pending_captcha).await {
                Ok(results) => return Ok(results),
                Err(e) => {
                    // Si ambos fallan, devolver lo que tengamos de DDG
                    match search_duckduckgo(query).await {
                        Ok(results) => return Ok(format!("(Google falló: {}) Resultados de DuckDuckGo:\n{}", e, results)),
                        Err(_) => return Err(format!("Tanto Google como DuckDuckGo fallaron. Google: {}", e).into()),
                    }
                }
            }
        }
    }
}

/// Busca en DuckDuckGo Lite, que devuelve HTML simple y fácil de parsear.
/// URL: https://lite.duckduckgo.com/lite?q=...
async fn search_duckduckgo(query: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build()?;

    let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query));
    let response = client.get(&url)
        .header("Accept", "text/html,application/xhtml+xml")
        .send()
        .await?;
    
    let body = response.text().await?;

    let mut results = Vec::new();

    let link_re = Regex::new(r#"<a[^>]*rel="nofollow"[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#)?;
    for cap in link_re.captures_iter(&body) {
        let url = &cap[1];
        let title = scraper_clean_tags(&cap[2]);
        if !title.trim().is_empty() && !url.contains("duckduckgo.com") {
            results.push(format!("{} ({})", title.trim(), url));
        }
    }

    if results.is_empty() {
        let snippet_re = Regex::new(r#"<td[^>]*class="[^"]*result-snippet[^"]*"[^>]*>(.*?)</td>"#)?;
        for cap in snippet_re.captures_iter(&body) {
            let snippet = scraper_clean_tags(&cap[1]);
            if !snippet.trim().is_empty() {
                results.push(snippet.trim().to_string());
            }
        }
    }

    if results.is_empty() {
        let any_link_re = Regex::new(r#"<a[^>]*href="(https?://[^"]*)"[^>]*>([^<]+)</a>"#)?;
        for cap in any_link_re.captures_iter(&body) {
            let url = &cap[1];
            let title = scraper_clean_tags(&cap[2]);
            if !title.trim().is_empty() 
                && !url.contains("duckduckgo.com") 
                && !url.contains("duck.com")
                && title.trim().len() > 3 
            {
                results.push(format!("{} ({})", title.trim(), url));
            }
            if results.len() >= 10 {
                break;
            }
        }
    }

    if results.is_empty() {
        Ok("No se encontraron resultados en DuckDuckGo. Intenta refinar la búsqueda.".to_string())
    } else {
        results.truncate(10);
        Ok(results.join("\n"))
    }
}

/// Fallback: búsqueda en Google (probablemente falle por CAPTCHA/bloqueo).
/// FIX #24: Espera máxima de 10 segundos para resolver CAPTCHA (antes eran 60s).
async fn search_google(
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

        // FIX #24: 10 segundos máximo (20 iteraciones × 500ms) en vez de 60s
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let current = pending_captcha.lock().unwrap();
            if let Some(ref req) = *current {
                if req.id == id && req.solved_content.is_some() {
                    return Ok(req.solved_content.as_ref().unwrap().clone());
                }
            } else {
                break;
            }
        }
        return Err("Google CAPTCHA triggered — timeout después de 10s. Usando DuckDuckGo como respaldo.".into());
    }

    let mut results = Vec::new();
    let re = Regex::new(r#"<h3[^>]*>(.*?)</h3>"#)?;
    for cap in re.captures_iter(&body) {
        let title = cap[1].replace("href=\"", "").replace("</a>", "");
        let clean_title = scraper_clean_tags(&title);
        results.push(clean_title);
    }

    if results.is_empty() {
        Err("Google no devolvió resultados (probablemente bloqueó la petición). Usa search_google con DuckDuckGo como respaldo automático.".into())
    } else {
        Ok(results.join("\n"))
    }
}

/// Limpia todas las etiquetas HTML de un string.
/// Usa OnceLock para compilar la regex una sola vez (LUT de regex precomputada).
pub fn scraper_clean_tags(html: &str) -> String {
    let re = TAG_CLEAN_REGEX.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
    re.replace_all(html, "").to_string()
}
