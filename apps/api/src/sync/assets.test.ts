import assert from 'node:assert/strict';
import test from 'node:test';
import { assetKindOf, buildAssetStateEntries, groupAssetIds, leaseKindOf, toLeaseItems, toLeaseMap } from './assets.ts';
import { DocumentChannel } from './channel.ts';
import { ASSET_STATE_MAX_FRAME_BYTES, chunkByEncodedBytes, SyncConnection } from './connection.ts';
import { decodeClientMessage, decodeRaw, encodeMessage } from './protocol.ts';
import { collectSend, FakeSyncDeps } from './testing.ts';
import type { AssetLeaseSummary } from './assets.ts';
import type { SyncSocket } from './connection.ts';
import type { AssetStateEntry, ClientMessage, ReadyAssetPayload, ServerMessage } from './protocol.ts';

class FakeSocket implements SyncSocket {
  sent: ServerMessage[] = [];
  closed: { code: number; reason?: string } | null = null;
  buffered = 0;
  send = async (data: Uint8Array): Promise<void> => {
    this.sent.push(decodeRaw(data) as ServerMessage);
  };
  close = (code: number, reason?: string): void => {
    this.closed ??= { code, reason };
  };
  bufferedAmount = (): number => this.buffered;
}

const setup = () => {
  const deps = new FakeSyncDeps();
  deps.tickets.set('TK', { sessionId: 'S1', userId: 'U1', deviceId: 'DEV1' });
  const socket = new FakeSocket();
  const connection = new SyncConnection({ deps, socket });
  const dispatch = (message: ClientMessage) => connection.handleMessage(encodeMessage(message));
  return { deps, socket, connection, dispatch };
};

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

const hello = async (d: ReturnType<typeof setup>) => {
  await d.dispatch({ t: 'hello', ticket: 'TK', clientId: 'me', capabilities: [] });
};

const attach = async (d: ReturnType<typeof setup>, documentId = 'D1') => {
  await d.dispatch({ t: 'attach', documentId });
  await tick();
};

const readyImage = (id: string): AssetStateEntry => ({
  id,
  state: 'ready',
  asset: {
    type: 'image',
    id,
    url: `https://typie.net/images/${id}.png`,
    originalUrl: `https://typie.net/original-images/${id}.png`,
    width: 100,
    height: 50,
    placeholder: 'ph',
  },
});

const pendingFile = (id: string, name = 'doc.pdf', size = 1234): AssetStateEntry => ({
  id,
  state: 'pending',
  meta: { kind: 'file', name, size },
});

/**
 * * 분류·조립 (leaf: db/redis/pubsub 없이 실행되는 실제 프로덕션 규칙)
 */

const IMAGE_ID = 'IMG0AAAAAAAAAAAAAA';
const FILE_ID = 'FILE0BBBBBBBBBBBBBB';
const EMBED_ID = 'EMBD0CCCCCCCCCCCCCC';
const ARCHIVED_ID = 'DAN0DDDDDDDDDDDDDD';

test('assetKindOf: 문서가 참조하는 네 종류를 분류하고 그 밖은 null이다', () => {
  assert.equal(assetKindOf(IMAGE_ID), 'image');
  assert.equal(assetKindOf(FILE_ID), 'file');
  assert.equal(assetKindOf(EMBED_ID), 'embed');
  assert.equal(assetKindOf(ARCHIVED_ID), 'archived');
  assert.equal(assetKindOf('D0EEEEEEEEEEEEEE'), null);
  assert.equal(assetKindOf('FONT0FFFFFFFFFFFFF'), null);
  assert.equal(assetKindOf(''), null);
  assert.equal(assetKindOf('garbage'), null);
});

test('leaseKindOf: lease를 갖는 건 image/file뿐 — embed·archived는 lease 경로로 새지 않는다', () => {
  assert.equal(leaseKindOf(IMAGE_ID), 'image');
  assert.equal(leaseKindOf(FILE_ID), 'file');
  assert.equal(leaseKindOf(EMBED_ID), null);
  assert.equal(leaseKindOf(ARCHIVED_ID), null);
});

