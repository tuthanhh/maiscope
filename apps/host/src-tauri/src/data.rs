// Native catalog fetch. Pulls the full catalog snapshot from the global backend
// (apps/server `GET /catalog`); the web build fetches the same URL directly.
// Offline caching is not implemented yet — a failed fetch surfaces as an error.
#[tauri::command]
pub async fn load_chart_data(catalog_url: String) -> Result<serde_json::Value, String> {
    match reqwest::get(&catalog_url).await {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }
        Ok(resp) => Err(format!("catalog fetch failed: HTTP {}", resp.status())),
        Err(e) => Err(format!("catalog fetch failed: {e}")),
    }
}
