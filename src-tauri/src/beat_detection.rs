use std::fs;
use std::path::Path;
use std::process::Command;

/// Detected beat info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BeatInfo {
    /// BPM detected
    pub bpm: f64,
    /// Beat positions in seconds
    pub beats: Vec<f64>,
    /// Total duration in seconds
    pub duration: f64,
}

/// Waveform data for visualization (downsampled)
#[derive(Debug, Clone, serde::Serialize)]
pub struct WaveformData {
    pub samples: Vec<f64>,
    pub sample_rate: f64,
    pub duration: f64,
}

/// Read audio file and detect beats
pub fn detect_beats(file_path: &str) -> Result<BeatInfo, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Convert to WAV if needed
    let wav_path = convert_to_wav(file_path)?;

    // Read WAV
    let (samples, sample_rate) = read_wav(&wav_path)?;

    // If we converted a temp file, clean up
    if wav_path != file_path {
        let _ = fs::remove_file(&wav_path);
    }

    let duration = samples.len() as f64 / sample_rate as f64;

    // Beat detection
    let beats = detect_beats_from_samples(&samples, sample_rate)?;
    let bpm = if beats.len() > 2 {
        let intervals: Vec<f64> = beats.windows(2).map(|w| w[1] - w[0]).collect();
        let avg_interval: f64 = intervals.iter().sum::<f64>() / intervals.len() as f64;
        if avg_interval > 0.0 {
            60.0 / avg_interval
        } else {
            120.0
        }
    } else {
        120.0
    };

    Ok(BeatInfo { bpm: bpm.round(), beats, duration })
}

/// Get waveform data for visualization
pub fn get_waveform(file_path: &str, max_points: usize) -> Result<WaveformData, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let wav_path = convert_to_wav(file_path)?;
    let (samples, sample_rate) = read_wav(&wav_path)?;

    if wav_path != file_path {
        let _ = fs::remove_file(&wav_path);
    }

    let duration = samples.len() as f64 / sample_rate as f64;

    // Downsample to max_points
    let step = (samples.len() / max_points).max(1);
    let downsampled: Vec<f64> = samples
        .iter()
        .step_by(step)
        .copied()
        .collect();

    Ok(WaveformData {
        samples: downsampled,
        sample_rate,
        duration,
    })
}

/// Convert any audio file to WAV using ffmpeg
fn convert_to_wav(file_path: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "wav" {
        return Ok(file_path.to_string());
    }

    // Create temp file
    let temp_dir = std::env::temp_dir();
    let temp_wav = temp_dir.join(format!("beatcut_{}.wav", std::process::id()));
    let temp_wav_str = temp_wav.to_str().unwrap().to_string();

    let output = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(file_path)
        .args(["-ac", "1"])      // mono
        .args(["-ar", "44100"])  // 44.1kHz
        .args(["-sample_fmt", "s16"]) // 16-bit
        .arg(&temp_wav_str)
        .output()
        .map_err(|e| format!("ffmpeg not found: {}. Install ffmpeg to use this feature.", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg conversion failed: {}", stderr));
    }

    Ok(temp_wav_str)
}

/// Read WAV file into mono f64 samples
fn read_wav(file_path: &str) -> Result<(Vec<f64>, f64), String> {
    let mut reader = hound::WavReader::open(file_path)
        .map_err(|e| format!("Failed to read WAV: {}", e))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f64;
    let channels = spec.channels as usize;

    let samples: Vec<f64> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (2i64.pow(bits as u32 - 1)) as f64;
            reader.samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f64 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => {
            reader.samples::<f32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f64)
                .collect()
        }
    };

    // Convert stereo to mono by averaging channels
    let mono: Vec<f64> = if channels > 1 {
        samples.chunks(channels)
            .map(|chunk| chunk.iter().sum::<f64>() / channels as f64)
            .collect()
    } else {
        samples
    };

    Ok((mono, sample_rate))
}

