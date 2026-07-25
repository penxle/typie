import { TableCode } from '#/db/schemas/codes.ts';
import { env } from '#/env.ts';
import type { Redis } from 'ioredis';

export type BlobLeaseKind = 'image' | 'file';

export type BlobLease = {
  documentId: string;
  userId: string;
  kind: BlobLeaseKind;
  name: string;
  format: string;
  size: number;
  path: string;
  nonce: string;
  state: 'pending' | 'finalizing';
  modification?: unknown;
  createdAt: number;
};

export const deps: { redis: Redis | undefined } = { redis: undefined };

// deps.redis가 비어 있으면 cache.ts의 공유 클라이언트를 첫 사용 시점에만 동적 import한다. import 시점에
// 소켓을 열지 않아 이 모듈을 요구하는 것만으로 네트워크/env 부작용이 생기지 않는다(테스트는 deps.redis를
// 먼저 채워 이 경로를 건너뛴다).
const getRedis = async (): Promise<Redis> => {
  let redis = deps.redis;
  if (redis === undefined) {
    const cache = await import('#/cache.ts');
    redis = cache.redis;
    deps.redis = redis;
  }
  return redis;
};

export const blobLeaseKey = (assetId: string): string => `blob:upload:${assetId}`;

// modification은 어떤 값이든(빈 배열 포함) cjson round-trip을 타면 구조가 깨질 수 있어 Lua에는 절대
// 구조화된 값으로 넘기지 않는다 — 저장 시 JSON 문자열로 한 번 더 감싸 Lua에는 opaque string으로만
// 보이게 하고, 읽을 때 JS에서 그 문자열을 다시 파싱한다.
type StoredBlobLease = Omit<BlobLease, 'modification'> & { modification?: string };

const serializeLeaseForStorage = (lease: BlobLease): string => {
  const { modification, ...rest } = lease;
  const stored: StoredBlobLease = modification === undefined ? rest : { ...rest, modification: JSON.stringify(modification) };
  return JSON.stringify(stored);
};

const deserializeLeaseFromStorage = (raw: string): BlobLease => {
  const stored = JSON.parse(raw) as StoredBlobLease;
  const { modification, ...rest } = stored;
  return modification === undefined ? rest : { ...rest, modification: JSON.parse(modification) as unknown };
};

const ASSET_ID_SUFFIX_LENGTH = 14;

const ASSET_ID_PATTERNS: Record<BlobLeaseKind, RegExp> = {
  image: new RegExp(`^${TableCode.IMAGES}0[A-Z0-9]{${ASSET_ID_SUFFIX_LENGTH}}$`),
  file: new RegExp(`^${TableCode.FILES}0[A-Z0-9]{${ASSET_ID_SUFFIX_LENGTH}}$`),
};

export const isValidAssetId = (assetId: string, kind: BlobLeaseKind): boolean => {
  return ASSET_ID_PATTERNS[kind].test(assetId);
};

const KEY_PREFIX_LENGTH = blobLeaseKey('').length;

// NX 전 항목 확인 후 전부 SET. 하나라도 존재하면 아무것도 만들지 않고 0.
const CREATE_LEASES_SCRIPT = `
-- blob-lease:create
local n = #KEYS
local ttlMs = ARGV[n + 1]
for i = 1, n do
  if redis.call('EXISTS', KEYS[i]) == 1 then
    return 0
  end
end
for i = 1, n do
  redis.call('SET', KEYS[i], ARGV[i], 'PX', ttlMs)
end
return 1
`;

// userId+nonce 일치 키만 PEXPIRE.
const EXTEND_LEASES_SCRIPT = `
-- blob-lease:extend
local n = #KEYS
local userId = ARGV[n + 1]
local ttlMs = ARGV[n + 2]
for i = 1, n do
  local raw = redis.call('GET', KEYS[i])
  if raw then
    local lease = cjson.decode(raw)
    if lease.userId == userId and lease.nonce == ARGV[i] then
      redis.call('PEXPIRE', KEYS[i], ttlMs)
    end
  end
end
return 1
`;

// userId+nonce 일치 키만 DEL. [assetId, documentId] 목록 반환.
const DELETE_LEASES_SCRIPT = `
-- blob-lease:delete-many
local n = #KEYS
local userId = ARGV[n + 1]
local prefixLen = ${KEY_PREFIX_LENGTH}
local deleted = {}
for i = 1, n do
  local raw = redis.call('GET', KEYS[i])
  if raw then
    local lease = cjson.decode(raw)
    if lease.userId == userId and lease.nonce == ARGV[i] then
      redis.call('DEL', KEYS[i])
      table.insert(deleted, {string.sub(KEYS[i], prefixLen + 1), lease.documentId})
    end
  end
end
return deleted
`;

// nonce 일치 시에만 DEL (finalize 성공 경로 전용).
const DELETE_LEASE_SCRIPT = `
-- blob-lease:delete-one
local raw = redis.call('GET', KEYS[1])
if raw then
  local lease = cjson.decode(raw)
  if lease.nonce == ARGV[1] then
    redis.call('DEL', KEYS[1])
  end
end
return 1
`;

