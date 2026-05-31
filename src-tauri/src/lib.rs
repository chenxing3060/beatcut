mod beat_detection;
mod hyperframes;
mod video_processor;

use std::path::Path;
use std::sync::Mutex;
use tauri::State;
use log;

/// App state
struct AppState {
    export_dir: Mutex<String>,
}

/// Find yt-dlp executable across common locations
fn find_ytdlp() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates: Vec<String> = vec![
        "yt-dlp".to_string(),
        "/opt/homebrew/bin/yt-dlp".to_string(),
        "/usr/local/bin/yt-dlp".to_string(),
        format!("{}/.local/bin/yt-dlp", home),
        format!("{}/Library/Python/3.9/bin/yt-dlp", home),
        format!("{}/Library/Python/3.11/bin/yt-dlp", home),
        format!("{}/Library/Python/3.12/bin/yt-dlp", home),
        format!("{}/Library/Python/3.13/bin/yt-dlp", home),
    ];
    for cmd in &candidates {
        if std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return cmd.to_string();
        }
    }
    "yt-dlp".to_string()
}

#[tauri::command]
fn detect_beats(file_path: String) -> Result<beat_detection::BeatInfo, String> {
    beat_detection::detect_beats(&file_path)
}

#[tauri::command]
fn get_waveform(file_path: String, max_points: usize) -> Result<beat_detection::WaveformData, String> {
    beat_detection::get_waveform(&file_path, max_points)
}

/// Download audio from URL using yt-dlp
#[tauri::command]
fn download_audio(url: String, state: State<AppState>) -> Result<String, String> {
    let ytdlp = find_ytdlp();
    let has_ytdlp = std::process::Command::new(&ytdlp)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_ytdlp {
        return Err(
            "yt-dlp 未安装。请运行:\n  brew install yt-dlp\n或:\n  pip3 install yt-dlp".to_string()
        );
    }

    let export_dir = state.export_dir.lock().map_err(|e| e.to_string())?;
    let download_dir = Path::new(&*export_dir).join("downloads");
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("创建下载目录失败: {}", e))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let output_template = download_dir.join(format!("music_{}.%(ext)s", timestamp));

    log::info!("Downloading: {} -> {:?}", url, output_template);

    let output = std::process::Command::new(&ytdlp)
        .arg("-x")                          // extract audio
        .arg("--audio-format").arg("mp3")   // convert to mp3
        .arg("--audio-quality").arg("0")    // best quality
        .arg("-o").arg(output_template.to_str().unwrap())
        .arg("--print").arg("filename")     // print final filename
        .arg("--no-warnings")
        .arg(&url)
        .output()
        .map_err(|e| format!("yt-dlp 执行失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("下载失败: {}", if stderr.is_empty() { &stdout } else { &stderr }));
    }

    let actual_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    if actual_path.is_empty() || !Path::new(&actual_path).exists() {
        // Fallback: try to find the file by pattern
        let pattern = format!("music_{}.mp3", timestamp);
        let fallback = download_dir.join(&pattern);
        if fallback.exists() {
            return Ok(fallback.to_str().unwrap().to_string());
        }
        return Err("下载完成但未找到输出文件".to_string());
    }

    log::info!("Downloaded: {}", actual_path);
    Ok(actual_path)
}

/// Render image-based beat-sync video (HyperFrames)
#[tauri::command]
fn render_image_beat_video(
    audio_path: String,
    image_paths: Vec<String>,
    beats: Vec<f64>,
    bpm: f64,
    state: State<AppState>,
) -> Result<String, String> {
    let export_dir = state.export_dir.lock().map_err(|e| e.to_string())?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let project_dir = Path::new(&*export_dir).join(format!("render_{}", timestamp));
    let output_path = Path::new(&*export_dir)
        .join(format!("beatcut_image_export_{}.mp4", timestamp))
        .to_str()
        .unwrap()
        .to_string();

    hyperframes::render_beat_video(
        &audio_path, &image_paths, &beats, bpm,
        &output_path, project_dir.to_str().unwrap(),
    )
}

/// Render video-based beat-sync video (ffmpeg)
#[tauri::command]
fn render_video_beat_video(
    audio_path: String,
    video_paths: Vec<String>,
    beats: Vec<f64>,
    state: State<AppState>,
) -> Result<String, String> {
    let export_dir = state.export_dir.lock().map_err(|e| e.to_string())?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let output_path = Path::new(&*export_dir)
        .join(format!("beatcut_video_export_{}.mp4", timestamp))
        .to_str()
        .unwrap()
        .to_string();

    video_processor::render_beat_video(&audio_path, &video_paths, &beats, &output_path)
}

#[tauri::command]
fn check_dependencies() -> serde_json::Value {
    let ffmpeg = std::process::Command::new("ffmpeg")
        .arg("-version").output().is_ok();
    let ffprobe = std::process::Command::new("ffprobe")
        .arg("-version").output().is_ok();
    let hyperframes = std::process::Command::new("npx")
        .args(["hyperframes", "--version"]).output().is_ok();
    let ytdlp = std::process::Command::new(&find_ytdlp())
        .arg("--version").output().is_ok();

    serde_json::json!({
        "ffmpeg": ffmpeg,
        "ffprobe": ffprobe,
        "hyperframes": hyperframes,
        "ytdlp": ytdlp,
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
            download_audio,
            render_image_beat_video,
            render_video_beat_video,
            check_dependencies,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