test('assetKindOf: 길이가 다른 과거 id도 접두사로 분류된다 (isValidAssetId 아님)', () => {
  assert.equal(assetKindOf('IMG0SHORT'), 'image');
  assert.equal(assetKindOf('FILE0AAAAAAAAAAAAAAAAAA'), 'file');
});

test('groupAssetIds: 네 그룹으로 나누되 assetIds(lease MGET)는 image→file만 연결한다', () => {
  const grouped = groupAssetIds([FILE_ID, EMBED_ID, IMAGE_ID, ARCHIVED_ID, 'IMG0ZZZZZZZZZZZZZZ', 'FONT0FFFFFFFFFFFFF']);
  assert.deepEqual(grouped.imageIds, [IMAGE_ID, 'IMG0ZZZZZZZZZZZZZZ']);
  assert.deepEqual(grouped.fileIds, [FILE_ID]);
  assert.deepEqual(grouped.embedIds, [EMBED_ID]);
  assert.deepEqual(grouped.archivedIds, [ARCHIVED_ID]);
  assert.deepEqual(grouped.assetIds, [IMAGE_ID, 'IMG0ZZZZZZZZZZZZZZ', FILE_ID]);
});

test('toLeaseItems: lease가 없는 종류의 id는 lease 키로 만들지 않는다', () => {
  assert.deepEqual(
    toLeaseItems([
      { id: IMAGE_ID, nonce: 'n1' },
      { id: EMBED_ID, nonce: 'n2' },
      { id: FILE_ID, nonce: 'n3' },
    ]),
    [
      { assetId: IMAGE_ID, nonce: 'n1' },
      { assetId: FILE_ID, nonce: 'n3' },
    ],
  );
});

test('toLeaseMap: null을 건너뛰고 인덱스를 assetIds와 1:1로 맞춘다', () => {
  const map = toLeaseMap([IMAGE_ID, FILE_ID, 'IMG0ZZZZZZZZZZZZZZ'], [null, { name: 'b' }, { name: 'c' }]);
  assert.deepEqual(
    [...map.entries()],
    [
      [FILE_ID, { name: 'b' }],
      ['IMG0ZZZZZZZZZZZZZZ', { name: 'c' }],
    ],
  );
});

const readyPayload = (id: string): ReadyAssetPayload => ({ type: 'file', id, url: `https://typie.net/files/${id}`, name: 'f', size: 1 });
const leaseSummary: AssetLeaseSummary = { kind: 'file', name: 'uploading.pdf', size: 9 };

test('buildAssetStateEntries: completed row가 lease를 이긴다', () => {
  const entries = buildAssetStateEntries([FILE_ID], new Map([[FILE_ID, readyPayload(FILE_ID)]]), new Map([[FILE_ID, leaseSummary]]));
  assert.deepEqual(entries, [{ id: FILE_ID, state: 'ready', asset: readyPayload(FILE_ID) }]);
});

test('buildAssetStateEntries: lease만 있으면 pending(kind/name/size), 둘 다 없으면 missing', () => {
  const entries = buildAssetStateEntries([FILE_ID, IMAGE_ID], new Map(), new Map([[FILE_ID, leaseSummary]]));
  assert.deepEqual(entries, [
    { id: FILE_ID, state: 'pending', meta: { kind: 'file', name: 'uploading.pdf', size: 9 } },
    { id: IMAGE_ID, state: 'missing' },
  ]);
});

test('buildAssetStateEntries: embed·archived는 행이 있으면 ready, 없으면 곧바로 missing이다(pending 없음)', () => {
  const embedReady: ReadyAssetPayload = {
    type: 'embed',
    id: EMBED_ID,
    url: 'https://example.com/v',
    title: 't',
    description: null,
    thumbnailUrl: null,
    html: null,
  };
  const entries = buildAssetStateEntries([EMBED_ID, ARCHIVED_ID], new Map([[EMBED_ID, embedReady]]), new Map());
  assert.deepEqual(entries, [
    { id: EMBED_ID, state: 'ready', asset: embedReady },
    { id: ARCHIVED_ID, state: 'missing' },
  ]);
});

