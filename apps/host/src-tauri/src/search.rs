// Native sheet search fetch — same CORS-dodge pattern as data::load_chart_data.
#[tauri::command]
pub async fn search_sheets(search_url: String) -> Result<serde_json::Value, String> {
    match reqwest::get(&search_url).await {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| e.to_string())
        }
        Ok(resp) => Err(format!("sheet search fetch failed: HTTP {}", resp.status())),
        Err(e) => Err(format!("sheet search fetch failed: {e}")),
    }
}
