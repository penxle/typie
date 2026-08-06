import { execFile } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { promisify } from 'node:util';
import ffmpeg from 'fluent-ffmpeg';

const execFileAsync = promisify(execFile);

const FRAMES_BINARY = 'webp-anim-frames';
const MAX_DECODED_BYTES = 4 * 1024 * 1024 * 1024;
const EXTRACT_TIMEOUT_MS = 180_000;
const ENCODE_TIMEOUT_SECONDS = 180;

export function exceedsAnimatedImageBudget(metadata: { width?: number; height?: number; pages?: number }): boolean {
  const { width, height, pages } = metadata;
  if (!width || !height || !pages) {
    return true;
  }
  return width * height * 4 * pages > MAX_DECODED_BYTES;
}

export function buildConcatScript(framePaths: string[], timestampsMs: number[]): string {
  const lines = ['ffconcat version 1.0'];
  let prev = 0;
  for (const [i, framePath] of framePaths.entries()) {
    const centiseconds = Math.max(Math.round((timestampsMs[i] - prev) / 10), 1);
    lines.push(`file '${framePath}'`, 'option framerate 100', `duration ${(centiseconds / 100).toFixed(2)}`);
    prev = timestampsMs[i];
  }
  return `${lines.join('\n')}\n`;
}

export async function convertAnimatedWebpToMp4(inputPath: string, outputPath: string): Promise<void> {
  const tempDir = path.dirname(inputPath);
  const framesDir = path.join(tempDir, 'frames');
  await fs.mkdir(framesDir);

  const { stdout } = await execFileAsync(FRAMES_BINARY, [inputPath, framesDir], {
    timeout: EXTRACT_TIMEOUT_MS,
    maxBuffer: 16 * 1024 * 1024,
  });

  const trimmed = stdout.trim();
  const entries = trimmed ? trimmed.split('\n').map((line) => line.split(' ').map(Number)) : [];
  if (entries.length === 0 || entries.some(([index, timestamp]) => !Number.isSafeInteger(index) || !Number.isSafeInteger(timestamp))) {
    throw new Error(`unexpected webp-anim-frames output: ${trimmed.slice(0, 100)}`);
  }

  const framePaths = entries.map(([index]) => path.join(framesDir, `frame_${String(index).padStart(5, '0')}.png`));
  const timestamps = entries.map(([, timestamp]) => timestamp);

  const concatPath = path.join(tempDir, 'frames.ffconcat');
  await fs.writeFile(concatPath, buildConcatScript(framePaths, timestamps));

  return new Promise((resolve, reject) => {
    ffmpeg(concatPath, { timeout: ENCODE_TIMEOUT_SECONDS })
      .inputOptions(['-f', 'concat', '-safe', '0'])
      .outputOptions([
        '-movflags',
        'faststart',
        '-pix_fmt',
        'yuv420p',
        '-vf',
        'scale=trunc(iw/2)*2:trunc(ih/2)*2',
        '-c:v',
        'libx264',
        '-preset',
        'fast',
        '-crf',
        '23',
        '-fps_mode',
        'vfr',
      ])
      .noAudio()
      .toFormat('mp4')
      .on('end', () => resolve())
      .on('error', (err) => reject(err))
      .save(outputPath);
  });
}
