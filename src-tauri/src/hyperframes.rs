use std::path::Path;
use std::process::Command;
use log;

/// Generate a HyperFrames composition HTML and render it to video
#[allow(unused)]
pub fn render_beat_video(
    audio_path: &str,
    image_paths: &[String],
    beats: &[f64],
    bpm: f64,
    output_path: &str,
    project_dir: &str,
) -> Result<String, String> {
    if beats.is_empty() {
        return Err("No beats detected. Cannot render video.".to_string());
    }

    // Prepare media files
    let media_dir = Path::new(project_dir).join("media");
    std::fs::create_dir_all(&media_dir)
        .map_err(|e| format!("Failed to create media dir: {}", e))?;

    // Copy audio file
    let audio_filename = format!("audio{}", get_extension(audio_path));
    let audio_dest = media_dir.join(&audio_filename);
    std::fs::copy(audio_path, &audio_dest)
        .map_err(|e| format!("Failed to copy audio: {}", e))?;

    // Copy image files
    let mut image_filenames: Vec<String> = Vec::new();
    for (i, img_path) in image_paths.iter().enumerate() {
        let ext = get_extension(img_path);
        let filename = format!("img_{:04}.{}", i, ext);
        let dest = media_dir.join(&filename);
        std::fs::copy(img_path, &dest)
            .map_err(|e| format!("Failed to copy image {}: {}", i, e))?;
        image_filenames.push(filename);
    }

    // Generate the composition HTML
    let has_images = !image_filenames.is_empty();
    let html = generate_composition_html(
        &audio_filename,
        &image_filenames,
        beats,
        bpm,
        has_images,
    );

    let html_path = Path::new(project_dir).join("index.html");
    std::fs::write(&html_path, &html)
        .map_err(|e| format!("Failed to write composition HTML: {}", e))?;

    // Write a minimal package.json
    let pkg_json = r#"{"name":"beatcut-export","private":true,"version":"0.1.0"}"#;
    std::fs::write(Path::new(project_dir).join("package.json"), pkg_json)
        .map_err(|e| format!("Failed to write package.json: {}", e))?;

    // Check if hyperframes CLI is available
    let which_output = Command::new("which")
        .arg("hyperframes")
        .output()
        .map_err(|e| format!("Failed to check hyperframes: {}", e))?;

    let hf_cmd = if which_output.status.success() {
        "hyperframes".to_string()
    } else {
        "npx".to_string()
    };

    // Run hyperframes render
    let mut cmd = Command::new(&hf_cmd);
    if hf_cmd == "npx" {
        cmd.arg("hyperframes");
    }
    cmd.arg("render")
        .arg("--no-watch")
        .arg("--output")
        .arg(output_path)
        .current_dir(project_dir);

    log::info!("Running: {:?}", cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run hyperframes render: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "HyperFrames rendering failed.\nstdout: {}\nstderr: {}",
            stdout, stderr
        ));
    }

    Ok(output_path.to_string())
}

fn get_extension(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3")
}

