mod beat_detection;
mod hyperframes;

use std::path::Path;
use std::sync::Mutex;
use tauri::State;

/// App state holding exported video path
struct AppState {
    export_dir: Mutex<String>,
}

#[tauri::command]
fn detect_beats(file_path: String) -> Result<beat_detection::BeatInfo, String> {
    beat_detection::detect_beats(&file_path)
}

#[tauri::command]
fn get_waveform(file_path: String, max_points: usize) -> Result<beat_detection::WaveformData, String> {
    beat_detection::get_waveform(&file_path, max_points)
}

#[tauri::command]
fn render_video(
    audio_path: String,
    image_paths: Vec<String>,
    beats: Vec<f64>,
    bpm: f64,
    state: State<AppState>,
) -> Result<String, String> {
    let export_dir = state.export_dir.lock().map_err(|e| e.to_string())?;

    // Create a unique project dir for this render
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let project_dir = Path::new(&*export_dir).join(format!("render_{}", timestamp));
    let output_path = Path::new(&*export_dir)
        .join(format!("beatcut_export_{}.mp4", timestamp))
        .to_str()
        .unwrap()
        .to_string();

    hyperframes::render_beat_video(
        &audio_path,
        &image_paths,
        &beats,
        bpm,
        &output_path,
        project_dir.to_str().unwrap(),
    )
}

#[tauri::command]
fn check_dependencies() -> serde_json::Value {
    let ffmpeg = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok();

    let hyperframes = std::process::Command::new("npx")
        .args(["hyperframes", "--version"])
        .output()
        .is_ok();

    serde_json::json!({
        "ffmpeg": ffmpeg,
        "hyperframes": hyperframes,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let export_dir = dirs::document_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("BeatCutExports");

    std::fs::create_dir_all(&export_dir).ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            export_dir: Mutex::new(export_dir.to_str().unwrap_or("./exports").to_string()),
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_beats,
            get_waveform,
            render_video,
            check_dependencies,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
