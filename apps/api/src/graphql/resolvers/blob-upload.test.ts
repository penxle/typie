import assert from 'node:assert/strict';
import test from 'node:test';
import { TypieError } from '@typie/lib/errors';
import { abandonBlobUpload, finalizeBlobUpload, isUniqueViolation, normalizeContentType, reserveBlobUploads } from './blob-upload.ts';
import type { BlobLease, BlobLeaseKind } from '#/utils/blob-lease.ts';
import type {
  AbandonBlobUploadDeps,
  FinalizeBlobUploadDeps,
  ReserveBlobUploadItem,
  ReserveBlobUploadsDeps,
  UploadedObject,
} from './blob-upload.ts';

type FakeRow = { id: string; path: string; name: string };

type World = ReturnType<typeof createWorld>;

const errorWithCode = (code: string) => (error: unknown) => error instanceof TypieError && error.code === code;

const createWorld = () => {
  const leases = new Map<string, BlobLease>();
  const completed = new Map<string, FakeRow>();
  const published: { documentId: string; ids: string[] }[] = [];
  const presigns: { key: string; name: string; contentType: string; size: number; userId: string }[] = [];
  const outputKeys: string[] = [];
  const persisted: { assetId: string; outputKey: string; modification: unknown; body: Uint8Array | undefined }[] = [];

  let sequence = 0;
  const next = () => {
    sequence += 1;
    return String(sequence).padStart(14, '0');
  };

  const uniqueViolation = new Error('duplicate key value violates unique constraint');

  const state = {
    leases,
    completed,
    published,
    presigns,
    outputKeys,
    persisted,
    subscribed: true,
    createLeasesResult: true as boolean,
    persistError: null as Error | null,
    uploadedObject: {
      userId: 'U0OWNER',
      contentType: 'image/png',
      contentLength: 100,
      body: new Uint8Array([1, 2, 3]),
    } as UploadedObject,
    readUploadedObjectError: null as Error | null,
    uniqueViolation,
  };

  const reserveDeps: ReserveBlobUploadsDeps = {
    assertSubscription: async () => {
      if (!state.subscribed) {
        throw new TypieError({ code: 'subscription_required', status: 403 });
      }
    },
    hasCompletedAsset: async (assetId) => completed.has(assetId),
    readLeases: async (assetIds) => assetIds.map((assetId) => leases.get(assetId) ?? null),
    createLeases: async (entries) => {
      if (!state.createLeasesResult || entries.some(([assetId]) => leases.has(assetId))) {
        return false;
      }

      for (const [assetId, lease] of entries) {
        leases.set(assetId, { ...lease });
      }

      return true;
    },
    createAssetId: (kind) => `${kind === 'image' ? 'IMG' : 'FILE'}0${next()}`,
    createNonce: () => `nonce-${next()}`,
    createObjectKey: () => `26/07/a/ab/key-${next()}`,
    presign: async ({ key, name, contentType, size, userId }) => {
      presigns.push({ key, name, contentType, size, userId });
      return { url: 'https://typie-uploads.s3.amazonaws.com', fields: { key, 'Content-Type': contentType } };
    },
    publishAssetsChanged: async (documentId, ids) => {
      published.push({ documentId, ids });
    },
    now: () => 1000,
  };

  const finalizeDeps: FinalizeBlobUploadDeps<FakeRow> = {
    loadCompletedAsset: async (assetId) => completed.get(assetId) ?? null,
    readLease: async (assetId) => leases.get(assetId) ?? null,
    claimLease: async (assetId, nonce, modification) => {
      const lease = leases.get(assetId);
      if (!lease || lease.nonce !== nonce) {
        return null;
      }

      if (lease.state === 'pending') {
        lease.state = 'finalizing';
        if (modification !== undefined) {
          lease.modification = modification;
        }
      }

      return { ...lease };
    },
    releaseClaim: async (assetId, nonce) => {
      const lease = leases.get(assetId);
      if (lease && lease.nonce === nonce && lease.state === 'finalizing') {
        lease.state = 'pending';
      }
    },
    deleteLease: async (assetId, nonce) => {
      const lease = leases.get(assetId);
      if (lease && lease.nonce === nonce) {
        leases.delete(assetId);
      }
    },
    readUploadedObject: async () => {
      if (state.readUploadedObjectError) {
        throw state.readUploadedObjectError;
      }

      return state.uploadedObject;
    },
    createOutputKey: () => {
      const key = `26/07/o/ou/out-${next()}`;
      outputKeys.push(key);
      return key;
    },
    persist: async ({ assetId, lease, object, outputKey }) => {
      persisted.push({ assetId, outputKey, modification: lease.modification, body: object.body });

      if (state.persistError) {
        throw state.persistError;
      }

      if (completed.has(assetId)) {
        throw uniqueViolation;
      }

      const row = { id: assetId, path: outputKey, name: lease.name };
      completed.set(assetId, row);
      return row;
    },
    isUniqueViolation: (error) => error === uniqueViolation,
    publishAssetsChanged: async (documentId, ids) => {
      published.push({ documentId, ids });
    },
  };

  const abandonDeps: AbandonBlobUploadDeps = {
    abandonLease: async (assetId, nonce, userId) => {
      const lease = leases.get(assetId);
      if (lease && lease.userId === userId && lease.nonce === nonce && lease.state === 'pending') {
        leases.delete(assetId);
        return lease.documentId;
      }

      return null;
    },
    publishAssetsChanged: async (documentId, ids) => {
      published.push({ documentId, ids });
    },
  };

  return { state, reserveDeps, finalizeDeps, abandonDeps };
};

