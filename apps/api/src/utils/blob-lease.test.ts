import assert from 'node:assert/strict';
import { beforeEach, test } from 'node:test';
import {
  abandonBlobLease,
  blobLeaseKey,
  claimBlobLeaseForFinalize,
  createBlobLeases,
  deleteBlobLease,
  deleteBlobLeases,
  deps,
  extendBlobLeases,
  isValidAssetId,
  readBlobLeases,
  releaseFinalizeClaim,
} from './blob-lease.ts';
import type { Redis } from 'ioredis';
import type { BlobLease } from './blob-lease.ts';

const KEY_PREFIX_LENGTH = blobLeaseKey('').length;

type FakeRecord = { value: string; expiresAt: number };

class FakeRedis {
  store = new Map<string, FakeRecord>();

  #read(key: string): { record: FakeRecord; lease: BlobLease } | null {
    const record = this.store.get(key);
    if (!record || record.expiresAt <= Date.now()) {
      return null;
    }
    return { record, lease: JSON.parse(record.value) as BlobLease };
  }

  #create(keys: string[], argv: string[]): number {
    const ttlMs = Number(argv[keys.length]);
    if (keys.some((key) => this.#read(key) !== null)) {
      return 0;
    }
    const expiresAt = Date.now() + ttlMs;
    for (const [i, key] of keys.entries()) {
      this.store.set(key, { value: argv[i], expiresAt });
    }
    return 1;
  }

  #extend(keys: string[], argv: string[]): number {
    const userId = argv[keys.length];
    const ttlMs = Number(argv[keys.length + 1]);
    for (const [i, key] of keys.entries()) {
      const found = this.#read(key);
      if (found && found.lease.userId === userId && found.lease.nonce === argv[i]) {
        found.record.expiresAt = Date.now() + ttlMs;
      }
    }
    return 1;
  }

  #deleteMany(keys: string[], argv: string[]): [string, string][] {
    const userId = argv[keys.length];
    const deleted: [string, string][] = [];
    for (const [i, key] of keys.entries()) {
      const found = this.#read(key);
      if (found && found.lease.userId === userId && found.lease.nonce === argv[i]) {
        this.store.delete(key);
        deleted.push([key.slice(KEY_PREFIX_LENGTH), found.lease.documentId]);
      }
    }
    return deleted;
  }

  #deleteOne(keys: string[], argv: string[]): number {
    const found = this.#read(keys[0]);
    if (found && found.lease.nonce === argv[0]) {
      this.store.delete(keys[0]);
    }
    return 1;
  }

  #claim(keys: string[], argv: string[]): string | null {
    const found = this.#read(keys[0]);
    if (!found || found.lease.nonce !== argv[0]) {
      return null;
    }
    const { lease } = found;
    if (lease.state === 'pending') {
      lease.state = 'finalizing';
      if (argv[1] === '1') {
        // production never cjson.decodes this — it stays an opaque (pre-stringified) string on the
        // wire so empty arrays/objects inside it survive the round-trip; mirror that here verbatim.
        lease.modification = argv[2];
      }
    }
    const ttlMs = Number(argv[3]);
    const encoded = JSON.stringify(lease);
    this.store.set(keys[0], { value: encoded, expiresAt: Date.now() + ttlMs });
    return encoded;
  }

  #release(keys: string[], argv: string[]): number {
    const found = this.#read(keys[0]);
    if (found && found.lease.nonce === argv[0] && found.lease.state === 'finalizing') {
      found.lease.state = 'pending';
      found.record.value = JSON.stringify(found.lease);
    }
    return 1;
  }

  #abandon(keys: string[], argv: string[]): string | null {
    const found = this.#read(keys[0]);
    if (found && found.lease.userId === argv[0] && found.lease.nonce === argv[1] && found.lease.state === 'pending') {
      this.store.delete(keys[0]);
      return found.lease.documentId;
    }
    return null;
  }

  async mget(...keys: string[]): Promise<(string | null)[]> {
    return keys.map((key) => this.#read(key)?.record.value ?? null);
  }

  async eval(script: string, numkeys: number, ...rest: (string | number)[]): Promise<unknown> {
    const args = rest.map(String);
    const keys = args.slice(0, numkeys);
    const argv = args.slice(numkeys);
    const tag = script
      .split('\n')
      .map((line) => line.trim())
      .find((line) => line.startsWith('-- blob-lease:'));

    switch (tag) {
      case '-- blob-lease:create': {
        return this.#create(keys, argv);
      }
      case '-- blob-lease:extend': {
        return this.#extend(keys, argv);
      }
      case '-- blob-lease:delete-many': {
        return this.#deleteMany(keys, argv);
      }
      case '-- blob-lease:delete-one': {
        return this.#deleteOne(keys, argv);
      }
      case '-- blob-lease:claim': {
        return this.#claim(keys, argv);
      }
      case '-- blob-lease:release': {
        return this.#release(keys, argv);
      }
      case '-- blob-lease:abandon': {
        return this.#abandon(keys, argv);
      }
      default: {
        throw new Error(`FakeRedis: unrecognized script (${String(tag)})`);
      }
    }
  }
}