test('buildAssetStateEntries: embed·archived에 lease가 섞여 들어와도 pending으로 새지 않는다', () => {
  const entries = buildAssetStateEntries(
    [EMBED_ID, ARCHIVED_ID],
    new Map(),
    new Map([
      [EMBED_ID, leaseSummary],
      [ARCHIVED_ID, leaseSummary],
    ]),
  );
  assert.deepEqual(entries, [
    { id: EMBED_ID, state: 'missing' },
    { id: ARCHIVED_ID, state: 'missing' },
  ]);
});

test('buildAssetStateEntries: 아는 TableCode가 아니면 ready·lease가 주어져도 missing으로 고정된다', () => {
  const unknownId = 'FONT0FFFFFFFFFFFFF';
  const entries = buildAssetStateEntries(
    [unknownId],
    new Map([[unknownId, readyPayload(unknownId)]]),
    new Map([[unknownId, leaseSummary]]),
  );
  assert.deepEqual(entries, [{ id: unknownId, state: 'missing' }]);
});

test('buildAssetStateEntries: 요청 id마다 정확히 1건, 순서 보존', () => {
  const ids = [IMAGE_ID, EMBED_ID, FILE_ID];
  const entries = buildAssetStateEntries(ids, new Map(), new Map());
  assert.deepEqual(
    entries.map((entry) => entry.id),
    ids,
  );
});

/**
 * * decode
 */