/// Energy-based beat detection algorithm
fn detect_beats_from_samples(samples: &[f64], sample_rate: f64) -> Result<Vec<f64>, String> {
    if samples.is_empty() {
        return Ok(vec![]);
    }

    let frame_size = 1024;
    let hop_size = 512;
    let total_frames = (samples.len() - frame_size) / hop_size;

    if total_frames < 10 {
        return Ok(vec![]); // Too short to detect
    }

    // 1. Compute energy per frame
    let mut energy: Vec<f64> = Vec::with_capacity(total_frames);
    for i in 0..total_frames {
        let start = i * hop_size;
        let frame: f64 = samples[start..start + frame_size]
            .iter()
            .map(|s| s * s)
            .sum::<f64>() / frame_size as f64;
        energy.push(frame);
    }

    // 2. Normalize energy
    let max_energy = energy.iter().cloned().fold(0.0f64, f64::max);
    if max_energy > 0.0 {
        for e in energy.iter_mut() {
            *e /= max_energy;
        }
    }

    // 3. Compute onset strength (positive derivative only)
    let mut onset_strength: Vec<f64> = Vec::with_capacity(total_frames);
    let window_size = 10usize;
    for i in 0..total_frames {
        let local_avg = if i >= window_size {
            energy[i - window_size..i].iter().sum::<f64>() / window_size as f64
        } else {
            energy[..=i].iter().sum::<f64>() / (i + 1) as f64
        };
        let diff = energy[i] - local_avg;
        onset_strength.push(if diff > 0.0 { diff } else { 0.0 });
    }

    // 4. Smooth onset strength
    let smooth_window = 3usize;
    let mut smoothed: Vec<f64> = onset_strength.clone();
    for i in smooth_window..onset_strength.len() - smooth_window {
        smoothed[i] = onset_strength[i - smooth_window..=i + smooth_window]
            .iter().sum::<f64>() / (2 * smooth_window + 1) as f64;
    }

    // 5. Adaptive threshold
    let mean_onset = smoothed.iter().sum::<f64>() / smoothed.len() as f64;
    let threshold = mean_onset * 2.5;

    // 6. Find peaks above threshold
    let mut raw_peaks: Vec<usize> = Vec::new();
    for i in 2..smoothed.len() - 2 {
        if smoothed[i] > threshold
            && smoothed[i] > smoothed[i - 1]
            && smoothed[i] > smoothed[i - 2]
            && smoothed[i] >= smoothed[i + 1]
            && smoothed[i] >= smoothed[i + 2]
        {
            raw_peaks.push(i);
        }
    }

    // 7. If no peaks found with strict threshold, lower it
    if raw_peaks.len() < 4 {
        let threshold2 = mean_onset * 1.5;
        for i in 2..smoothed.len() - 2 {
            if smoothed[i] > threshold2
                && smoothed[i] > smoothed[i - 1]
                && smoothed[i] >= smoothed[i + 1]
            {
                raw_peaks.push(i);
            }
        }
        // Remove duplicates while preserving order
        raw_peaks.sort();
        raw_peaks.dedup();
    }

    // 8. Convert frames to seconds
    let mut beats: Vec<f64> = raw_peaks
        .iter()
        .map(|&f| f as f64 * hop_size as f64 / sample_rate)
        .collect();

    // 9. If still too few beats, estimate from energy peaks differently
    if beats.len() < 4 && beats.len() > 0 {
        // Just use what we have
    } else if beats.is_empty() {
        // Fallback: evenly spaced beats at 120 BPM
        let bpm = 120.0;
        let interval = 60.0 / bpm;
        let duration = samples.len() as f64 / sample_rate;
        let mut t = interval;
        while t < duration {
            beats.push(t);
            t += interval;
        }
    }

    // 10. Filter out beats too close together (within 100ms)
    beats.dedup_by(|a, b| (*b - *a).abs() < 0.1);

    Ok(beats)
}