let fakeRedis: FakeRedis;

beforeEach(() => {
  fakeRedis = new FakeRedis();
  deps.redis = fakeRedis as unknown as Redis;
});

const IMG_A = `IMG0${'A'.repeat(14)}`;
const IMG_B = `IMG0${'B'.repeat(14)}`;
const FILE_A = `FILE0${'C'.repeat(14)}`;

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

test('isValidAssetId: 올바른 IMG…/FILE… 통과', () => {
  assert.equal(isValidAssetId(IMG_A, 'image'), true);
  assert.equal(isValidAssetId(FILE_A, 'file'), true);
});

test('isValidAssetId: IMG0(빈 접미사)·빈 문자열·과도한 길이·kind 불일치 거부', () => {
  assert.equal(isValidAssetId('IMG0', 'image'), false);
  assert.equal(isValidAssetId('', 'image'), false);
  assert.equal(isValidAssetId(`IMG0${'A'.repeat(15)}`, 'image'), false);
  assert.equal(isValidAssetId(IMG_A, 'file'), false);
  assert.equal(isValidAssetId(FILE_A, 'image'), false);
});

test('createBlobLeases: 전 항목 신규면 원자 생성 후 true', async () => {
  const ok = await createBlobLeases([
    [IMG_A, makeLease({ nonce: 'n1' })],
    [IMG_B, makeLease({ nonce: 'n2' })],
  ]);
  assert.equal(ok, true);

  const [a, b] = await readBlobLeases([IMG_A, IMG_B]);
  assert.equal(a?.nonce, 'n1');
  assert.equal(b?.nonce, 'n2');
});

test('createBlobLeases: 하나라도 기존 키 존재 시 아무것도 만들지 않고 false', async () => {
  const ok1 = await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  assert.equal(ok1, true);

  const ok2 = await createBlobLeases([
    [IMG_A, makeLease({ nonce: 'n-conflict' })],
    [IMG_B, makeLease({ nonce: 'n2' })],
  ]);
  assert.equal(ok2, false);

  const [a, b] = await readBlobLeases([IMG_A, IMG_B]);
  assert.equal(a?.nonce, 'n1');
  assert.equal(b, null);
});

test('createBlobLeases: 입력에 같은 assetId가 둘 이상이면 거부', async () => {
  await assert.rejects(
    createBlobLeases([
      [IMG_A, makeLease({ nonce: 'n1' })],
      [IMG_A, makeLease({ nonce: 'n2' })],
    ]),
  );
  const [a] = await readBlobLeases([IMG_A]);
  assert.equal(a, null);
});

test('readBlobLeases: 순서 보존, 없는 키 null', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  const result = await readBlobLeases([IMG_B, IMG_A]);
  assert.equal(result[0], null);
  assert.equal(result[1]?.nonce, 'n1');
});