test('decode: 새 client 메시지 3종이 왕복한다', () => {
  const messages: ClientMessage[] = [
    { t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01', 'FILE01'] },
    { t: 'asset-heartbeat', documentId: 'D1', items: [{ id: 'IMG01', nonce: 'n1' }] },
    { t: 'asset-failed', documentId: 'D1', items: [{ id: 'IMG01', nonce: 'n1' }] },
  ];
  for (const message of messages) {
    const result = decodeClientMessage(encodeMessage(message));
    assert.ok(result.ok, message.t);
    assert.deepEqual(result.message, message);
  }
});

test('decode: asset-pull은 documentId·requestId·string[] ids가 모두 있어야 한다', () => {
  const cases = [
    { t: 'asset-pull', documentId: 'D1', requestId: 'q1' },
    { t: 'asset-pull', documentId: 'D1', ids: ['IMG01'] },
    { t: 'asset-pull', requestId: 'q1', ids: ['IMG01'] },
    { t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: 'IMG01' },
    { t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: [1, 2] },
    { t: 'asset-pull', documentId: 'D1', requestId: 1, ids: ['IMG01'] },
  ];
  for (const message of cases) {
    assert.deepEqual(decodeClientMessage(encodeMessage(message as never)), { ok: false, reason: 'malformed' }, JSON.stringify(message));
  }
});

test('decode: asset-pull ids는 1~100건만 허용', () => {
  const empty = encodeMessage({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: [] });
  assert.deepEqual(decodeClientMessage(empty), { ok: false, reason: 'malformed' });

  const hundred = Array.from({ length: 100 }, (_, i) => `IMG0${i}`);
  assert.ok(decodeClientMessage(encodeMessage({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: hundred })).ok);

  const overflow = [...hundred, 'IMG0100'];
  assert.deepEqual(decodeClientMessage(encodeMessage({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: overflow })), {
    ok: false,
    reason: 'malformed',
  });
});

test('decode: heartbeat/failed items는 {id,nonce} 형식만 허용', () => {
  for (const t of ['asset-heartbeat', 'asset-failed'] as const) {
    const cases = [
      { t, documentId: 'D1', items: [{ id: 'IMG01' }] },
      { t, documentId: 'D1', items: [{ nonce: 'n1' }] },
      { t, documentId: 'D1', items: [{ id: 'IMG01', nonce: 2 }] },
      { t, documentId: 'D1', items: ['IMG01'] },
      { t, documentId: 'D1', items: [null] },
      { t, documentId: 'D1', items: {} },
      { t, documentId: 'D1' },
      { t, items: [{ id: 'IMG01', nonce: 'n1' }] },
      { t, documentId: 'D1', items: [] },
    ];
    for (const message of cases) {
      assert.deepEqual(
        decodeClientMessage(encodeMessage(message as never)),
        { ok: false, reason: 'malformed' },
        `${t} ${JSON.stringify(message)}`,
      );
    }
  }
});

test('decode: heartbeat/failed items도 1~100건만 허용', () => {
  const hundred = Array.from({ length: 100 }, (_, i) => ({ id: `IMG0${i}`, nonce: `n${i}` }));
  assert.ok(decodeClientMessage(encodeMessage({ t: 'asset-heartbeat', documentId: 'D1', items: hundred })).ok);
  const overflow = [...hundred, { id: 'IMG0100', nonce: 'n100' }];
  assert.deepEqual(decodeClientMessage(encodeMessage({ t: 'asset-failed', documentId: 'D1', items: overflow })), {
    ok: false,
    reason: 'malformed',
  });
});

test('decode: items의 모르는 추가 키는 무시된다 (전방 호환)', () => {
  const result = decodeClientMessage(
    encodeMessage({ t: 'asset-heartbeat', documentId: 'D1', items: [{ id: 'IMG01', nonce: 'n1', future: 'x' }] } as never),
  );
  assert.ok(result.ok);
  if (result.message.t !== 'asset-heartbeat') return assert.fail();
  assert.deepEqual(result.message.items, [{ id: 'IMG01', nonce: 'n1' }]);
});

test('decode: 모르는 t는 여전히 unknown으로 남는다 (회귀)', () => {
  assert.deepEqual(decodeClientMessage(encodeMessage({ t: 'asset-brand-new' } as never)), {
    ok: false,
    reason: 'unknown',
    type: 'asset-brand-new',
  });
});

const CONTRACT_ASSET_PULL_HEX =
  'b9000461746a61737365742d70756c6c6a646f63756d656e74496462443169726571756573744964627131636964738165494d473031';

test('클라이언트 계약 벡터: 고정 바이트가 asset-pull로 디코드된다', () => {
  const result = decodeClientMessage(Uint8Array.from(Buffer.from(CONTRACT_ASSET_PULL_HEX, 'hex')));
  assert.ok(result.ok);
  assert.deepEqual(result.message, { t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });
});

const CONTRACT_ASSET_STATE_HEX =
  'b9000561746b61737365742d73746174656a646f63756d656e744964624431697265717565737449646271316661737365747381b9000262696465494d473031657374617465676d697373696e676566696e616cf5';

test('서버 계약 벡터: asset-state 인코딩이 고정 바이트와 일치한다', () => {
  const encoded = encodeMessage({
    t: 'asset-state',
    documentId: 'D1',
    requestId: 'q1',
    assets: [{ id: 'IMG01', state: 'missing' }],
    final: true,
  });
  assert.equal(Buffer.from(encoded).toString('hex'), CONTRACT_ASSET_STATE_HEX);
});

const CONTRACT_ASSET_CHANGED_HEX = 'b9000361746d61737365742d6368616e6765646a646f63756d656e744964624431636964738165494d473031';

test('서버 계약 벡터: asset-changed 인코딩이 고정 바이트와 일치한다', () => {
  const encoded = encodeMessage({ t: 'asset-changed', documentId: 'D1', ids: ['IMG01'] });
  assert.equal(Buffer.from(encoded).toString('hex'), CONTRACT_ASSET_CHANGED_HEX);
});

/**
 * * asset-pull
 */

test('asset-pull: 미attach 문서는 조용히 무시된다 (에러도 close도 없음)', async () => {
  const d = setup();
  await hello(d);
  const before = d.socket.sent.length;
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });
  assert.equal(d.socket.sent.length, before);
  assert.equal(d.socket.closed, null);
  assert.equal(d.deps.resolveAssetStatesCalls.length, 0);
});

