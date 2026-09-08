// Tauri wrapper: the native (desktop/Android) shell around the web frontend.
// Only desktop/mobile builds run this Rust code — the web build skips it
// entirely. Kept minimal for now; native-only features (e.g. offline chart
// caching via `data::load_chart_data`) live behind Tauri commands here.
mod data;
mod search;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            data::load_chart_data,
            search::search_sheets
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