test('extendBlobLeases: userId+nonce 일치 키만 TTL 갱신, 불일치·부재는 무시(오류 없음)', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1' })]]);
  const key = blobLeaseKey(IMG_A);
  const before = fakeRedis.store.get(key)?.expiresAt;
  assert.ok(before !== undefined);

  await new Promise((resolve) => setTimeout(resolve, 5));
  await extendBlobLeases(
    [
      { assetId: IMG_A, nonce: 'n1' },
      { assetId: IMG_A, nonce: 'wrong-nonce' },
      { assetId: IMG_B, nonce: 'n-missing' },
    ],
    'U1',
  );

  const after = fakeRedis.store.get(key)?.expiresAt;
  assert.ok(after !== undefined && after > (before as number));
});

test('extendBlobLeases: nonce 불일치 키는 TTL 변경 없이 그대로', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1' })]]);
  const key = blobLeaseKey(IMG_A);
  const before = fakeRedis.store.get(key)?.expiresAt;

  await extendBlobLeases([{ assetId: IMG_A, nonce: 'other' }], 'U1');
  const after = fakeRedis.store.get(key)?.expiresAt;
  assert.equal(after, before);
});

test('extendBlobLeases: nonce는 맞아도 userId가 다르면(타 테넌트) TTL 변경 없이 그대로', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1' })]]);
  const key = blobLeaseKey(IMG_A);
  const before = fakeRedis.store.get(key)?.expiresAt;

  await extendBlobLeases([{ assetId: IMG_A, nonce: 'n1' }], 'U2');
  const after = fakeRedis.store.get(key)?.expiresAt;
  assert.equal(after, before);
});

test('deleteBlobLeases: userId+nonce 일치 키만 삭제, [assetId, documentId] 반환', async () => {
  await createBlobLeases([
    [IMG_A, makeLease({ nonce: 'n1', userId: 'U1', documentId: 'DOC-A' })],
    [IMG_B, makeLease({ nonce: 'n2', userId: 'U1', documentId: 'DOC-B' })],
  ]);

  const result = await deleteBlobLeases(
    [
      { assetId: IMG_A, nonce: 'n1' },
      { assetId: IMG_B, nonce: 'stale-nonce' },
    ],
    'U1',
  );

  assert.deepEqual(result, [[IMG_A, 'DOC-A']]);

  const [a, b] = await readBlobLeases([IMG_A, IMG_B]);
  assert.equal(a, null);
  assert.equal(b?.nonce, 'n2');
});

test('deleteBlobLeases: nonce는 맞아도 userId가 다르면(타 테넌트) 삭제하지 않고 빈 목록 반환', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1', documentId: 'DOC-A' })]]);

  const result = await deleteBlobLeases([{ assetId: IMG_A, nonce: 'n1' }], 'U2');
  assert.deepEqual(result, []);

  const [a] = await readBlobLeases([IMG_A]);
  assert.equal(a?.nonce, 'n1');
});

test('deleteBlobLease: nonce 일치 시에만 삭제', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);

  await deleteBlobLease(IMG_A, 'wrong-nonce');
  const [afterWrongNonce] = await readBlobLeases([IMG_A]);
  assert.notEqual(afterWrongNonce, null);

  await deleteBlobLease(IMG_A, 'n1');
  const [afterMatchingNonce] = await readBlobLeases([IMG_A]);
  assert.equal(afterMatchingNonce, null);
});

test('claimBlobLeaseForFinalize: nonce 일치면 finalizing 전이·modification 고정·TTL 갱신 후 lease 반환', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);

  const claimed = await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: 'a' });
  assert.equal(claimed?.state, 'finalizing');
  assert.deepEqual(claimed?.modification, { crop: 'a' });
});