test('asset-pull: detach로 채널이 멈춘 뒤에도 조용히 무시된다', async () => {
  const d = setup();
  await hello(d);
  await attach(d);
  await d.dispatch({ t: 'detach', documentId: 'D1' });
  const before = d.socket.sent.length;
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });
  assert.equal(d.socket.sent.length, before);
  assert.equal(d.deps.resolveAssetStatesCalls.length, 0);
});

test('asset-pull: attach된 문서면 requestId를 반향한 asset-state로 응답한다', async () => {
  const d = setup();
  d.deps.assetStates.set('IMG01', readyImage('IMG01'));
  d.deps.assetStates.set('FILE01', pendingFile('FILE01'));
  await hello(d);
  await attach(d);
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q7', ids: ['IMG01', 'FILE01', 'EMBD01'] });

  const states = d.socket.sent.filter((m) => m.t === 'asset-state');
  assert.equal(states.length, 1);
  const state = states[0];
  if (state.t !== 'asset-state') return assert.fail();
  assert.equal(state.documentId, 'D1');
  assert.equal(state.requestId, 'q7');
  assert.equal(state.final, true);
  assert.deepEqual(state.assets, [readyImage('IMG01'), pendingFile('FILE01'), { id: 'EMBD01', state: 'missing' }]);
});

test('asset-pull: resolve 실패는 연결을 끊지 않고 요청을 미응답으로 남긴다', async () => {
  const d = setup();
  d.deps.resolveAssetStates = async () => {
    throw new Error('db down');
  };
  await hello(d);
  await attach(d);
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });

  assert.equal(d.socket.sent.filter((m) => m.t === 'asset-state').length, 0);
  assert.equal(
    d.socket.sent.some((m) => m.t === 'error'),
    false,
  );
  assert.equal(d.socket.closed, null);

  // 연결은 살아 있어야 한다 — 후속 메시지가 정상 처리된다.
  await d.dispatch({ t: 'ping' });
  assert.equal(d.socket.sent.at(-1)?.t, 'pong');
});

test('asset-pull: resolve 도중 채널이 멈추면(start 실패·reload) 응답하지 않는다', async () => {
  const d = setup();
  let releaseBundles!: (err: Error) => void;
  d.deps.readBundlesAfter = () => new Promise((_, reject) => (releaseBundles = reject));
  let releaseResolve!: () => void;
  const resolveGate = new Promise<void>((resolve) => (releaseResolve = resolve));
  const original = d.deps.resolveAssetStates;
  d.deps.resolveAssetStates = async (ids) => {
    await resolveGate;
    return original(ids);
  };

  await hello(d);
  await attach(d);
  const pull = d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });
  await tick();

  // 채널의 detached start()가 실패해 stop()된다 — 연결은 destroy되지 않는다.
  releaseBundles(new Error('bundle read failed'));
  await tick();
  releaseResolve();
  await pull;

  assert.equal(d.socket.sent.filter((m) => m.t === 'asset-state').length, 0);
  assert.equal(d.socket.closed, null);
});

test('asset-pull: 응답은 요청한 연결로만 간다', async () => {
  const a = setup();
  const b = { ...a, socket: new FakeSocket() } as ReturnType<typeof setup>;
  const other = new SyncConnection({ deps: a.deps, socket: b.socket });
  a.deps.tickets.set('TK2', { sessionId: 'S2', userId: 'U2', deviceId: 'DEV2' });
  await hello(a);
  await other.handleMessage(encodeMessage({ t: 'hello', ticket: 'TK2', clientId: 'other', capabilities: [] }));
  await attach(a);
  await other.handleMessage(encodeMessage({ t: 'attach', documentId: 'D1' }));
  await tick();

  await a.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });
  assert.equal(a.socket.sent.filter((m) => m.t === 'asset-state').length, 1);
  assert.equal(b.socket.sent.filter((m) => m.t === 'asset-state').length, 0);
  other.destroy();
});

test('asset 메시지도 hello 전에는 close 4003', async () => {
  for (const message of [
    { t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] },
    { t: 'asset-heartbeat', documentId: 'D1', items: [{ id: 'IMG01', nonce: 'n1' }] },
    { t: 'asset-failed', documentId: 'D1', items: [{ id: 'IMG01', nonce: 'n1' }] },
  ] as ClientMessage[]) {
    const d = setup();
    await d.dispatch(message);
    assert.equal(d.socket.closed?.code, 4003, message.t);
  }
});

