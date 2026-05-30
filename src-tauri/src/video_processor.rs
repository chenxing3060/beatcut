use std::path::PathBuf;
use std::process::Command;
use std::fs;

/// Render a beat-synced video from video clips using ffmpeg
pub fn render_beat_video(
    audio_path: &str,
    video_paths: &[String],
    beats: &[f64],
    output_path: &str,
) -> Result<String, String> {
    if beats.is_empty() {
        return Err("未检测到节拍，无法渲染视频".to_string());
    }
    if video_paths.is_empty() {
        return Err("没有导入视频素材".to_string());
    }

    let temp_dir = std::env::temp_dir().join(format!("beatcut_vid_{}", std::process::id()));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    // 1. Get durations of all input videos
    let mut video_durations: Vec<f64> = Vec::new();
    for v in video_paths {
        let dur = get_media_duration(v)?;
        if dur <= 0.0 {
            return Err(format!("无法获取视频时长: {}", v));
        }
        video_durations.push(dur);
    }

    // 2. Calculate how many beats to assign to each video
    let total_avail: f64 = video_durations.iter().sum();
    let num_beats = beats.len();
    let mut temp_segments: Vec<PathBuf> = Vec::new();

    // 3. For each beat, extract a segment from the appropriate video
    for i in 0..num_beats {
        // Segment duration = time to next beat (or last interval for final)
        let seg_dur = if i + 1 < num_beats {
            (beats[i + 1] - beats[i]).max(0.2)
        } else if i > 0 {
            (beats[i] - beats[i - 1]).max(0.2)
        } else {
            0.5
        };

        // Distribute source position proportionally across total video duration
        let progress = if num_beats > 1 {
            i as f64 / (num_beats - 1) as f64
        } else {
            0.0
        };
        let source_time = progress * total_avail;

        // Find which video and at what offset
        let (vid_idx, offset) = locate_in_videos(source_time, &video_durations);

        let seg_path = temp_dir.join(format!("seg_{:04}.mp4", i));
        let seg_str = seg_path.to_str().unwrap();

        log::info!(
            "Extracting beat {}: video[{}] @ {:.2}s, duration {:.2}s",
            i, vid_idx, offset, seg_dur
        );

        extract_segment(&video_paths[vid_idx], offset, seg_dur, seg_str)?;
        temp_segments.push(seg_path);
    }

    // 4. Create concat file and concatenate segments
    let concat_list = temp_dir.join("concat.txt");
    let mut list_content = String::new();
    for seg in &temp_segments {
        list_content.push_str(&format!("file '{}'\n", seg.to_string_lossy()));
    }
    fs::write(&concat_list, &list_content)
        .map_err(|e| format!("写入拼接列表失败: {}", e))?;

    let concat_video = temp_dir.join("concat.mp4");
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f").arg("concat")
        .arg("-safe").arg("0")
        .arg("-i").arg(concat_list.to_str().unwrap())
        .arg("-c:v").arg("libx264")
        .arg("-pix_fmt").arg("yuv420p")
        .arg("-preset").arg("fast")
        .arg("-c:a").arg("aac")
        .arg("-b:a").arg("128k")
        .arg("-movflags").arg("+faststart")
        .arg(concat_video.to_str().unwrap())
        .output()
        .map_err(|e| format!("ffmpeg 拼接失败: {}", e))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(format!("ffmpeg 拼接失败: {}", stderr));
        }

    // 5. Overlay the music audio
    let status2 = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i").arg(concat_video.to_str().unwrap())
        .arg("-i").arg(audio_path)
        .arg("-c:v").arg("libx264")
        .arg("-pix_fmt").arg("yuv420p")
        .arg("-preset").arg("fast")
        .arg("-c:a").arg("aac")
        .arg("-b:a").arg("192k")
        .arg("-map").arg("0:v:0")
        .arg("-map").arg("1:a:0")
        .arg("-shortest")
        .arg("-movflags").arg("+faststart")
        .arg(output_path)
        .output()
        .map_err(|e| format!("ffmpeg 音频合成失败: {}", e))?;

    if !status2.status.success() {
        let stderr = String::from_utf8_lossy(&status2.stderr);
        return Err(format!("ffmpeg 音频合成失败: {}", stderr));
    }

    // Clean up temp files
    fs::remove_dir_all(&temp_dir).ok();

    log::info!("Video exported: {}", output_path);
    Ok(output_path.to_string())
}

/// Get the duration of a media file in seconds using ffprobe
fn get_media_duration(path: &str) -> Result<f64, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            path,
        ])
        .output()
        .map_err(|e| format!("ffprobe 未找到: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe 失败: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    trimmed
        .parse::<f64>()
        .map_err(|e| format!("解析时长失败 '{}': {}", trimmed, e))
}

/// Extract a segment from a video file
fn extract_segment(input: &str, start: f64, duration: f64, output: &str) -> Result<(), String> {
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss").arg(format!("{:.3}", start))
        .arg("-i").arg(input)
        .arg("-t").arg(format!("{:.3}", duration))
        .arg("-c:v").arg("libx264")
        .arg("-preset").arg("fast")
        .arg("-pix_fmt").arg("yuv420p")
        .arg("-c:a").arg("aac")
        .arg("-b:a").arg("128k")
        .arg("-movflags").arg("+faststart")
        .arg(output)
        .output()
        .map_err(|e| format!("ffmpeg 未找到: {}", e))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(format!("提取视频片段失败 ({}s @ {}s): {}", start, duration, stderr));
    }

    Ok(())
}

/// Given a time position and list of durations, find which video index and offset within it
fn locate_in_videos(position: f64, durations: &[f64]) -> (usize, f64) {
    let mut accum = 0.0;
    for (i, &dur) in durations.iter().enumerate() {
        if position < accum + dur {
            let offset = position - accum;
            return (i, offset.max(0.0).min(dur - 0.1));
        }
        accum += dur;
    }
    // Past all videos - return the last one near its end
    let last_idx = durations.len() - 1;
    (last_idx, durations[last_idx].max(0.5) - 0.5)
}
