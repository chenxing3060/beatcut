import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import Waveform from './components/Waveform';
import type { BeatInfo, WaveformData, AppStatus } from './types';

interface MediaItem {
  path: string;
  name: string;
  type: 'audio' | 'image' | 'video';
}

function App() {
  const [status, setStatus] = useState<AppStatus>('idle');
  const [musicFile, setMusicFile] = useState<MediaItem | null>(null);
  const [mediaItems, setMediaItems] = useState<MediaItem[]>([]);
  const [beatInfo, setBeatInfo] = useState<BeatInfo | null>(null);
  const [waveformData, setWaveformData] = useState<WaveformData | null>(null);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);

  const hasVideos = mediaItems.some(m => m.type === 'video');
const exportMode: 'video' | 'image' = hasVideos ? 'video' : 'image';

  const showToast = useCallback((message: string, type: 'success' | 'error') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  }, []);

  const importMusic = useCallback(async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{
          name: '音频文件',
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

      const deps: any = await invoke('check_dependencies');
      if (!deps.ffmpeg) {
        showToast('未找到 ffmpeg，请先安装：brew install ffmpeg', 'error');
        setStatus('idle');
        return;
      }
      if (exportMode === 'image' && !deps.hyperframes) {
        showToast('未找到 hyperframes，图片卡点模式可能不可用', 'error');
      }

      const waveform: WaveformData = await invoke('get_waveform', {
        filePath: file,
        maxPoints: 2000,
      });
      setWaveformData(waveform);

      const beats: BeatInfo = await invoke('detect_beats', { filePath: file });
      setBeatInfo(beats);
      setStatus('ready');
      showToast(`检测完成：${beats.bpm} BPM，${beats.beats.length} 个节拍`, 'success');
    } catch (err: any) {
      showToast(`分析失败: ${err}`, 'error');
      setStatus('idle');
    }
  }, [showToast, exportMode]);

  const importImages = useCallback(async () => {
    try {
      const files = await open({
        multiple: true,
        filters: [{
          name: '图片',
          extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'],
        }],
      });
      if (!files || !files.length) return;

      const items: MediaItem[] = files.map((f: string) => ({
        path: f,
        name: f.split('/').pop() || f.split('\\').pop() || f,
        type: 'image' as const,
      }));
      setMediaItems(prev => [...prev, ...items]);
    } catch (err: any) {
      showToast(`导入图片失败: ${err}`, 'error');
    }
  }, [showToast]);

  const importVideos = useCallback(async () => {
    try {
      const files = await open({
        multiple: true,
        filters: [{
          name: '视频',
          extensions: ['mp4', 'mov', 'avi', 'mkv', 'webm', 'm4v'],
        }],
      });
      if (!files || !files.length) return;

      const items: MediaItem[] = files.map((f: string) => ({
        path: f,
        name: f.split('/').pop() || f.split('\\').pop() || f,
        type: 'video' as const,
      }));
      setMediaItems(prev => [...prev, ...items]);
    } catch (err: any) {
      showToast(`导入视频失败: ${err}`, 'error');
    }
  }, [showToast]);

  const removeMedia = useCallback((index: number) => {
    setMediaItems(prev => prev.filter((_, i) => i !== index));
  }, []);

  const exportVideo = useCallback(async () => {
    if (!musicFile || !beatInfo) return;

    setStatus('rendering');
    setProgress(10);
    setExportPath(null);

    try {
      const progressInterval = setInterval(() => {
        setProgress(p => Math.min(p + 8, 90));
      }, 2000);

      let result: string;

      if (exportMode === 'video') {
        const videoPaths = mediaItems.filter(m => m.type === 'video').map(m => m.path);
        result = await invoke('render_video_beat_video', {
          audioPath: musicFile.path,
          videoPaths,
          beats: beatInfo.beats,
        });
      } else {
        const imagePaths = mediaItems.filter(m => m.type === 'image').map(m => m.path);
        result = await invoke('render_image_beat_video', {
          audioPath: musicFile.path,
          imagePaths,
          beats: beatInfo.beats,
          bpm: beatInfo.bpm,
        });
      }

      clearInterval(progressInterval);
      setProgress(100);
      setExportPath(result);
      setStatus('done');
      showToast('导出成功！', 'success');
    } catch (err: any) {
      showToast(`导出失败: ${err}`, 'error');
      setStatus('ready');
      setProgress(0);
    }
  }, [musicFile, beatInfo, mediaItems, exportMode, showToast]);

  const clearAll = useCallback(() => {
    setMusicFile(null);
    setMediaItems([]);
    setBeatInfo(null);
    setWaveformData(null);
    setExportPath(null);
    setStatus('idle');
    setProgress(0);
  }, []);

  const statusLabel = () => {
    switch (status) {
      case 'idle': return '就绪';
      case 'analyzing': return '分析中';
      case 'ready': return '准备就绪';
      case 'rendering': return '导出中';
      case 'done': return '完成';
      case 'error': return '错误';
    }
  };

  const mediaIcon = (type: string) => {
    switch (type) {
      case 'audio': return <div className="media-icon audio">♪</div>;
      case 'image': return <div className="media-icon image">🖼</div>;
      case 'video': return <div className="media-icon video">▶</div>;
    }
  };

  return (
    <div className="app-container">
      {/* 标题栏 */}
      <div className="titlebar">
        <div className="titlebar-left">
          <span className="titlebar-logo">BEATCUT</span>
          <span className="titlebar-version">v0.1.0</span>
        </div>
        <div className="titlebar-right">
          <span className={`status-dot ${status}`} />
          <span className="status-label">{statusLabel()}</span>
        </div>
      </div>

      {/* 主内容 */}
      <div className="main-content">
        {/* 左侧面板 */}
        <div className="side-panel">
          <div className="panel-section">
            <div className="panel-title">音乐</div>
            {musicFile ? (
              <div className="media-list">
                <div className="media-item">
                  {mediaIcon('audio')}
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
                导入音乐
              </button>
            )}
          </div>

          <div className="panel-section">
            <div className="panel-title">素材</div>
            <button className="import-btn" onClick={importImages}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <circle cx="8.5" cy="8.5" r="1.5" />
                <polyline points="21,15 16,10 5,21" />
              </svg>
              添加图片
            </button>
            <button className="import-btn" onClick={importVideos}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polygon points="23 7 16 12 23 17 23 7" />
                <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
              </svg>
              添加视频
            </button>
            {mediaItems.length > 0 ? (
              <div className="media-list">
                {mediaItems.map((item, i) => (
                  <div className="media-item" key={i}>
                    {mediaIcon(item.type)}
                    <span className="media-name">{item.name}</span>
                    <button className="remove-btn" onClick={() => removeMedia(i)}>×</button>
                  </div>
                ))}
              </div>
            ) : (
              <div className="empty-state">
                添加图片或视频素材，<br />生成节拍卡点视频
              </div>
            )}
          </div>

          {musicFile && (
            <div className="panel-section">
              <button className="import-btn" onClick={clearAll} style={{ borderColor: 'transparent', justifyContent: 'center' }}>
                清空项目
              </button>
            </div>
          )}
        </div>

        {/* 中间波形面板 */}
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
                  <span className="stat-label">节拍</span>
                  <span className="stat-value">{beatInfo.beats.length}</span>
                </div>
                <div className="stat">
                  <span className="stat-label">时长</span>
                  <span className="stat-value">{formatDuration(beatInfo.duration)}</span>
                </div>
                <div className="stat">
                  <span className="stat-label">素材</span>
                  <span className="stat-value">{mediaItems.length || 0}</span>
                </div>
              </div>
            </>
          ) : (
            <div className="waveform-container">
              <div className="waveform-empty">
                <h2>BeatCut</h2>
                <p>导入音乐自动检测节拍，添加图片或视频生成卡点视频</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* 底部导出栏 */}
      <div className="bottom-bar">
        <div className="bottom-bar-left">
          {(status === 'rendering' || status === 'done') && (
            <div className="progress-container">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${progress}%` }} />
              </div>
              <div className="progress-text">
                {status === 'rendering' ? '正在导出...' : '导出完成！'} {progress}%
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
            disabled={!beatInfo || status === 'rendering' || status === 'analyzing' || mediaItems.length === 0}
            onClick={exportVideo}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            {status === 'rendering'
              ? '导出中...'
              : hasVideos
                ? '导出视频卡点'
                : '导出图片卡点'
            }
          </button>
        </div>
      </div>

      {/* Toast 提示 */}
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