test('asset-pull: 중복 id는 dedup 후 resolve된다', async () => {
  const d = setup();
  await hello(d);
  await attach(d);
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01', 'IMG01', 'FILE01', 'IMG01'] });
  assert.deepEqual(d.deps.resolveAssetStatesCalls, [['IMG01', 'FILE01']]);
});

test('asset-pull: 결과가 0건이어도 빈 배열 + final 프레임을 1회 보낸다', async () => {
  const d = setup();
  d.deps.resolveAssetStates = async () => [];
  await hello(d);
  await attach(d);
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['IMG01'] });

  const states = d.socket.sent.filter((m) => m.t === 'asset-state');
  assert.equal(states.length, 1);
  if (states[0].t !== 'asset-state') return assert.fail();
  assert.deepEqual(states[0].assets, []);
  assert.equal(states[0].final, true);
  assert.equal(states[0].requestId, 'q1');
});

test('asset-pull: 프레임 상한을 넘으면 분할되고 마지막 프레임만 final', async () => {
  const d = setup();
  const name = 'n'.repeat(400 * 1024);
  for (const id of ['FILE01', 'FILE02', 'FILE03']) {
    d.deps.assetStates.set(id, pendingFile(id, name));
  }
  await hello(d);
  await attach(d);
  await d.dispatch({ t: 'asset-pull', documentId: 'D1', requestId: 'q1', ids: ['FILE01', 'FILE02', 'FILE03'] });

  const states = d.socket.sent.filter((m) => m.t === 'asset-state');
  assert.equal(states.length, 2);
  assert.deepEqual(
    states.map((m) => (m.t === 'asset-state' ? m.final : null)),
    [false, true],
  );
  assert.deepEqual(
    states.map((m) => (m.t === 'asset-state' ? m.requestId : null)),
    ['q1', 'q1'],
  );
  assert.deepEqual(
    states.flatMap((m) => (m.t === 'asset-state' ? m.assets.map((a) => a.id) : [])),
    ['FILE01', 'FILE02', 'FILE03'],
  );
});

/**
 * * chunkByEncodedBytes
 */

const frame = { documentId: 'D1', requestId: 'q1' };

const encodedFrameBytes = (assets: AssetStateEntry[]): number =>
  encodeMessage({ t: 'asset-state', documentId: frame.documentId, requestId: frame.requestId, assets, final: true }).length;

test('chunkByEncodedBytes: 빈 입력은 빈 배열 1프레임(final)', () => {
  assert.deepEqual(chunkByEncodedBytes([], frame), [[[], true]]);
});

test('chunkByEncodedBytes: 상한 안이면 1프레임(final)', () => {
  const assets = [readyImage('IMG01'), pendingFile('FILE01')];
  assert.deepEqual(chunkByEncodedBytes(assets, frame), [[assets, true]]);
});

test('chunkByEncodedBytes: entry가 아니라 envelope 전체 바이트로 판단한다', () => {
  const assets = Array.from({ length: 4 }, (_, i) => readyImage(`IMG0${i}`));
  const twoEntryBytes = encodedFrameBytes(assets.slice(0, 2));

  // 정확히 2건짜리 envelope 크기를 상한으로 주면 2건씩 들어간다.
  const exact = chunkByEncodedBytes(assets, frame, twoEntryBytes);
  assert.deepEqual(
    exact.map(([chunk]) => chunk.length),
    [2, 2],
  );

  // 1바이트만 줄이면 2건이 못 들어간다 — envelope를 빼먹었다면 이 경계가 어긋난다.
  const tighter = chunkByEncodedBytes(assets, frame, twoEntryBytes - 1);
  assert.deepEqual(
    tighter.map(([chunk]) => chunk.length),
    [1, 1, 1, 1],
  );
});

