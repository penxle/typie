import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { Redis } from 'ioredis';
import {
  abandonBlobLease,
  blobLeaseKey,
  claimBlobLeaseForFinalize,
  createBlobLeases,
  deleteBlobLeases,
  deps,
  readBlobLeases,
  releaseFinalizeClaim,
} from './blob-lease.ts';
import type { TestContext } from 'node:test';
import type { BlobLease } from './blob-lease.ts';

// 실 ioredis 대상 최소 통합 테스트. fake로는 검증할 수 없는 KEYS/ARGV 인덱싱·PX 단위·실제 NX/Lua
// 원자성을 확인한다. 로컬/CI에 Redis가 없으면(연결 실패) 전부 스킵 — 기본 test 커맨드는 항상 green.
const REDIS_URL = process.env.BLOB_LEASE_TEST_REDIS_URL ?? '127.0.0.1:6379';
const [host, port] = REDIS_URL.split(':');

const client = new Redis({ host, port: Number(port), lazyConnect: true, connectTimeout: 300, maxRetriesPerRequest: 0 });
// eslint-disable-next-line @typescript-eslint/no-empty-function -- connection errors are handled via the connect() try/catch in before()
client.on('error', () => {});

let available = false;

before(async () => {
  try {
    await client.connect();
    await client.ping();
    available = true;
  } catch {
    available = false;
  }
});

after(async () => {
  if (available) {
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- swallow quit error, client is being torn down
    await client.quit().catch(() => {});
  } else {
    client.disconnect();
  }
});

const skipIfUnavailable = (t: TestContext): boolean => {
  if (!available) {
    t.skip('no reachable redis at ' + REDIS_URL);
    return true;
  }
  deps.redis = client;
  return false;
};

const IMG_A = `IMG0${'A'.repeat(14)}`;
const IMG_B = `IMG0${'B'.repeat(14)}`;

const makeLease = (overrides: Partial<BlobLease> = {}): BlobLease => ({
  documentId: 'D1',
  userId: 'U1',
  kind: 'image',
  name: 'photo.png',
  format: 'png',
  size: 1024,
  path: 'uploads/photo.png',
  nonce: 'nonce-1',
  state: 'pending',
  createdAt: Date.now(),
  ...overrides,
});

test('실 Redis: createBlobLeases는 PX 단위로 TTL을 설정한다', async (t) => {
  if (skipIfUnavailable(t)) return;
  await client.del(blobLeaseKey(IMG_A));

  const ok = await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  assert.equal(ok, true);

  const pttl = await client.pttl(blobLeaseKey(IMG_A));
  // lower-bounded too: env.BLOB_UPLOAD_TTL_SECONDS * 1000 losing its `* 1000` would still be > 0.
  assert.ok(pttl > 80_000 && pttl <= 90_000, `unexpected pttl: ${pttl}`);

  await client.del(blobLeaseKey(IMG_A));
});

test('실 Redis: createBlobLeases는 중복 키가 하나라도 있으면 원자적으로 전체 거부한다', async (t) => {
  if (skipIfUnavailable(t)) return;
  await client.del(blobLeaseKey(IMG_A), blobLeaseKey(IMG_B));

  await createBlobLeases([[IMG_A, makeLease({ nonce: 'existing' })]]);
  const ok = await createBlobLeases([
    [IMG_A, makeLease({ nonce: 'new-a' })],
    [IMG_B, makeLease({ nonce: 'new-b' })],
  ]);
  assert.equal(ok, false);

  const [a, b] = await readBlobLeases([IMG_A, IMG_B]);
  assert.equal(a?.nonce, 'existing');
  assert.equal(b, null);

  await client.del(blobLeaseKey(IMG_A), blobLeaseKey(IMG_B));
});