const item = (kind: BlobLeaseKind, overrides: Partial<ReserveBlobUploadItem> = {}): ReserveBlobUploadItem => ({
  kind,
  name: kind === 'image' ? 'photo.png' : 'report.pdf',
  format: kind === 'image' ? 'image/png' : 'application/pdf',
  size: 100,
  ...overrides,
});

const reserve = (world: World, items: ReserveBlobUploadItem[], documentId = 'DOC0DOCUMENT') =>
  reserveBlobUploads({ documentId, userId: 'U0OWNER', items, deps: world.reserveDeps });

const seedLease = (world: World, assetId: string, overrides: Partial<BlobLease> = {}) => {
  const lease: BlobLease = {
    documentId: 'DOC0DOCUMENT',
    userId: 'U0OWNER',
    kind: 'image',
    name: 'photo.png',
    format: 'image/png',
    size: 100,
    path: '26/07/a/ab/upload.png',
    nonce: 'nonce-1',
    state: 'pending',
    createdAt: 1000,
    ...overrides,
  };

  world.state.leases.set(assetId, lease);
  return lease;
};

// completed row를 한 번만 감춰, 두 run이 completed-first 검사를 모두 통과한 뒤 PK에서 갈리는 경합을 만든다.
const hideCompletedOnce = (world: World) => {
  const load = world.finalizeDeps.loadCompletedAsset;
  let hidden = true;

  world.finalizeDeps.loadCompletedAsset = async (assetId) => {
    if (hidden) {
      hidden = false;
      return null;
    }

    return await load(assetId);
  };
};

const finalizeImage = (world: World, assetId: string, nonce: string, modification?: unknown) =>
  finalizeBlobUpload({ assetId, nonce, kind: 'image', userId: 'U0OWNER', modification, deps: world.finalizeDeps });

test('normalizeContentType: 빈 값·공백은 application/octet-stream으로, 나머지는 그대로', () => {
  assert.equal(normalizeContentType(''), 'application/octet-stream');
  assert.equal(normalizeContentType(' '.repeat(3)), 'application/octet-stream');
  assert.equal(normalizeContentType(undefined), 'application/octet-stream');
  assert.equal(normalizeContentType('image/png'), 'image/png');
});

test('isUniqueViolation: drizzle가 감싼 오류의 cause 체인에서 SQLSTATE 23505를 찾는다', () => {
  const pgError = Object.assign(new Error('duplicate key value violates unique constraint'), { code: '23505' });

  assert.equal(isUniqueViolation(pgError), true);
  assert.equal(isUniqueViolation(new Error('Failed query', { cause: pgError })), true);
  assert.equal(isUniqueViolation(new Error('Failed query', { cause: new Error('connection reset') })), false);
  assert.equal(isUniqueViolation(Object.assign(new Error('fk'), { code: '23503' })), false);
  assert.equal(isUniqueViolation(null), false);
});

