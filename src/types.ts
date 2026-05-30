export interface BeatInfo {
  bpm: number;
  beats: number[];
  duration: number;
}

export interface WaveformData {
  samples: number[];
  sample_rate: number;
  duration: number;
}

export interface DependencyStatus {
  ffmpeg: boolean;
  hyperframes: boolean;
}

export interface MediaItem {
  path: string;
  name: string;
  type: 'audio' | 'image';
}

export type AppStatus = 'idle' | 'analyzing' | 'ready' | 'rendering' | 'done' | 'error';
