import { createHash } from 'node:crypto';
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import {
  DeleteObjectCommand,
  GetObjectCommand,
  HeadObjectCommand,
  ListObjectsV2Command,
  PutObjectCommand,
  S3Client,
} from '@aws-sdk/client-s3';

const s3 = new S3Client({});

const BUCKET = 'typie-misc';
const PREFIX = 'bmo/knowledge/';

export const KNOWLEDGE_DIR = '/tmp/knowledge';

export type KnowledgeChange = {
  path: string;
  action: 'created' | 'updated';
  summary: string;
};

export type KnowledgeConflict = {
  path: string;
  action: 'write' | 'delete';
};

export type KnowledgeChanges = {
  written: KnowledgeChange[];
  deleted: string[];
  conflicts: KnowledgeConflict[];
};

type BaselineEntry = {
  hash: string;
  etag: string;
};

const MAX_CONTENTION_RETRIES = 4;

let baseline = new Map<string, BaselineEntry>();

const hash = (content: Buffer) => createHash('sha256').update(content).digest('hex');

const statusOf = (err: unknown) => (err as { $metadata?: { httpStatusCode?: number } })?.$metadata?.httpStatusCode;

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const fetchRemote = async (relativePath: string): Promise<Buffer | null> => {
  try {
    const result = await s3.send(new GetObjectCommand({ Bucket: BUCKET, Key: `${PREFIX}${relativePath}` }));
    const bytes = await result.Body?.transformToByteArray();
    return bytes ? Buffer.from(bytes) : null;
  } catch {
    return null;
  }
};

const exists = async (relativePath: string) => {
  try {
    await s3.send(new HeadObjectCommand({ Bucket: BUCKET, Key: `${PREFIX}${relativePath}` }));
    return true;
  } catch {
    return false;
  }
};

const listLocalFiles = () => {
  return readdirSync(KNOWLEDGE_DIR, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => `${entry.parentPath}/${entry.name}`.slice(KNOWLEDGE_DIR.length + 1));
};

export const downloadKnowledge = async (): Promise<void> => {
  rmSync(KNOWLEDGE_DIR, { recursive: true, force: true });
  mkdirSync(`${KNOWLEDGE_DIR}/learned`, { recursive: true });
  mkdirSync(`${KNOWLEDGE_DIR}/stated`, { recursive: true });
  mkdirSync(`${KNOWLEDGE_DIR}/reports`, { recursive: true });

  baseline = new Map();

  let token: string | undefined;
  do {
    const result = await s3.send(new ListObjectsV2Command({ Bucket: BUCKET, Prefix: PREFIX, ContinuationToken: token }));

    for (const object of result.Contents ?? []) {
      if (!object.Key || object.Key.endsWith('/')) continue;

      const relativePath = object.Key.slice(PREFIX.length);
      const body = await s3.send(new GetObjectCommand({ Bucket: BUCKET, Key: object.Key }));
      const bytes = await body.Body?.transformToByteArray();
      if (!bytes) continue;

      const content = Buffer.from(bytes);
      mkdirSync(path.dirname(`${KNOWLEDGE_DIR}/${relativePath}`), { recursive: true });
      writeFileSync(`${KNOWLEDGE_DIR}/${relativePath}`, content);
      baseline.set(relativePath, { hash: hash(content), etag: body.ETag ?? '' });
    }

    token = result.NextContinuationToken;
  } while (token);
};

const MAX_INLINE_STATED_BYTES = 64 * 1024;

const readFile = (relativePath: string) => readFileSync(`${KNOWLEDGE_DIR}/${relativePath}`, 'utf8');

const SUMMARY_LIMIT = 300;

type Document = {
  meta: Record<string, string>;
  body: string;
};

const parse = (relativePath: string): Document => {
  const content = readFile(relativePath);
  if (!content.startsWith('---\n')) return { meta: {}, body: content };

  const end = content.indexOf('\n---', 4);
  if (end === -1) return { meta: {}, body: content };

  const meta: Record<string, string> = {};
  for (const line of content.slice(4, end).split('\n')) {
    const separator = line.indexOf(':');
    if (separator === -1) continue;

    const key = line.slice(0, separator).trim();
    if (key) meta[key] = line.slice(separator + 1).trim();
  }

  return { meta, body: content.slice(end + 4) };
};

