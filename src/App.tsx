import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import Waveform from './components/Waveform';
import type { BeatInfo, WaveformData, MediaItem, AppStatus } from './types';

function App() {
  const [status, setStatus] = useState<AppStatus>('idle');
  const [musicFile, setMusicFile] = useState<MediaItem | null>(null);
  const [images, setImages] = useState<MediaItem[]>([]);
  const [beatInfo, setBeatInfo] = useState<BeatInfo | null>(null);
  const [waveformData, setWaveformData] = useState<WaveformData | null>(null);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);

  const showToast = useCallback((message: string, type: 'success' | 'error') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  }, []);

  const importMusic = useCallback(async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{
          name: 'Audio',
          extensions: ['mp3', 'wav', 'm4a', 'aac', 'flac', 'ogg'],
        }],
      });
      if (!file) return;

      const name = file.split('/').pop() || file.split('\\').pop() || file;
      setMusicFile({ path: file, name, type: 'audio' });
      setExportPath(null);
      setBeatInfo(null);
      setWaveformData(null);
      setStatus('analyzing');

      // Check dependencies
      const deps: any = await invoke('check_dependencies');
      if (!deps.ffmpeg) {
        showToast('ffmpeg not found. Please install it with: brew install ffmpeg', 'error');
        setStatus('idle');
        return;
      }

      // Get waveform data
      const waveform: WaveformData = await invoke('get_waveform', {
        filePath: file,
        maxPoints: 2000,
      });
      setWaveformData(waveform);

      // Detect beats
      const beats: BeatInfo = await invoke('detect_beats', { filePath: file });
      setBeatInfo(beats);
      setStatus('ready');
      showToast(`Detected ${beats.bpm} BPM, ${beats.beats.length} beats`, 'success');
    } catch (err: any) {
      showToast(`Error: ${err}`, 'error');
      setStatus('idle');
    }
  }, [showToast]);

  const importImages = useCallback(async () => {
    try {
      const files = await open({
        multiple: true,
        filters: [{
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'],
        }],
      });
      if (!files || !files.length) return;

      const items: MediaItem[] = files.map((f: string) => ({
        path: f,
        name: f.split('/').pop() || f.split('\\').pop() || f,
        type: 'image' as const,
      }));
      setImages(prev => [...prev, ...items]);
    } catch (err: any) {
      showToast(`Error importing images: ${err}`, 'error');
    }
  }, [showToast]);

  const removeImage = useCallback((index: number) => {
    setImages(prev => prev.filter((_, i) => i !== index));
  }, []);

  const exportVideo = useCallback(async () => {
    if (!musicFile || !beatInfo) return;

    setStatus('rendering');
    setProgress(10);

    try {
      const imagePaths = images.map(i => i.path);
      
      // Simulate progress as render runs
      const progressInterval = setInterval(() => {
        setProgress(p => Math.min(p + 5, 85));
      }, 2000);

      const result: string = await invoke('render_video', {
        audioPath: musicFile.path,
        imagePaths,
        beats: beatInfo.beats,
        bpm: beatInfo.bpm,
      });

      clearInterval(progressInterval);
      setProgress(100);
      setExportPath(result);
      setStatus('done');
      showToast(`Video exported!`, 'success');
    } catch (err: any) {
      showToast(`Export failed: ${err}`, 'error');
      setStatus('ready');
      setProgress(0);
    }
  }, [musicFile, beatInfo, images, showToast]);

  const clearAll = useCallback(() => {
    setMusicFile(null);
    setImages([]);
    setBeatInfo(null);
    setWaveformData(null);
    setExportPath(null);
    setStatus('idle');
    setProgress(0);
  }, []);

  return (
    <div className="app-container">
      {/* Title Bar */}
      <div className="titlebar">
        <div className="titlebar-left">
          <span className="titlebar-logo">BEATCUT</span>
          <span className="titlebar-version">v0.1.0</span>
        </div>
        <div className="titlebar-right">
          <span className={`status-dot ${status}`} />
          <span className="status-label">{status === 'idle' ? 'Ready' : status}</span>
        </div>
      </div>

      {/* Main Content */}
      <div className="main-content">
        {/* Side Panel */}
        <div className="side-panel">
          <div className="panel-section">
            <div className="panel-title">Music</div>
            {musicFile ? (
              <div className="media-list">
                <div className="media-item">
                  <div className="media-icon audio">♪</div>
                  <span className="media-name">{musicFile.name}</span>
                  <button className="remove-btn" onClick={() => { setMusicFile(null); setBeatInfo(null); setWaveformData(null); setStatus('idle'); }}>×</button>
                </div>
              </div>
            ) : (
              <button className="import-btn" onClick={importMusic}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M9 18V5l12-2v13" />
                  <circle cx="6" cy="18" r="3" />
                  <circle cx="18" cy="16" r="3" />
                </svg>
                Import Music
              </button>
            )}
          </div>

          <div className="panel-section">
            <div className="panel-title">Images</div>
            <button className="import-btn" onClick={importImages}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <circle cx="8.5" cy="8.5" r="1.5" />
                <polyline points="21,15 16,10 5,21" />
              </svg>
              Add Images
            </button>
            {images.length > 0 ? (
              <div className="media-list">
                {images.map((img, i) => (
                  <div className="media-item" key={i}>
                    <div className="media-icon image">🖼</div>
                    <span className="media-name">{img.name}</span>
                    <button className="remove-btn" onClick={() => removeImage(i)}>×</button>
                  </div>
                ))}
              </div>
            ) : (
              <div className="empty-state">
                Add images to create a<br />beat-synced slideshow
              </div>
            )}
          </div>

          {musicFile && (
            <div className="panel-section">
              <button className="import-btn" onClick={clearAll} style={{ borderColor: 'transparent', justifyContent: 'center' }}>
                Clear Project
              </button>
            </div>
          )}
        </div>

        {/* Center Panel */}
        <div className="center-panel">
          {waveformData && beatInfo ? (
            <>
              <Waveform
                samples={waveformData.samples}
                beats={beatInfo.beats}
                duration={beatInfo.duration}
              />
              <div className="waveform-info">
                <div className="stat">
                  <span className="stat-label">BPM</span>
                  <span className="stat-value">{beatInfo.bpm}</span>
                </div>
                <div className="stat">
                  <span className="stat-label">Beats</span>
                  <span className="stat-value">{beatInfo.beats.length}</span>
                </div>
                <div className="stat">
                  <span className="stat-label">Duration</span>
                  <span className="stat-value">{formatDuration(beatInfo.duration)}</span>
                </div>
                <div className="stat">
                  <span className="stat-label">Images</span>
                  <span className="stat-value">{images.length || 0}</span>
                </div>
              </div>
            </>
          ) : (
            <div className="waveform-container">
              <div className="waveform-empty">
                <h2>BeatCut</h2>
                <p>Import a music file to detect beats, then add images for a beat-synced video.</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Bottom Bar */}
      <div className="bottom-bar">
        <div className="bottom-bar-left">
          {(status === 'rendering' || status === 'done') && (
            <div className="progress-container">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${progress}%` }} />
              </div>
              <div className="progress-text">
                {status === 'rendering' ? 'Rendering...' : 'Done!'} {progress}%
              </div>
            </div>
          )}
          {exportPath && (
            <span style={{ fontSize: 12, color: 'var(--green)', marginLeft: 8 }}>
              ✓ {exportPath.split('/').pop()}
            </span>
          )}
        </div>
        <div className="bottom-bar-right">
          <button
            className="btn btn-primary"
            disabled={!beatInfo || status === 'rendering' || status === 'analyzing'}
            onClick={exportVideo}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            {status === 'rendering' ? 'Rendering...' : 'Export MP4'}
          </button>
        </div>
      </div>

      {/* Toast */}
      {toast && (
        <div className={`toast ${toast.type}`}>
          {toast.message}
        </div>
      )}
    </div>
  );
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

export default App;