test('chunkByEncodedBytes: 각 프레임은 상한을 넘지 않고 순서·항목이 보존되며 마지막만 final', () => {
  const assets = Array.from({ length: 7 }, (_, i) => readyImage(`IMG0${i}`));
  const limit = encodedFrameBytes(assets.slice(0, 3));
  const chunks = chunkByEncodedBytes(assets, frame, limit);

  assert.deepEqual(
    chunks.map(([, final]) => final),
    [false, false, true],
  );
  for (const [chunk] of chunks) {
    assert.ok(encodedFrameBytes(chunk) <= limit);
  }
  assert.deepEqual(
    chunks.flatMap(([chunk]) => chunk),
    assets,
  );
});

test('chunkByEncodedBytes: 단일 entry가 상한을 넘으면 그 entry만 담은 프레임으로 내보낸다', () => {
  const huge = pendingFile('FILE09', 'n'.repeat(4096));
  const assets = [readyImage('IMG01'), huge, readyImage('IMG02')];
  const limit = encodedFrameBytes([readyImage('IMG01')]);
  const chunks = chunkByEncodedBytes(assets, frame, limit);

  assert.deepEqual(
    chunks.map(([chunk]) => chunk),
    [[readyImage('IMG01')], [huge], [readyImage('IMG02')]],
  );
  assert.deepEqual(
    chunks.map(([, final]) => final),
    [false, false, true],
  );
});

test('chunkByEncodedBytes: 상한 초과 단일 entry가 마지막이어도 final은 그 프레임에 붙는다', () => {
  const huge = pendingFile('FILE09', 'n'.repeat(4096));
  const chunks = chunkByEncodedBytes([readyImage('IMG01'), huge], frame, encodedFrameBytes([readyImage('IMG01')]));
  assert.deepEqual(
    chunks.map(([chunk, final]) => [chunk.map((a) => a.id), final]),
    [
      [['IMG01'], false],
      [['FILE09'], true],
    ],
  );
});

test('chunkByEncodedBytes: 기본 상한은 ASSET_STATE_MAX_FRAME_BYTES', () => {
  const assets = Array.from({ length: 3 }, (_, i) => pendingFile(`FILE0${i}`, 'n'.repeat(400 * 1024)));
  assert.deepEqual(
    chunkByEncodedBytes(assets, frame).map(([chunk]) => chunk.length),
    chunkByEncodedBytes(assets, frame, ASSET_STATE_MAX_FRAME_BYTES).map(([chunk]) => chunk.length),
  );
  assert.equal(ASSET_STATE_MAX_FRAME_BYTES, 1024 * 1024);
});

/**
 * * asset-heartbeat / asset-failed
 */

test('asset-heartbeat: nonce를 포함한 items와 session userId로 extendAssetLeases 호출', async () => {
  const d = setup();
  await hello(d);
  const items = [
    { id: 'IMG01', nonce: 'n1' },
    { id: 'FILE01', nonce: 'n2' },
  ];
  await d.dispatch({ t: 'asset-heartbeat', documentId: 'D1', items });
  assert.deepEqual(d.deps.extendAssetLeaseCalls, [{ items, userId: 'U1' }]);
  assert.equal(d.socket.closed, null);
});

test('asset-failed: clearAssetLeases 후 정리된 (assetId, documentId)마다 무효화를 발행한다', async () => {
  const d = setup();
  d.deps.assetLeaseOwners.set('IMG01', { documentId: 'D1', nonce: 'n1', userId: 'U1' });
  d.deps.assetLeaseOwners.set('FILE01', { documentId: 'D2', nonce: 'n2', userId: 'U1' });
  await hello(d);
  const items = [
    { id: 'IMG01', nonce: 'n1' },
    { id: 'FILE01', nonce: 'n2' },
  ];
  await d.dispatch({ t: 'asset-failed', documentId: 'D1', items });

  assert.deepEqual(d.deps.clearAssetLeaseCalls, [{ items, userId: 'U1' }]);
  assert.deepEqual(d.deps.assetChangesPublished, [
    { documentId: 'D1', ids: ['IMG01'] },
    { documentId: 'D2', ids: ['FILE01'] },
  ]);
});

