import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { GetObjectCommand, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';

const s3 = new S3Client({});
const BUCKET = 'typie-misc';
const PREFIX = 'bmo/sessions';

const removeSpecialFiles = (dir: string) => {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = `${dir}/${entry.name}`;
    if (entry.isDirectory()) {
      removeSpecialFiles(fullPath);
    } else if (entry.isSocket() || entry.isFIFO()) {
      unlinkSync(fullPath);
    }
  }
};

export const downloadSession = async (sessionId: string): Promise<boolean> => {
  const archivePath = `/tmp/${sessionId}.tar.zst`;

  try {
    const result = await s3.send(
      new GetObjectCommand({
        Bucket: BUCKET,
        Key: `${PREFIX}/${sessionId}.tar.zst`,
      }),
    );

    const bytes = await result.Body?.transformToByteArray();
    if (!bytes) return false;
    writeFileSync(archivePath, bytes);
  } catch (err: unknown) {
    if (err instanceof Error && err.name === 'NoSuchKey') {
      return false;
    }
    throw err;
  }

  execFileSync('tar', ['--zstd', '-xf', archivePath, '-C', '/tmp'], { stdio: 'ignore' });
  return true;
};

export const uploadSession = async (sessionId: string): Promise<void> => {
  const archivePath = `/tmp/${sessionId}.tar.zst`;
  removeSpecialFiles('/tmp/.claude');
  execFileSync('tar', ['--zstd', '-cf', archivePath, '-C', '/tmp', '.claude'], { stdio: 'ignore' });

  await s3.send(
    new PutObjectCommand({
      Bucket: BUCKET,
      Key: `${PREFIX}/${sessionId}.tar.zst`,
      Body: readFileSync(archivePath),
    }),
  );
};