test('reserve: [IMAGE, FILE, IMAGE] 응답이 입력 순서·kind를 보존하고 ID prefix와 nonce를 갖는다', async () => {
  const world = createWorld();

  const reservations = await reserve(world, [item('image'), item('file'), item('image')]);

  assert.equal(reservations.length, 3);
  assert.ok(reservations[0].assetId.startsWith('IMG0'));
  assert.ok(reservations[1].assetId.startsWith('FILE0'));
  assert.ok(reservations[2].assetId.startsWith('IMG0'));

  for (const reservation of reservations) {
    assert.ok(reservation.nonce.length > 0);
    assert.ok(reservation.path.length > 0);
    assert.equal(reservation.uploadUrl, 'https://typie-uploads.s3.amazonaws.com');
    assert.equal(reservation.uploadFields.key, reservation.path);
  }

  assert.equal(new Set(reservations.map((reservation) => reservation.nonce)).size, 3);
  assert.equal(new Set(reservations.map((reservation) => reservation.path)).size, 3);
  assert.deepEqual(
    reservations.map((reservation) => world.state.leases.get(reservation.assetId)?.kind),
    ['image', 'file', 'image'],
  );
});

test('reserve: presign은 정규화 Content-Type과 예약 size로 발급되고 lease.format도 같은 정규화 값이다', async () => {
  const world = createWorld();

  const [reservation] = await reserve(world, [item('file', { format: ' ', size: 4096, name: 'archive.zip' })]);

  assert.deepEqual(world.state.presigns, [
    {
      key: reservation.path,
      name: 'archive.zip',
      contentType: 'application/octet-stream',
      size: 4096,
      userId: 'U0OWNER',
    },
  ]);
  assert.equal(world.state.leases.get(reservation.assetId)?.format, 'application/octet-stream');
  assert.ok(reservation.path.endsWith('.zip'));
});

test('reserve: 성공 시 batch 전체를 한 무효화 이벤트로 publish한다', async () => {
  const world = createWorld();

  const reservations = await reserve(world, [item('image'), item('file')]);

  assert.deepEqual(world.state.published, [{ documentId: 'DOC0DOCUMENT', ids: reservations.map((reservation) => reservation.assetId) }]);
});

test('reserve: lease 생성이 하나라도 실패하면 전체 실패하고 응답도 publish도 없다', async () => {
  const world = createWorld();
  world.state.createLeasesResult = false;

  await assert.rejects(reserve(world, [item('image'), item('file')]), errorWithCode('asset_upload_in_progress'));

  assert.equal(world.state.leases.size, 0);
  assert.equal(world.state.published.length, 0);
});

test('reserve: 항목 수 상한 초과·빈 batch는 거부된다', async () => {
  const world = createWorld();

  await assert.rejects(reserve(world, []), errorWithCode('validation_error'));
  await assert.rejects(
    reserve(
      world,
      Array.from({ length: 51 }, () => item('image')),
    ),
    errorWithCode('validation_error'),
  );

  assert.equal(world.state.leases.size, 0);
});

test('reserve: batch 내 assetId 중복은 거부된다', async () => {
  const world = createWorld();

  await assert.rejects(
    reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' }), item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]),
    errorWithCode('validation_error'),
  );

  assert.equal(world.state.leases.size, 0);
});

test('reserve: 상한을 넘는 size는 거부된다', async () => {
  const world = createWorld();

  await assert.rejects(reserve(world, [item('image', { size: 1024 * 1024 * 1024 + 1 })]), errorWithCode('validation_error'));
  await assert.rejects(reserve(world, [item('image', { size: -1 })]), errorWithCode('validation_error'));

  assert.equal(world.state.leases.size, 0);
});

test('reserve: re-claim assetId는 엄격 형식 검증을 통과해야 한다', async () => {
  const world = createWorld();

  await assert.rejects(reserve(world, [item('image', { assetId: 'IMG0' })]), errorWithCode('validation_error'));
  await assert.rejects(reserve(world, [item('image', { assetId: 'IMG0AAAA' })]), errorWithCode('validation_error'));
  await assert.rejects(reserve(world, [item('image', { assetId: 'FILE0AAAAAAAAAAAAAA' })]), errorWithCode('validation_error'));
  await assert.rejects(reserve(world, [item('file', { assetId: 'IMG0AAAAAAAAAAAAAA' })]), errorWithCode('validation_error'));

  assert.equal(world.state.leases.size, 0);
  assert.equal(world.state.presigns.length, 0);
});

test('reserve: re-claim은 completed row가 있으면 거부된다', async () => {
  const world = createWorld();
  world.state.completed.set('IMG0AAAAAAAAAAAAAA', { id: 'IMG0AAAAAAAAAAAAAA', path: 'p', name: 'n' });

  await assert.rejects(reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]), errorWithCode('asset_already_exists'));

  assert.equal(world.state.leases.size, 0);
});