test('실 Redis: claimBlobLeaseForFinalize는 cjson 왕복으로 finalizing 전이·modification 고정을 수행한다', async (t) => {
  if (skipIfUnavailable(t)) return;
  await client.del(blobLeaseKey(IMG_A));

  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  const claimed = await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: [1, 2, 3] });
  assert.equal(claimed?.state, 'finalizing');
  assert.deepEqual(claimed?.modification, { crop: [1, 2, 3] });

  const reclaimed = await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: 'changed' });
  assert.deepEqual(reclaimed?.modification, { crop: [1, 2, 3] });

  await client.del(blobLeaseKey(IMG_A));
});

test('실 Redis: modification 안의 빈 배열은 cjson round-trip(claim/release 양쪽)에서 빈 오브젝트로 뭉개지지 않는다', async (t) => {
  if (skipIfUnavailable(t)) return;
  await client.del(blobLeaseKey(IMG_A));

  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);

  // claimBlobLeaseForFinalize's Lua script decodes+re-encodes the whole lease via cjson; if
  // modification were ever passed through cjson.decode/encode as a structured value, an empty
  // array here would come back as `{}` (verified against real Redis 8.8 — this is the exact
  // divergence the fix in blob-lease.ts:149 (`lease.modification = ARGV[3]`, no cjson.decode) and
  // the StoredBlobLease double-encoding exist to prevent).
  const claimed = await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: [], meta: { tags: [] } });
  assert.deepEqual(claimed?.modification, { crop: [], meta: { tags: [] } });
  assert.ok(Array.isArray((claimed?.modification as { crop: unknown }).crop), 'crop must stay an array, not become {}');

  // releaseFinalizeClaim also decodes+re-encodes the whole lease via cjson (to flip state back to
  // 'pending'), so it round-trips the already-pinned modification through cjson a second time.
  await releaseFinalizeClaim(IMG_A, 'n1');
  const [releasedLease] = await readBlobLeases([IMG_A]);
  assert.deepEqual(releasedLease?.modification, { crop: [], meta: { tags: [] } });
  assert.ok(Array.isArray((releasedLease?.modification as { crop: unknown }).crop), 'crop must survive the release round-trip too');

  await client.del(blobLeaseKey(IMG_A));
});

test('실 Redis: deleteBlobLeases는 KEYS/ARGV 오프셋을 정확히 사용해 일치 키만 지우고 [assetId, documentId]를 반환한다', async (t) => {
  if (skipIfUnavailable(t)) return;
  await client.del(blobLeaseKey(IMG_A), blobLeaseKey(IMG_B));

  await createBlobLeases([
    [IMG_A, makeLease({ nonce: 'n1', userId: 'U1', documentId: 'DOC-A' })],
    [IMG_B, makeLease({ nonce: 'n2', userId: 'U1', documentId: 'DOC-B' })],
  ]);

  const result = await deleteBlobLeases(
    [
      { assetId: IMG_A, nonce: 'n1' },
      { assetId: IMG_B, nonce: 'stale' },
    ],
    'U1',
  );
  assert.deepEqual(result, [[IMG_A, 'DOC-A']]);

  const [a, b] = await readBlobLeases([IMG_A, IMG_B]);
  assert.equal(a, null);
  assert.equal(b?.nonce, 'n2');

  await client.del(blobLeaseKey(IMG_A), blobLeaseKey(IMG_B));
});

test('실 Redis: abandonBlobLease는 pending에서만 삭제하고 이후 새 nonce 재예약을 허용한다', async (t) => {
  if (skipIfUnavailable(t)) return;
  await client.del(blobLeaseKey(IMG_A));

  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1', documentId: 'DOC-A' })]]);
  const documentId = await abandonBlobLease(IMG_A, 'n1', 'U1');
  assert.equal(documentId, 'DOC-A');

  const reselect = await createBlobLeases([[IMG_A, makeLease({ nonce: 'n2', userId: 'U1' })]]);
  assert.equal(reselect, true);

  await client.del(blobLeaseKey(IMG_A));
});
