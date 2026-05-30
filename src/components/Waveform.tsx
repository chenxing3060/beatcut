import { useRef, useEffect, useCallback } from 'react';

interface WaveformProps {
  samples: number[];
  beats: number[];
  duration: number;
  currentTime?: number;
}

export default function Waveform({ samples, beats, duration, currentTime = 0 }: WaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;

    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.scale(dpr, dpr);

    // Clear
    ctx.fillStyle = '#151515';
    ctx.fillRect(0, 0, width, height);

    if (!samples.length) {
      ctx.fillStyle = '#2a2a2e';
      ctx.font = '14px -apple-system, BlinkMacSystemFont, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('导入音乐后将显示波形', width / 2, height / 2);
      return;
    }

    const centerY = height / 2;
    const waveHeight = height * 0.7;

    // Draw waveform
    ctx.beginPath();
    ctx.strokeStyle = '#3a3a3c';
    ctx.lineWidth = 2;

    for (let x = 0; x < width; x++) {
      const idx = Math.min(Math.floor((x / width) * samples.length), samples.length - 1);
      const val = samples[idx] * waveHeight;
      ctx.lineTo(x, centerY - val);
    }
    ctx.stroke();

    // Mirror
    ctx.beginPath();
    for (let x = 0; x < width; x++) {
      const idx = Math.min(Math.floor((x / width) * samples.length), samples.length - 1);
      const val = samples[idx] * waveHeight;
      ctx.lineTo(x, centerY + val);
    }
    ctx.stroke();

    // Draw beat markers
    if (beats.length > 0) {
      const startX = 0;
      const endX = width;

      for (const beat of beats) {
        const x = startX + (beat / duration) * (endX - startX);
        
        // Beat line
        ctx.beginPath();
        ctx.strokeStyle = 'rgba(255, 68, 88, 0.7)';
        ctx.lineWidth = 1.5;
        ctx.setLineDash([4, 4]);
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
        ctx.setLineDash([]);

        // Beat dot
        ctx.beginPath();
        ctx.arc(x, centerY, 4, 0, Math.PI * 2);
        ctx.fillStyle = '#ff4458';
        ctx.fill();
      }
    }

    // Draw playhead
    if (currentTime > 0 && duration > 0) {
      const x = (currentTime / duration) * width;
      ctx.beginPath();
      ctx.strokeStyle = '#ffd60a';
      ctx.lineWidth = 2;
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    // Beat count label
    ctx.fillStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.font = '600 48px -apple-system, BlinkMacSystemFont, sans-serif';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'bottom';
    ctx.fillText(`${beats.length}`, width - 16, height - 8);
    ctx.font = '14px -apple-system, BlinkMacSystemFont, sans-serif';
    ctx.fillStyle = 'rgba(255, 255, 255, 0.06)';
    ctx.fillText('BEATS', width - 16, height - 56);
  }, [samples, beats, duration, currentTime]);

  useEffect(() => {
    draw();
  }, [draw]);

  useEffect(() => {
    const handleResize = () => draw();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [draw]);

  return (
    <div ref={containerRef} className="waveform-container">
      <canvas ref={canvasRef} />
    </div>
  );
}