test('claimBlobLeaseForFinalize: modification 안의 빈 배열은 배열로 왕복한다(빈 오브젝트로 뭉개지지 않음)', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);

  const claimed = await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: [], meta: { tags: [] } });
  assert.deepEqual(claimed?.modification, { crop: [], meta: { tags: [] } });
  assert.ok(Array.isArray((claimed?.modification as { crop: unknown }).crop));

  const [read] = await readBlobLeases([IMG_A]);
  assert.deepEqual(read?.modification, { crop: [], meta: { tags: [] } });
});

test('claimBlobLeaseForFinalize: 이미 finalizing이어도 같은 nonce면 재진입(non-null)하고 modification은 고정된 채 유지', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: 'first' });

  const reclaimed = await claimBlobLeaseForFinalize(IMG_A, 'n1', { crop: 'second' });
  assert.equal(reclaimed?.state, 'finalizing');
  assert.deepEqual(reclaimed?.modification, { crop: 'first' });
});

test('claimBlobLeaseForFinalize: nonce 불일치·부재면 null', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  assert.equal(await claimBlobLeaseForFinalize(IMG_A, 'wrong'), null);
  assert.equal(await claimBlobLeaseForFinalize(IMG_B, 'n1'), null);
});

test('createBlobLeases(re-claim)는 다른 nonce의 finalizing lease를 거부한다', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  await claimBlobLeaseForFinalize(IMG_A, 'n1');

  const reclaim = await createBlobLeases([[IMG_A, makeLease({ nonce: 'n2' })]]);
  assert.equal(reclaim, false);
});

test('releaseFinalizeClaim: nonce 일치 + finalizing이면 pending으로 복귀, 그 외 무시', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  await claimBlobLeaseForFinalize(IMG_A, 'n1');

  await releaseFinalizeClaim(IMG_A, 'n1');
  const [released] = await readBlobLeases([IMG_A]);
  assert.equal(released?.state, 'pending');
});

test('releaseFinalizeClaim: 해제 후에도 키가 남아 새 nonce createBlobLeases는 거부되고 같은 nonce 재시도는 성공', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1' })]]);
  await claimBlobLeaseForFinalize(IMG_A, 'n1');
  await releaseFinalizeClaim(IMG_A, 'n1');

  const reserveWithNewNonce = await createBlobLeases([[IMG_A, makeLease({ nonce: 'n2' })]]);
  assert.equal(reserveWithNewNonce, false);

  const retry = await claimBlobLeaseForFinalize(IMG_A, 'n1');
  assert.equal(retry?.state, 'finalizing');
});

test('abandonBlobLease: userId·nonce 일치 且 pending이면 DEL 후 documentId 반환, 이후 새 nonce 예약 성공', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1', documentId: 'DOC-A' })]]);

  const documentId = await abandonBlobLease(IMG_A, 'n1', 'U1');
  assert.equal(documentId, 'DOC-A');

  const reselect = await createBlobLeases([[IMG_A, makeLease({ nonce: 'n2', userId: 'U1' })]]);
  assert.equal(reselect, true);
});

test('abandonBlobLease: finalizing이면 null(삭제하지 않음)', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1' })]]);
  await claimBlobLeaseForFinalize(IMG_A, 'n1');

  const result = await abandonBlobLease(IMG_A, 'n1', 'U1');
  assert.equal(result, null);

  const [lease] = await readBlobLeases([IMG_A]);
  assert.equal(lease?.state, 'finalizing');
});

test('abandonBlobLease: nonce 불일치·부재면 null', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1' })]]);
  assert.equal(await abandonBlobLease(IMG_A, 'wrong', 'U1'), null);
  assert.equal(await abandonBlobLease(IMG_B, 'n1', 'U1'), null);
});

test('abandonBlobLease: nonce는 맞아도 userId가 다르면(타 테넌트) null, 삭제하지 않음', async () => {
  await createBlobLeases([[IMG_A, makeLease({ nonce: 'n1', userId: 'U1', documentId: 'DOC-A' })]]);

  const result = await abandonBlobLease(IMG_A, 'n1', 'U2');
  assert.equal(result, null);

  const [lease] = await readBlobLeases([IMG_A]);
  assert.equal(lease?.nonce, 'n1');
});