test('reserve: re-claim은 pending·finalizing lease가 남아 있으면 거부된다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');

  await assert.rejects(reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]), errorWithCode('asset_upload_in_progress'));

  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { state: 'finalizing' });
  await assert.rejects(reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]), errorWithCode('asset_upload_in_progress'));
});

test('reserve: re-claim이 네 검사를 모두 통과하면 같은 ID로 새 lease를 만든다', async () => {
  const world = createWorld();

  const [reservation] = await reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]);

  assert.equal(reservation.assetId, 'IMG0AAAAAAAAAAAAAA');
  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.nonce, reservation.nonce);
  assert.deepEqual(world.state.published, [{ documentId: 'DOC0DOCUMENT', ids: ['IMG0AAAAAAAAAAAAAA'] }]);
});

test('reserve: 구독이 없으면 subscription_required이고 lease가 생기지 않는다', async () => {
  const world = createWorld();
  world.state.subscribed = false;

  await assert.rejects(reserve(world, [item('image')]), errorWithCode('subscription_required'));

  assert.equal(world.state.leases.size, 0);
  assert.equal(world.state.presigns.length, 0);
  assert.equal(world.state.published.length, 0);
});

test('finalize: 성공 시 explicit ID row를 만들고 lease를 지운 뒤 무효화를 publish한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');

  const row = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  assert.equal(row.id, 'IMG0AAAAAAAAAAAAAA');
  assert.equal(row.path, `${world.state.outputKeys[0]}.png`);
  assert.equal(world.state.leases.size, 0);
  assert.deepEqual(world.state.published, [{ documentId: 'DOC0DOCUMENT', ids: ['IMG0AAAAAAAAAAAAAA'] }]);
});

test('finalize: 이미 finalizing인 lease도 같은 nonce면 재진입해 완주한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { state: 'finalizing' });

  const row = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  assert.equal(row.id, 'IMG0AAAAAAAAAAAAAA');
  assert.equal(world.state.leases.size, 0);
});

test('finalize: 다른 nonce는 선점하지 못하고 asset_upload_expired가 된다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { state: 'finalizing' });

  await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-other'), errorWithCode('asset_upload_expired'));

  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.state, 'finalizing');
  assert.equal(world.state.persisted.length, 0);
});

test('finalize: completed row가 있으면 처리 없이 반환하고, 남은 nonce 일치 lease를 정리·재발행한다', async () => {
  const world = createWorld();
  world.state.completed.set('IMG0AAAAAAAAAAAAAA', { id: 'IMG0AAAAAAAAAAAAAA', path: 'settled', name: 'photo.png' });
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');

  const row = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  assert.equal(row.path, 'settled');
  assert.equal(world.state.persisted.length, 0);
  assert.equal(world.state.leases.size, 0);
  assert.deepEqual(world.state.published, [{ documentId: 'DOC0DOCUMENT', ids: ['IMG0AAAAAAAAAAAAAA'] }]);
});

test('finalize: completed row만 있고 lease가 없으면 재발행 없이 반환한다', async () => {
  const world = createWorld();
  world.state.completed.set('IMG0AAAAAAAAAAAAAA', { id: 'IMG0AAAAAAAAAAAAAA', path: 'settled', name: 'photo.png' });

  const row = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  assert.equal(row.path, 'settled');
  assert.equal(world.state.published.length, 0);
});

test('finalize: completed row가 있어도 nonce가 다른 lease는 건드리지 않는다', async () => {
  const world = createWorld();
  world.state.completed.set('IMG0AAAAAAAAAAAAAA', { id: 'IMG0AAAAAAAAAAAAAA', path: 'settled', name: 'photo.png' });
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { nonce: 'nonce-new' });

  await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.nonce, 'nonce-new');
  assert.equal(world.state.published.length, 0);
});

test('finalize: 다른 사용자의 lease는 선점조차 하지 않아 pending·modification 미고정으로 남는다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { userId: 'U0OTHER' });

  await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1', { resize: { width: 1 } }), errorWithCode('forbidden'));

  const lease = world.state.leases.get('IMG0AAAAAAAAAAAAAA');
  assert.equal(lease?.state, 'pending');
  assert.equal(lease?.modification, undefined);
  assert.equal(world.state.persisted.length, 0);

  // 소유자의 reselect 경로(abandon)가 막히지 않는다.
  assert.equal(
    await abandonBlobUpload({ assetId: 'IMG0AAAAAAAAAAAAAA', nonce: 'nonce-1', userId: 'U0OTHER', deps: world.abandonDeps }),
    true,
  );
});