fn generate_composition_html(
    audio_filename: &str,
    image_filenames: &[String],
    beats: &[f64],
    _bpm: f64,
    has_images: bool,
) -> String {
    let duration = beats.last().copied().unwrap_or(10.0) + 2.0;

    let audio_src = format!("media/{}", audio_filename);
    let images_src: Vec<String> = image_filenames
        .iter()
        .map(|f| format!("media/{}", f))
        .collect();

    // If we have fewer images than beats, cycle them
    let image_refs: Vec<&str> = if images_src.is_empty() {
        vec![]
    } else if images_src.len() >= beats.len() {
        images_src.iter().map(|s| s.as_str()).collect()
    } else {
        (0..beats.len())
            .map(|i| images_src[i % images_src.len()].as_str())
            .collect()
    };

    let mut scenes_html = String::new();
    let mut gsap_tweens = String::new();

    // Generate a scene for each beat
    for (i, &beat_time) in beats.iter().enumerate() {
        let scene_id = format!("s{}", i);
        let _next_beat = if i + 1 < beats.len() {
            beats[i + 1]
        } else {
            duration
        };

        // Background image or color
        let img_src = image_refs.get(i).copied().unwrap_or("");
        let bg_style = if !img_src.is_empty() && has_images {
            format!(
                "background-image: url('{}'); background-size: cover; background-position: center;",
                img_src
            )
        } else {
            let colors = [
                "#FF3366", "#6C5CE7", "#00CEC9", "#FD79A8", "#E17055",
                "#0984E3", "#FDCB6E", "#E84393", "#00B894", "#6C5CE7",
            ];
            format!("background-color: {}", colors[i % colors.len()])
        };

        scenes_html.push_str(&format!(
            r#"<div id="{id}" class="scene" style="position:absolute;top:0;left:0;width:100%;height:100%;display:flex;align-items:center;justify-content:center;{bg}">
                <div class="beat-flash" style="position:absolute;top:0;left:0;width:100%;height:100%;background:rgba(255,255,255,0.12);opacity:0;"></div>
            </div>"#,
            id = scene_id,
            bg = bg_style,
        ));

        // Entrance animation - scale + fade in
        let entrance_start = beat_time + 0.05;
        let entrance_dur = 0.3f64.max(0.2);
        gsap_tweens.push_str(&format!(
            "tl.from('#{id}', {{ scale: 1.12, opacity: 0, duration: {dur}, ease: 'power3.out' }}, {start});\n",
            id = scene_id,
            start = entrance_start,
            dur = entrance_dur,
        ));

        // Flash effect at beat
        gsap_tweens.push_str(&format!(
            "tl.to('#{id} .beat-flash', {{ opacity: 1, duration: 0.05, ease: 'none' }}, {start});\n",
            id = scene_id,
            start = beat_time,
        ));
        gsap_tweens.push_str(&format!(
            "tl.to('#{id} .beat-flash', {{ opacity: 0, duration: 0.15, ease: 'power2.out' }}, {fade});\n",
            id = scene_id,
            fade = beat_time + 0.05,
        ));
    }

    // Title overlay (when no images)
    let overlay_html = if !has_images {
        r#"
        <div class="overlay" style="position:absolute;top:0;left:0;width:100%;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;pointer-events:none;">
            <h1 id="title-text" style="color:white;font-size:80px;font-family:'Helvetica Neue',Arial,sans-serif;font-weight:900;letter-spacing:-2px;text-shadow:0 0 40px rgba(0,0,0,0.5);opacity:0;">BEATCUT</h1>
        </div>"#
    } else {
        ""
    };


    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>BeatCut Export</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ background: #000; overflow: hidden; }}
  #main {{ position: relative; width: 1920px; height: 1080px; background: #000; }}
  .scene {{ overflow: hidden; }}
</style>
</head>
<body>
<div id="main" data-composition-id="main" data-width="1920" data-height="1080">

  <!-- Audio -->
  <audio id="audio-track" data-start="0" data-duration="{duration}" data-track-index="1" data-volume="1" src="{audio_src}"></audio>

  <!-- Beat scenes -->
  {scenes}

  {overlay}

  <script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script>
  <script>
    window.__timelines = window.__timelines || {{}};
    const tl = gsap.timeline({{ paused: true }});

    {title_anim}

    {tweens}

    window.__timelines["main"] = tl;
  </script>
</div>
</body>
</html>"#,
        duration = duration,
        audio_src = audio_src,
        scenes = scenes_html,
        overlay = overlay_html,
        title_anim = if !has_images {
            "tl.from('#title-text', { y: 40, opacity: 0, scale: 0.9, duration: 0.8, ease: 'expo.out' }, 0.3);"
        } else {
            ""
        },
        tweens = gsap_tweens,
    )
}