// nonce 일치일 때 state를 finalizing으로 전이(pending에서만 modification 고정)하고 TTL을 갱신 후
// lease를 반환. 이미 finalizing이어도 같은 nonce면 재진입 허용.
const CLAIM_FOR_FINALIZE_SCRIPT = `
-- blob-lease:claim
local raw = redis.call('GET', KEYS[1])
if not raw then
  return false
end
local lease = cjson.decode(raw)
if lease.nonce ~= ARGV[1] then
  return false
end
if lease.state == 'pending' then
  lease.state = 'finalizing'
  if ARGV[2] == '1' then
    lease.modification = ARGV[3]
  end
end
local encoded = cjson.encode(lease)
redis.call('SET', KEYS[1], encoded, 'PX', ARGV[4])
return encoded
`;

// nonce 일치 + finalizing이면 pending으로 복귀. TTL은 KEEPTTL로 보존.
const RELEASE_FINALIZE_CLAIM_SCRIPT = `
-- blob-lease:release
local raw = redis.call('GET', KEYS[1])
if raw then
  local lease = cjson.decode(raw)
  if lease.nonce == ARGV[1] and lease.state == 'finalizing' then
    lease.state = 'pending'
    redis.call('SET', KEYS[1], cjson.encode(lease), 'KEEPTTL')
  end
end
return 1
`;

// userId+nonce 일치 且 state == pending 일 때만 DEL, documentId 반환.
const ABANDON_LEASE_SCRIPT = `
-- blob-lease:abandon
local raw = redis.call('GET', KEYS[1])
if not raw then
  return false
end
local lease = cjson.decode(raw)
if lease.userId == ARGV[1] and lease.nonce == ARGV[2] and lease.state == 'pending' then
  redis.call('DEL', KEYS[1])
  return lease.documentId
end
return false
`;

export const createBlobLeases = async (entries: [assetId: string, lease: BlobLease][]): Promise<boolean> => {
  const assetIds = entries.map(([assetId]) => assetId);
  if (new Set(assetIds).size !== assetIds.length) {
    throw new Error('Duplicate assetId in createBlobLeases entries');
  }

  if (entries.length === 0) {
    return true;
  }

  const keys = assetIds.map(blobLeaseKey);
  const values = entries.map(([, lease]) => serializeLeaseForStorage(lease));
  const ttlMs = env.BLOB_UPLOAD_TTL_SECONDS * 1000;

  const redis = await getRedis();
  const result = await redis.eval(CREATE_LEASES_SCRIPT, keys.length, ...keys, ...values, ttlMs);
  return result === 1;
};

export const readBlobLeases = async (assetIds: string[]): Promise<(BlobLease | null)[]> => {
  if (assetIds.length === 0) {
    return [];
  }

  const redis = await getRedis();
  const values = await redis.mget(...assetIds.map(blobLeaseKey));
  return values.map((value) => (value === null ? null : deserializeLeaseFromStorage(value)));
};

export const extendBlobLeases = async (items: { assetId: string; nonce: string }[], userId: string): Promise<void> => {
  if (items.length === 0) {
    return;
  }

  const keys = items.map((item) => blobLeaseKey(item.assetId));
  const nonces = items.map((item) => item.nonce);
  const ttlMs = env.BLOB_UPLOAD_TTL_SECONDS * 1000;

  const redis = await getRedis();
  await redis.eval(EXTEND_LEASES_SCRIPT, keys.length, ...keys, ...nonces, userId, ttlMs);
};

export const deleteBlobLeases = async (items: { assetId: string; nonce: string }[], userId: string): Promise<[string, string][]> => {
  if (items.length === 0) {
    return [];
  }

  const keys = items.map((item) => blobLeaseKey(item.assetId));
  const nonces = items.map((item) => item.nonce);

  const redis = await getRedis();
  const result = await redis.eval(DELETE_LEASES_SCRIPT, keys.length, ...keys, ...nonces, userId);
  return result as [string, string][];
};

export const deleteBlobLease = async (assetId: string, nonce: string): Promise<void> => {
  const redis = await getRedis();
  await redis.eval(DELETE_LEASE_SCRIPT, 1, blobLeaseKey(assetId), nonce);
};

export const claimBlobLeaseForFinalize = async (assetId: string, nonce: string, modification?: unknown): Promise<BlobLease | null> => {
  const hasModification = modification !== undefined;
  const ttlMs = env.BLOB_UPLOAD_TTL_SECONDS * 1000;

  const redis = await getRedis();
  const result = await redis.eval(
    CLAIM_FOR_FINALIZE_SCRIPT,
    1,
    blobLeaseKey(assetId),
    nonce,
    hasModification ? '1' : '0',
    hasModification ? JSON.stringify(modification) : '',
    ttlMs,
  );
  return result === null ? null : deserializeLeaseFromStorage(result as string);
};

export const releaseFinalizeClaim = async (assetId: string, nonce: string): Promise<void> => {
  const redis = await getRedis();
  await redis.eval(RELEASE_FINALIZE_CLAIM_SCRIPT, 1, blobLeaseKey(assetId), nonce);
};

export const abandonBlobLease = async (assetId: string, nonce: string, userId: string): Promise<string | null> => {
  const redis = await getRedis();
  const result = await redis.eval(ABANDON_LEASE_SCRIPT, 1, blobLeaseKey(assetId), userId, nonce);
  return result === null ? null : (result as string);
};