test('finalize: lease kind와 mutation 종류가 다르면 rejected이고 claim이 해제된다', async () => {
  const world = createWorld();
  seedLease(world, 'FILE0AAAAAAAAAAAAA', { kind: 'file' });

  await assert.rejects(finalizeImage(world, 'FILE0AAAAAAAAAAAAA', 'nonce-1'), errorWithCode('asset_finalize_rejected'));

  assert.equal(world.state.leases.get('FILE0AAAAAAAAAAAAA')?.state, 'pending');
  assert.equal(world.state.persisted.length, 0);
});

test('finalize: modification 형식이 잘못되면 rejected이고 claim이 해제된다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');

  await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1', ['resize']), errorWithCode('asset_finalize_rejected'));

  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.state, 'pending');
  assert.equal(world.state.persisted.length, 0);
});

test('finalize: S3 metadata owner·Content-Type·size 불일치는 rejected이고 claim이 해제된다', async () => {
  const cases: UploadedObject[] = [
    { userId: 'U0INTRUDER', contentType: 'image/png', contentLength: 100, body: new Uint8Array([1]) },
    { userId: 'U0OWNER', contentType: 'image/jpeg', contentLength: 100, body: new Uint8Array([1]) },
    { userId: 'U0OWNER', contentType: 'image/png', contentLength: 101, body: new Uint8Array([1]) },
  ];

  for (const uploaded of cases) {
    const world = createWorld();
    seedLease(world, 'IMG0AAAAAAAAAAAAAA');
    world.state.uploadedObject = uploaded;

    await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1'), errorWithCode('asset_finalize_rejected'));

    assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.state, 'pending');
    assert.equal(world.state.persisted.length, 0);
  }
});

test('finalize: Content-Type 비교는 양쪽 정규화 값으로 한다', async () => {
  const world = createWorld();
  seedLease(world, 'FILE0AAAAAAAAAAAAA', { kind: 'file', format: 'application/octet-stream' });
  world.state.uploadedObject = { userId: 'U0OWNER', contentType: '', contentLength: 100, body: undefined };

  const row = await finalizeBlobUpload({
    assetId: 'FILE0AAAAAAAAAAAAA',
    nonce: 'nonce-1',
    kind: 'file',
    userId: 'U0OWNER',
    deps: world.finalizeDeps,
  });

  assert.equal(row.id, 'FILE0AAAAAAAAAAAAA');
});

test('finalize: 처리 helper에서 난 예외는 전부 retryable이고 lease를 건드리지 않는다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');
  world.state.persistError = new Error('sharp: unsupported image format');

  await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1'), errorWithCode('asset_finalize_retryable'));

  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.state, 'finalizing');
  assert.equal(world.state.published.length, 0);
});

test('finalize: 업로드 객체 읽기 실패도 retryable이고 lease를 건드리지 않는다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');
  world.state.readUploadedObjectError = new Error('NoSuchKey');

  await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1'), errorWithCode('asset_finalize_retryable'));

  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.state, 'finalizing');
});

test('finalize: 선점 시 고정된 modification을 쓰고, 뒤늦은 다른 modification은 무시한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { state: 'finalizing', modification: { resize: { width: 10 } } });

  await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1', { resize: { width: 9999 } });

  assert.deepEqual(world.state.persisted[0].modification, { resize: { width: 10 } });
});

test('finalize: 경합하는 두 run이 서로 다른 출력 키를 쓰고, 이긴 row는 자기 run의 경로를 기록한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');

  const winner = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { nonce: 'nonce-2' });
  hideCompletedOnce(world);
  const loser = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-2');

  assert.equal(world.state.outputKeys.length, 2);
  assert.notEqual(world.state.outputKeys[0], world.state.outputKeys[1]);
  assert.equal(world.state.persisted.at(-1)?.outputKey, `${world.state.outputKeys[1]}.png`);
  assert.equal(winner.path, `${world.state.outputKeys[0]}.png`);
  assert.equal(loser.path, winner.path);
});