export const describeKnowledge = (relativePath: string) => {
  const { meta, body } = parse(relativePath);

  const summary =
    meta.summary ||
    body
      .split('\n')
      .find((line) => line.trim().length > 0)
      ?.replace(/^#+\s*/, '')
      .trim();

  if (!summary) return '(비어 있음)';

  return summary.length > SUMMARY_LIMIT ? `${summary.slice(0, SUMMARY_LIMIT)}…` : summary;
};

const buildIndex = (title: string, paths: string[]) =>
  [
    title,
    ...paths.map((relativePath) => {
      const { meta } = parse(relativePath);
      const verified = meta.verified_at ? ` (확인: ${meta.verified_at})` : '';
      return `- \`${relativePath}\`${verified} — ${describeKnowledge(relativePath)}`;
    }),
  ].join('\n');

export const buildKnowledgeContext = (): string | null => {
  const paths = listLocalFiles();
  const stated = paths.filter((relativePath) => relativePath.startsWith('stated/'));
  const learned = paths.filter((relativePath) => relativePath.startsWith('learned/'));
  const reports = paths.filter((relativePath) => relativePath.startsWith('reports/'));

  if (paths.length === 0) return null;

  const sections: string[] = [];

  if (stated.length > 0) {
    const total = stated.reduce((sum, relativePath) => sum + Buffer.byteLength(readFile(relativePath)), 0);

    sections.push(
      total <= MAX_INLINE_STATED_BYTES
        ? [
            '## 사용자가 알려준 정의와 규칙 (stated/) — 전문',
            ...stated.map((relativePath) => `### ${relativePath}\n${readFile(relativePath).trim()}`),
          ].join('\n\n')
        : buildIndex('## 사용자가 알려준 정의와 규칙 (stated/) — 양이 많아 목록만. 관련된 것은 반드시 Read로 열어 확인하세요.', stated),
    );
  }

  if (learned.length > 0) {
    sections.push(buildIndex('## 조사해서 알아낸 규칙과 함정 (learned/) — 목록. 관련 있어 보이면 Read로 열어보세요.', learned));
  }

  if (reports.length > 0) {
    sections.push(
      `## 과거 분석 결과 (reports/) — ${reports.length.toLocaleString()}건. 과거 분석을 참조하거나 비교해야 할 때만 Glob으로 목록을 보고 Read하세요. 여기 담긴 수치는 그 시점의 스냅샷이므로 현재 수치로 재사용하지 마세요.`,
    );
  }

  return sections.join('\n\n');
};

const putConditionally = async (relativePath: string, content: Buffer) => {
  const base = baseline.get(relativePath);
  const condition = base ? { IfMatch: base.etag } : { IfNoneMatch: '*' };

  for (let attempt = 0; attempt < MAX_CONTENTION_RETRIES; attempt++) {
    try {
      await s3.send(new PutObjectCommand({ Bucket: BUCKET, Key: `${PREFIX}${relativePath}`, Body: content, ...condition }));
      return true;
    } catch (err) {
      const status = statusOf(err);

      if (status === 409) {
        await delay(200 * 2 ** attempt);
        continue;
      }

      if (status === 412) {
        const remote = await fetchRemote(relativePath);
        return remote !== null && remote.equals(content);
      }

      throw err;
    }
  }

  console.error('[bmo] knowledge write gave up after contention:', relativePath);
  return false;
};

const deleteConditionally = async (relativePath: string) => {
  const base = baseline.get(relativePath);

  for (let attempt = 0; attempt < MAX_CONTENTION_RETRIES; attempt++) {
    try {
      await s3.send(new DeleteObjectCommand({ Bucket: BUCKET, Key: `${PREFIX}${relativePath}`, IfMatch: base?.etag }));
      return true;
    } catch (err) {
      const status = statusOf(err);

      if (status === 409) {
        await delay(200 * 2 ** attempt);
        continue;
      }

      if (status === 412) {
        return !(await exists(relativePath));
      }

      throw err;
    }
  }

  console.error('[bmo] knowledge delete gave up after contention:', relativePath);
  return false;
};

export const uploadKnowledge = async (): Promise<KnowledgeChanges> => {
  const written: KnowledgeChange[] = [];
  const deleted: string[] = [];
  const conflicts: KnowledgeConflict[] = [];
  const seen = new Set<string>();

  for (const relativePath of listLocalFiles()) {
    seen.add(relativePath);

    const content = readFileSync(`${KNOWLEDGE_DIR}/${relativePath}`);
    if (baseline.get(relativePath)?.hash === hash(content)) continue;

    if (await putConditionally(relativePath, content)) {
      written.push({
        path: relativePath,
        action: baseline.has(relativePath) ? 'updated' : 'created',
        summary: describeKnowledge(relativePath),
      });
    } else {
      conflicts.push({ path: relativePath, action: 'write' });
    }
  }

  for (const relativePath of baseline.keys()) {
    if (seen.has(relativePath)) continue;

    if (await deleteConditionally(relativePath)) {
      deleted.push(relativePath);
    } else {
      conflicts.push({ path: relativePath, action: 'delete' });
    }
  }

  return { written, deleted, conflicts };
};