test('asset-failed: nonce가 맞지 않으면 정리도 무효화도 없다', async () => {
  const d = setup();
  d.deps.assetLeaseOwners.set('IMG01', { documentId: 'D1', nonce: 'n1', userId: 'U1' });
  await hello(d);
  await d.dispatch({ t: 'asset-failed', documentId: 'D1', items: [{ id: 'IMG01', nonce: 'other' }] });
  assert.deepEqual(d.deps.assetChangesPublished, []);
  assert.equal(d.deps.assetLeaseOwners.has('IMG01'), true);
});

/**
 * * channel: 무효화 push
 */

const makeChannel = (deps: FakeSyncDeps, documentId = 'D1', clientId = 'c1') => {
  const { sent, send } = collectSend();
  return { sent, channel: new DocumentChannel({ deps, send, documentId, clientId }) };
};

test('channel: asset 이벤트의 ids를 그대로 asset-changed로 전달한다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  deps.emitAssetEvent('D1', ['IMG01', 'FILE01']);
  await tick();

  const changed = sent.filter((m) => m.t === 'asset-changed');
  assert.equal(changed.length, 1);
  if (changed[0].t !== 'asset-changed') return assert.fail();
  assert.equal(changed[0].documentId, 'D1');
  assert.deepEqual(changed[0].ids, ['IMG01', 'FILE01']);
  channel.stop();
});

test('channel: loading phase 중 도착한 무효화도 버퍼링 없이 즉시 전달된다', async () => {
  const deps = new FakeSyncDeps();
  deps.seedBundles('D1', [{ id: 'B1', seq: 1, payload: Uint8Array.of(1) }]);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  const original = deps.readBundlesAfter;
  deps.readBundlesAfter = async (documentId, afterSeq, limit) => {
    await gate;
    return original(documentId, afterSeq, limit);
  };
  const { sent, channel } = makeChannel(deps);
  const started = channel.start();
  await tick();
  deps.emitAssetEvent('D1', ['IMG01']);
  await tick();

  assert.equal(
    sent.some((m) => m.t === 'asset-changed'),
    true,
  );
  assert.equal(
    sent.some((m) => m.t === 'snapshot-end'),
    false,
  );
  release();
  await started;
  channel.stop();
});

test('channel: 다른 클라이언트 발 무효화도 echo 필터 없이 전달된다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps, 'D1', 'me');
  await channel.start();
  deps.publishAssetChanged('D1', ['IMG01']);
  await tick();
  assert.equal(sent.filter((m) => m.t === 'asset-changed').length, 1);
  channel.stop();
});

test('channel: stop이 asset 구독도 해제하고 이후 이벤트는 전달되지 않는다', async () => {
  const deps = new FakeSyncDeps();
  const { sent, channel } = makeChannel(deps);
  await channel.start();
  assert.equal(deps.assetSubscriberCount('D1'), 1);
  channel.stop();
  assert.equal(deps.assetSubscriberCount('D1'), 0);

  const before = sent.length;
  deps.emitAssetEvent('D1', ['IMG01']);
  await tick();
  assert.equal(sent.length, before);
});

test('connection: destroy가 asset 구독까지 정리한다', async () => {
  const d = setup();
  await hello(d);
  await attach(d);
  assert.equal(d.deps.assetSubscriberCount('D1'), 1);
  d.connection.destroy();
  assert.equal(d.deps.assetSubscriberCount('D1'), 0);
});

test('channel: 무효화는 attach된 문서 채널로만 나간다', async () => {
  const deps = new FakeSyncDeps();
  const a = makeChannel(deps, 'D1');
  const b = makeChannel(deps, 'D2');
  await a.channel.start();
  await b.channel.start();
  deps.emitAssetEvent('D1', ['IMG01']);
  await tick();
  assert.equal(a.sent.filter((m) => m.t === 'asset-changed').length, 1);
  assert.equal(b.sent.filter((m) => m.t === 'asset-changed').length, 0);
  a.channel.stop();
  b.channel.stop();
});