test('finalize: 출력 키는 업로드 path와 무관한 새 키이지만 확장자는 그대로 물려받는다', async () => {
  const world = createWorld();
  seedLease(world, 'FILE0AAAAAAAAAAAAA', { kind: 'file', format: 'application/pdf', path: '26/07/a/ab/upload.pdf' });
  world.state.uploadedObject = { userId: 'U0OWNER', contentType: 'application/pdf', contentLength: 100, body: undefined };

  const row = await finalizeBlobUpload({
    assetId: 'FILE0AAAAAAAAAAAAA',
    nonce: 'nonce-1',
    kind: 'file',
    userId: 'U0OWNER',
    deps: world.finalizeDeps,
  });

  assert.equal(row.path, `${world.state.outputKeys[0]}.pdf`);
  assert.ok(!row.path.startsWith('26/07/a/ab/upload'));
});

test('finalize: 확장자 없는 업로드 path면 출력 키에도 확장자를 붙이지 않는다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { path: '26/07/a/ab/upload' });

  const row = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  assert.equal(row.path, world.state.outputKeys[0]);
});

test('finalize: insert가 PK 위반이면 기존 row를 싣고 lease DEL·무효화는 그대로 수행한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');
  await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1');

  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { nonce: 'nonce-2', documentId: 'DOC0OTHER' });
  hideCompletedOnce(world);
  world.state.published.length = 0;

  const row = await finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-2');

  assert.equal(row.path, `${world.state.outputKeys[0]}.png`);
  assert.equal(world.state.leases.size, 0);
  assert.deepEqual(world.state.published, [{ documentId: 'DOC0OTHER', ids: ['IMG0AAAAAAAAAAAAAA'] }]);
});

test('abandon: pending lease는 DEL 후 무효화를 publish하고 true를 반환한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');

  const result = await abandonBlobUpload({
    assetId: 'IMG0AAAAAAAAAAAAAA',
    nonce: 'nonce-1',
    userId: 'U0OWNER',
    deps: world.abandonDeps,
  });

  assert.equal(result, true);
  assert.equal(world.state.leases.size, 0);
  assert.deepEqual(world.state.published, [{ documentId: 'DOC0DOCUMENT', ids: ['IMG0AAAAAAAAAAAAAA'] }]);
});

test('abandon: finalizing·nonce 불일치·부재는 false이고 publish도 없다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA', { state: 'finalizing' });

  assert.equal(
    await abandonBlobUpload({ assetId: 'IMG0AAAAAAAAAAAAAA', nonce: 'nonce-1', userId: 'U0OWNER', deps: world.abandonDeps }),
    false,
  );
  assert.equal(
    await abandonBlobUpload({ assetId: 'IMG0AAAAAAAAAAAAAA', nonce: 'nonce-x', userId: 'U0OWNER', deps: world.abandonDeps }),
    false,
  );
  assert.equal(
    await abandonBlobUpload({ assetId: 'IMG0BBBBBBBBBBBBBB', nonce: 'nonce-1', userId: 'U0OWNER', deps: world.abandonDeps }),
    false,
  );

  assert.ok(world.state.leases.has('IMG0AAAAAAAAAAAAAA'));
  assert.equal(world.state.published.length, 0);
});

test('reselect: 검증 거부로 해제된 lease는 abandon 후에야 새 nonce 예약이 성공한다', async () => {
  const world = createWorld();
  seedLease(world, 'IMG0AAAAAAAAAAAAAA');
  world.state.uploadedObject = { userId: 'U0OWNER', contentType: 'image/jpeg', contentLength: 100, body: new Uint8Array([1]) };

  await assert.rejects(finalizeImage(world, 'IMG0AAAAAAAAAAAAAA', 'nonce-1'), errorWithCode('asset_finalize_rejected'));
  assert.equal(world.state.leases.get('IMG0AAAAAAAAAAAAAA')?.state, 'pending');

  await assert.rejects(reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]), errorWithCode('asset_upload_in_progress'));

  assert.equal(
    await abandonBlobUpload({ assetId: 'IMG0AAAAAAAAAAAAAA', nonce: 'nonce-1', userId: 'U0OWNER', deps: world.abandonDeps }),
    true,
  );

  const [reservation] = await reserve(world, [item('image', { assetId: 'IMG0AAAAAAAAAAAAAA' })]);
  assert.equal(reservation.assetId, 'IMG0AAAAAAAAAAAAAA');
  assert.notEqual(reservation.nonce, 'nonce-1');
});
