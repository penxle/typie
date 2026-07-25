import { execFileSync } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { GetObjectCommand, HeadObjectCommand, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';

const s3 = new S3Client({});

const BUCKET = 'typie-misc';
const KEY = 'bmo/codebase/source.tar.zst';
const TARBALL_URL = 'https://codeload.github.com/penxle/typie/tar.gz/refs/heads/main';
const TTL = 30 * 60 * 1000;

export const CODEBASE_DIR = '/tmp/codebase';

const ARCHIVE_PATH = '/tmp/codebase.tar.zst';
const DOWNLOAD_PATH = '/tmp/repo.tar.gz';

const EXCLUDES = [
  '*.ttf',
  '*.otf',
  '*.woff',
  '*.woff2',
  '*.icns',
  '*.ico',
  '*.png',
  '*.jpg',
  '*.jpeg',
  '*.gif',
  '*.webp',
  '*.svg',
  '*.bin',
  '*.mp4',
  '*.mov',
  '*.zip',
  '*.jar',
  '*.keystore',
  '*/apps/api/drizzle/meta/*',
  '*/assets/fonts.json',
  '*/assets/icons.json',
  '*/benches/fixtures/*',
];

const extractArchive = () => {
  rmSync(CODEBASE_DIR, { recursive: true, force: true });
  mkdirSync(CODEBASE_DIR, { recursive: true });
  execFileSync('tar', ['--zstd', '-xf', ARCHIVE_PATH, '-C', '/tmp'], { stdio: 'ignore' });
};

const getCachedAge = async (): Promise<number | null> => {
  try {
    const head = await s3.send(new HeadObjectCommand({ Bucket: BUCKET, Key: KEY }));
    if (!head.LastModified) return null;
    return Date.now() - head.LastModified.getTime();
  } catch {
    return null;
  }
};

const restoreFromCache = async (): Promise<boolean> => {
  try {
    const result = await s3.send(new GetObjectCommand({ Bucket: BUCKET, Key: KEY }));
    const bytes = await result.Body?.transformToByteArray();
    if (!bytes) return false;
    writeFileSync(ARCHIVE_PATH, bytes);
  } catch (err) {
    console.error('[bmo] codebase cache unavailable:', err);
    return false;
  }

  extractArchive();
  return true;
};

const refreshFromGitHub = async (): Promise<void> => {
  const res = await fetch(TARBALL_URL);
  if (!res.ok) {
    throw new Error(`GitHub tarball error ${res.status}`);
  }

  writeFileSync(DOWNLOAD_PATH, Buffer.from(await res.arrayBuffer()));

  rmSync(CODEBASE_DIR, { recursive: true, force: true });
  mkdirSync(CODEBASE_DIR, { recursive: true });
  execFileSync(
    'tar',
    ['-xzf', DOWNLOAD_PATH, '-C', CODEBASE_DIR, '--strip-components=1', ...EXCLUDES.map((pattern) => `--exclude=${pattern}`)],
    { stdio: 'ignore' },
  );
  rmSync(DOWNLOAD_PATH, { force: true });

  try {
    execFileSync('tar', ['--zstd', '-cf', ARCHIVE_PATH, '-C', '/tmp', 'codebase'], { stdio: 'ignore' });
    await s3.send(new PutObjectCommand({ Bucket: BUCKET, Key: KEY, Body: await readFile(ARCHIVE_PATH) }));
  } catch (err) {
    console.error('[bmo] codebase cache write failed:', err);
  }
};

export const prepareCodebase = async (onRefreshStart?: () => void): Promise<void> => {
  const age = await getCachedAge();

  if (age !== null && age < TTL && (await restoreFromCache())) return;

  onRefreshStart?.();

  try {
    await refreshFromGitHub();
  } catch (err) {
    console.error('[bmo] codebase refresh failed, falling back to cache:', err);
    if (!(await restoreFromCache())) throw err;
  }
};
