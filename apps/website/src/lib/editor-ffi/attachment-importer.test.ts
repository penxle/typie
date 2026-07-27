import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ASSET_HEARTBEAT_INTERVAL_MS, EditorAttachmentImporter } from './attachment-importer';
import type { AttachmentPlaceholderKind, EditorEvent, InputModifiers, Message } from '@typie/editor-ffi/browser';
import type { ReadyAssetPayload } from '$lib/sync/protocol';
import type { AttachmentImporterDeps, AttachmentImporterOptions, ReserveUploadItem, UploadReservation } from './attachment-importer';
import type { AssetResolution, LocalUpload } from './types';

type Receipt = Extract<EditorEvent, { type: 'attachment_placeholders_inserted' }>;
type ReceiptListener = (editor: FakeEditor, event: Receipt) => void;

const finalizedImage = (id: string) => ({
  id,
  url: `https://example.com/${id}.webp`,
  originalUrl: `https://example.com/${id}.png`,
  width: 640,
  height: 480,
  placeholder: 'preview',
});

const imageAsset = (id: string): ReadyAssetPayload => ({
  type: 'image' as const,
  id,
  url: `https://example.com/${id}.webp`,
  originalUrl: `https://example.com/${id}.png`,
  width: 640,
  height: 480,
  placeholder: 'preview',
});

// finalize 뮤테이션은 BigInt를 문자열로 준다. importer가 캐시에 담을 때 숫자로 정규화한다.
const finalizedFile = (id: string, name = `${id}.pdf`) => ({ id, name, size: '1024', url: `https://example.com/${name}` });

const fileAsset = (id: string, name = `${id}.pdf`): ReadyAssetPayload => ({
  type: 'file' as const,
  id,
  name,
  size: 1024,
  url: `https://example.com/${name}`,
});

const file = (name: string, type = 'application/octet-stream'): File => new File([name], name, { type });

const external = (node: string, kind: AttachmentPlaceholderKind, id?: string) => ({
  page_idx: 0,
  node,
  bounds: { x: 0, y: 0, width: 100, height: 100 },
  is_selected: false,
  data: kind === 'image' ? { type: 'image' as const, id, proportion: 100 } : { type: 'file' as const, id },
});

const coded = (code: string): Error => Object.assign(new Error(code), { code });

const reservationOf = (assetId: string, nonce: string, format = 'application/octet-stream'): UploadReservation => ({
  assetId,
  nonce,
  path: `26/07/a/ab/${assetId}`,
  uploadUrl: 'https://uploads.example/bucket',
  uploadFields: { key: 'k', 'Content-Type': format },
});

class FakeEditor {
  #receiptListeners = new Set<ReceiptListener>();
  destroyed = false;
  readOnly = false;
  externalElements: ReturnType<typeof external>[] = [];
  localUploads = new Map<string, LocalUpload>();
  assets = new Map<string, ReadyAssetPayload>();
  resolutions = new Map<string, AssetResolution>();
  messages: Message[] = [];
  flushLog: string[] = [];
  focus = vi.fn();
  flushImpl: () => void = vi.fn();

  #writeId(nodeId: string, assetId: string | undefined): void {
    const element = this.externalElements.find(({ node }) => node === nodeId);
    if (element) element.data.id = assetId;
  }

  on(event: EditorEvent['type'], listener: ReceiptListener): () => void {
    if (event !== 'attachment_placeholders_inserted') throw new Error(`Unexpected event: ${event}`);
    this.#receiptListeners.add(listener);
    return () => this.#receiptListeners.delete(listener);
  }

  enqueue(message: Message): void {
    this.messages.push(message);
    if (message.type !== 'node') return;

    const { op } = message;
    if (op.type === 'set_attr' && op.attr.type === 'image' && op.attr.attr.type === 'id') {
      this.#writeId(op.id, op.attr.attr.value);
    } else if (op.type === 'set_attrs' && op.attrs.type === 'file') {
      this.#writeId(op.id, op.attrs.id);
    } else if (op.type === 'delete') {
      this.externalElements = this.externalElements.filter(({ node }) => node !== op.id);
    }
  }

  flush(): void {
    this.flushLog.push(this.messages.at(-1)?.type ?? 'empty');
    this.flushImpl();
  }

  emitReceipt(requestId: string, nodeIds: string[]): void {
    const event: Receipt = { type: 'attachment_placeholders_inserted', request_id: requestId, node_ids: nodeIds };
    for (const listener of this.#receiptListeners) listener(this, event);
  }

  get receiptListenerCount(): number {
    return this.#receiptListeners.size;
  }

  asset(id: string | null | undefined, type: ReadyAssetPayload['type']) {
    if (!id) return;
    const asset = this.assets.get(id);
    return asset?.type === type ? asset : undefined;
  }
}

type FakeContext = {
  editor?: FakeEditor;
  documentId: string | null;
  assetRefresh: ((ids: string[]) => void) | null;
};

type FakeInterval = { ms: number; tick: () => void; stopped: boolean };
type FakeTimeout = FakeInterval;

const createDeps = () => {
  let assetSeq = 0;
  let nonceSeq = 0;
  const intervals: FakeInterval[] = [];
  const timeouts: FakeTimeout[] = [];

  const deps = {
    getImageDimensions: vi.fn<AttachmentImporterDeps['getImageDimensions']>().mockResolvedValue({ width: 320, height: 240 }),
    reserveUploads: vi.fn<AttachmentImporterDeps['reserveUploads']>((_documentId, items: ReserveUploadItem[]) =>
      Promise.resolve(
        items.map((item) =>
          reservationOf(item.assetId ?? `${item.kind === 'image' ? 'IMG' : 'FILE'}${++assetSeq}`, `n${++nonceSeq}`, item.format),
        ),
      ),
    ),
    transferToReservation: vi.fn<AttachmentImporterDeps['transferToReservation']>(),
    finalizeImage: vi.fn<AttachmentImporterDeps['finalizeImage']>((assetId) => Promise.resolve(finalizedImage(assetId))),
    finalizeFile: vi.fn<AttachmentImporterDeps['finalizeFile']>((assetId) => Promise.resolve(finalizedFile(assetId))),
    abandonUpload: vi.fn<AttachmentImporterDeps['abandonUpload']>().mockResolvedValue(true),
    sendAssetFailed: vi.fn<AttachmentImporterDeps['sendAssetFailed']>(),
    sendAssetHeartbeat: vi.fn<AttachmentImporterDeps['sendAssetHeartbeat']>(),
    startInterval: vi.fn<AttachmentImporterDeps['startInterval']>((ms, tick) => {
      const entry: FakeInterval = { ms, tick, stopped: false };
      intervals.push(entry);
      return () => (entry.stopped = true);
    }),
    startTimeout: vi.fn<AttachmentImporterDeps['startTimeout']>((ms, tick) => {
      const entry: FakeTimeout = { ms, tick, stopped: false };
      timeouts.push(entry);
      return () => (entry.stopped = true);
    }),
    delay: vi.fn<AttachmentImporterDeps['delay']>(),
  };

  return { deps, intervals, timeouts };
};

const createImporter = (options?: AttachmentImporterOptions) => {
  const editor = new FakeEditor();
  const ctx: FakeContext = { editor, documentId: 'DOC1', assetRefresh: null };
  const { deps, intervals, timeouts } = createDeps();
  const importer = new EditorAttachmentImporter(ctx as never, deps as unknown as AttachmentImporterDeps, options);
  return { ctx, editor, deps, intervals, timeouts, importer };
};

const pendingRequest = (editor: FakeEditor): { requestId: string; kinds: AttachmentPlaceholderKind[] } | undefined => {
  const message = editor.messages.at(-1);
  if (message?.type === 'insertion' && message.op.type === 'attachment_placeholders') {
    return { requestId: message.op.request_id, kinds: message.op.kinds };
  }
  if (message?.type === 'dnd' && message.op.type === 'drop' && message.op.payload.type === 'files') {
    return { requestId: message.op.payload.request_id, kinds: message.op.payload.kinds };
  }
  return undefined;
};

const latestRequestId = (editor: FakeEditor): { requestId: string; kinds: AttachmentPlaceholderKind[] } => {
  const request = pendingRequest(editor);
  if (!request) throw new Error('Expected an attachment placeholder request');
  return request;
};

const installReceipt = (editor: FakeEditor, nodeIds: string[]): void => {
  let emitted = false;
  editor.flushImpl = () => {
    // 엔진은 placeholder 요청을 실은 flush에만 수신증을 낸다 — 다른 flush(예: dnd leave)에 반응하면 안 된다.
    const request = pendingRequest(editor);
    if (emitted || !request) return;
    emitted = true;
    for (const [index, nodeId] of nodeIds.entries()) {
      const kind = request.kinds[index];
      if (!kind) throw new Error(`Missing kind for ${nodeId}`);
      editor.externalElements.push(external(nodeId, kind));
    }
    editor.emitReceipt(request.requestId, nodeIds);
  };
};

const nodeMessages = (editor: FakeEditor): Message[] => editor.messages.filter((message) => message.type === 'node');

const dndOps = (editor: FakeEditor): string[] =>
  editor.messages.filter((message) => message.type === 'dnd').map((message) => (message.type === 'dnd' ? message.op.type : ''));

const droppedReuseNodeId = (editor: FakeEditor): string | undefined => {
  for (const message of editor.messages) {
    if (message.type !== 'dnd' || message.op.type !== 'drop' || message.op.payload.type !== 'files') continue;
    return message.op.payload.reuse_node_id;
  }
  return undefined;
};

// 모든 fake dep이 마이크로태스크로 즉시 결정되므로, 매크로태스크 경계 한 번이면 파이프라인이 전부 소진된다.
const settle = async (rounds = 3): Promise<void> => {
  for (let round = 0; round < rounds; round++) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
};

let createObjectURL: ReturnType<typeof vi.fn>;
let revokeObjectURL: ReturnType<typeof vi.fn>;

beforeEach(() => {
  createObjectURL = vi.fn((input: File) => `blob:${input.name}`);
  revokeObjectURL = vi.fn();
  Object.defineProperties(URL, {
    createObjectURL: { configurable: true, writable: true, value: createObjectURL },
    revokeObjectURL: { configurable: true, writable: true, value: revokeObjectURL },
  });
});

describe('reserve before any document mutation', () => {
  it('fills one empty placeholder with a SetAttr flush only, and enqueues nothing on completion', async () => {
    const { editor, deps, importer } = createImporter();
    const picture = file('cover.png', 'image/png');
    editor.externalElements.push(external('existing', 'image'));
    let messagesAtReserve = -1;
    deps.reserveUploads.mockImplementation(async (_documentId, items) => {
      messagesAtReserve = editor.messages.length;
      return items.map((item) => reservationOf('IMG01', 'nonce-1', item.format));
    });

    expect(importer.importAtSelection([{ file: picture, kind: 'image' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(true);
    await settle();

    expect(messagesAtReserve).toBe(0);
    expect(deps.reserveUploads).toHaveBeenCalledExactlyOnceWith('DOC1', [
      { kind: 'image', name: 'cover.png', format: 'image/png', size: picture.size, assetId: undefined },
    ]);
    expect(editor.messages).toEqual([
      { type: 'node', op: { type: 'set_attr', id: 'existing', attr: { type: 'image', attr: { type: 'id', value: 'IMG01' } } } },
    ]);
    expect(editor.flushLog).toEqual(['node']);
    expect(deps.transferToReservation).toHaveBeenCalledOnce();
    expect(deps.finalizeImage).toHaveBeenCalledExactlyOnceWith('IMG01', 'nonce-1');
    expect(editor.assets.get('IMG01')).toEqual(imageAsset('IMG01'));
    expect(editor.localUploads.size).toBe(0);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:cover.png');
  });

  it('writes a file ID through set_attrs and caches the asset on the shared context', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    expect(nodeMessages(editor)).toEqual([
      { type: 'node', op: { type: 'set_attrs', id: 'existing', attrs: { type: 'file', id: 'FILE1' } } },
    ]);
    expect(editor.assets.get('FILE1')).toEqual(fileAsset('FILE1'));
    expect(deps.finalizeFile).toHaveBeenCalledOnce();
    expect(createObjectURL).not.toHaveBeenCalled();
  });

  it('inserts placeholders in one flush and records each reserved ID in a later flush', async () => {
    const { editor, deps, importer } = createImporter();
    installReceipt(editor, ['node-a', 'node-b']);
    let messagesAtReserve = -1;
    deps.reserveUploads.mockImplementation(async (_documentId, items) => {
      messagesAtReserve = editor.messages.length;
      return items.map((item, index) => reservationOf(`IMG0${index + 1}`, `n${index + 1}`, item.format));
    });

    expect(
      importer.importAtSelection(
        [
          { file: file('a.png', 'image/png'), kind: 'image' },
          { file: file('b.png', 'image/png'), kind: 'image' },
        ],
        { onFailure: vi.fn() },
      ),
    ).toBe(true);
    await settle();

    // 수신증(node_ids)은 삽입 트랜잭션이 커밋된 뒤에 나오므로 "한 flush에 삽입+SetAttr"은 불가능하다.
    expect(messagesAtReserve).toBe(0);
    expect(editor.messages[0]).toMatchObject({ type: 'insertion', op: { type: 'attachment_placeholders', kinds: ['image', 'image'] } });
    expect(editor.flushLog).toEqual(['insertion', 'node', 'node']);
    expect(nodeMessages(editor)).toEqual([
      { type: 'node', op: { type: 'set_attr', id: 'node-a', attr: { type: 'image', attr: { type: 'id', value: 'IMG01' } } } },
      { type: 'node', op: { type: 'set_attr', id: 'node-b', attr: { type: 'image', attr: { type: 'id', value: 'IMG02' } } } },
    ]);
    expect(editor.receiptListenerCount).toBe(0);
  });

  it('keeps the document untouched when the reservation fails and reports every item', async () => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    installReceipt(editor, ['node-a', 'node-b']);
    deps.reserveUploads.mockRejectedValue(coded('asset_upload_in_progress'));
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());

    const items = [
      { file: file('a.png', 'image/png'), kind: 'image' as const },
      { file: file('b.pdf'), kind: 'file' as const },
    ];
    expect(importer.importAtSelection(items, { onFailure })).toBe(true);
    await settle();

    expect(editor.messages).toEqual([]);
    expect(editor.flushLog).toEqual([]);
    expect(editor.localUploads.size).toBe(0);
    expect(deps.transferToReservation).not.toHaveBeenCalled();
    expect(onFailure.mock.calls.map(([item]) => item)).toEqual(items);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:a.png');
    expect(consoleError).toHaveBeenCalledOnce();
  });

  it('reserves an existing asset ID with a new nonce and enqueues nothing when the node already has one', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('node-a', 'image', 'IMG09'));

    expect(
      importer.reselect({ assetId: 'IMG09', nodeId: 'node-a', kind: 'image', file: file('new.png', 'image/png'), onFailure: vi.fn() }),
    ).toBe(true);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledExactlyOnceWith('DOC1', [
      { kind: 'image', name: 'new.png', format: 'image/png', size: expect.any(Number), assetId: 'IMG09' },
    ]);
    expect(editor.messages).toEqual([]);
    expect(editor.flushLog).toEqual([]);
    expect(editor.assets.has('IMG09')).toBe(true);
  });

  it('refuses a second import for the same placeholder while its reservation is still in flight', async () => {
    const { editor, deps, importer } = createImporter();
    const reserve = Promise.withResolvers<UploadReservation[]>();
    editor.externalElements.push(external('existing', 'file'));
    deps.reserveUploads.mockReturnValueOnce(reserve.promise);

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    // 노드는 아직 비어 있지만(ID 미기록) 두 번째 import가 같은 노드를 잡으면 예약 두 개가 한 노드를 덮어쓴다.
    expect(importer.importAtSelection([{ file: file('b.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      false,
    );
    expect(importer.canReusePlaceholder('existing', 'file')).toBe(false);

    reserve.resolve([reservationOf('FILE1', 'n1')]);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledOnce();
    expect(nodeMessages(editor)).toHaveLength(1);
  });

  it('reserves a new import immediately even while five transfers occupy the transfer limit', async () => {
    const { editor, deps, importer } = createImporter();
    const transfers = Array.from({ length: 5 }, () => Promise.withResolvers<undefined>());
    let index = 0;
    installReceipt(
      editor,
      Array.from({ length: 5 }, (_, i) => `busy-${i}`),
    );
    deps.transferToReservation.mockImplementation(() => transfers[index++]?.promise ?? Promise.resolve(undefined));

    importer.importAtSelection(
      Array.from({ length: 5 }, (_, i) => ({ file: file(`${i}.png`, 'image/png'), kind: 'image' as const })),
      { onFailure: vi.fn() },
    );
    await settle();
    expect(deps.transferToReservation).toHaveBeenCalledTimes(5);

    // 검증(미리보기 디코드)이 전송과 상한을 공유하면, 여기서 예약조차 못 해 노드도 스피너도 토스트도 없다.
    editor.externalElements.push(external('late', 'image'));
    deps.reserveUploads.mockClear();
    expect(
      importer.importAtSelection([{ file: file('late.png', 'image/png'), kind: 'image' }], { existingNodeId: 'late', onFailure: vi.fn() }),
    ).toBe(true);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledOnce();
    expect(editor.localUploads.get('IMG6')?.phase).toBe('active');

    for (const transfer of transfers) transfer.resolve(undefined);
    await settle();
  });

  it('releases the destination claim when the reservation fails so the placeholder stays usable', async () => {
    const { editor, deps, importer } = createImporter();
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    editor.externalElements.push(external('existing', 'file'));
    deps.reserveUploads.mockRejectedValueOnce(coded('validation_error'));

    expect(importer.importAtSelection([{ file: file('big.bin'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    expect(importer.canReusePlaceholder('existing', 'file')).toBe(true);
    expect(
      importer.importAtSelection([{ file: file('small.bin'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() }),
    ).toBe(true);
    await settle();

    expect(nodeMessages(editor)).toEqual([
      { type: 'node', op: { type: 'set_attrs', id: 'existing', attrs: { type: 'file', id: 'FILE1' } } },
    ]);
  });

  it('revokes the head preview and reports every item when the tail receipt is unusable', async () => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    const head = file('head.png', 'image/png');
    const tail = file('tail.png', 'image/png');
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    editor.externalElements.push(external('existing', 'image'));
    editor.flushImpl = () => {
      const { requestId } = latestRequestId(editor);
      editor.emitReceipt(requestId, ['one', 'two']);
    };

    expect(
      importer.importAtSelection(
        [
          { file: head, kind: 'image' },
          { file: tail, kind: 'image' },
        ],
        { existingNodeId: 'existing', onFailure },
      ),
    ).toBe(true);
    await settle();

    expect(revokeObjectURL).toHaveBeenCalledWith('blob:head.png');
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:tail.png');
    expect(onFailure.mock.calls.map(([item]) => item.file)).toEqual([head, tail]);
    expect(nodeMessages(editor)).toEqual([]);
    expect(deps.transferToReservation).not.toHaveBeenCalled();
  });

  it('chunks a reservation larger than the server item cap instead of failing the batch', async () => {
    const { editor, importer, deps } = createImporter({ maxReserveItems: 2 });
    installReceipt(
      editor,
      ['n0', 'n1', 'n2', 'n3', 'n4'].map((id) => id),
    );

    expect(
      importer.importAtSelection(
        Array.from({ length: 5 }, (_, index) => ({ file: file(`${index}.pdf`), kind: 'file' as const })),
        { onFailure: vi.fn() },
      ),
    ).toBe(true);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledTimes(3);
    expect(deps.reserveUploads.mock.calls.map(([, items]) => items.length)).toEqual([2, 2, 1]);
    expect(nodeMessages(editor)).toHaveLength(5);
  });
});

describe('drop import', () => {
  it('keeps the drop receipt path and writes the reserved IDs after the reservation succeeds', async () => {
    const { editor, deps, importer } = createImporter();
    const modifiers: InputModifiers = { ctrl: true };
    editor.externalElements.push(external('candidate', 'image'));
    installReceipt(editor, ['candidate', 'created']);
    let messagesAtReserve = -1;
    deps.reserveUploads.mockImplementation(async (_documentId, items) => {
      messagesAtReserve = editor.messages.length;
      return items.map((item, index) => reservationOf(`IMG0${index}`, `n${index}`, item.format));
    });

    expect(
      importer.importAtDrop(
        [
          { file: file('a.png', 'image/png'), kind: 'image' },
          { file: file('b.png', 'image/png'), kind: 'image' },
        ],
        { page: 3, x: 12.5, y: 42, modifiers, reuseNodeId: 'candidate', onFailure: vi.fn() },
      ),
    ).toBe(true);
    await settle();

    expect(messagesAtReserve).toBe(0);
    expect(editor.messages[0]).toMatchObject({
      type: 'dnd',
      op: {
        type: 'drop',
        page: 3,
        x: 12.5,
        y: 42,
        modifiers,
        payload: { type: 'files', kinds: ['image', 'image'], reuse_node_id: 'candidate' },
      },
    });
    expect(editor.flushLog).toEqual(['dnd', 'node', 'node']);
    expect(nodeMessages(editor).map((message) => (message.type === 'node' ? message.op.id : ''))).toEqual(['candidate', 'created']);
  });

  it('drops the batch without SetAttr when the receipt does not match the request', async () => {
    const { editor, deps, importer } = createImporter();
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    editor.flushImpl = () => {
      const { requestId } = latestRequestId(editor);
      editor.emitReceipt(requestId, ['only-one']);
    };

    expect(
      importer.importAtDrop(
        [
          { file: file('a.pdf'), kind: 'file' },
          { file: file('b.pdf'), kind: 'file' },
        ],
        { page: 0, x: 0, y: 0, modifiers: {}, onFailure: vi.fn() },
      ),
    ).toBe(true);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledOnce();
    expect(nodeMessages(editor)).toEqual([]);
    expect(deps.transferToReservation).not.toHaveBeenCalled();
    expect(editor.localUploads.size).toBe(0);
  });

  it.each([
    ['reservation failure', (deps: ReturnType<typeof createDeps>['deps']) => deps.reserveUploads.mockRejectedValue(coded('forbidden'))],
    [
      'every item failing validation',
      (deps: ReturnType<typeof createDeps>['deps']) => deps.getImageDimensions.mockRejectedValue(new Error('decode failed')),
    ],
  ])('clears the engine drag state when the drop is abandoned after %s', async (_name, configure) => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    configure(deps);

    expect(
      importer.importAtDrop([{ file: file('a.png', 'image/png'), kind: 'image' }], { page: 0, x: 0, y: 0, modifiers: {}, onFailure }),
    ).toBe(true);
    await settle();

    // `drop`을 보내지 않고 끝내면 엔진은 ExternalDnd + 살아 있는 drop_target으로 남아 인디케이터가 계속 그려진다.
    expect(dndOps(editor)).toEqual(['leave']);
    expect(editor.flushLog).toEqual(['dnd']);
    expect(nodeMessages(editor)).toEqual([]);
    expect(onFailure).toHaveBeenCalledOnce();
  });

  it('rejects a second drop that lands while the first is still awaiting its reservation', async () => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    const reserve = Promise.withResolvers<UploadReservation[]>();
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    installReceipt(editor, ['node-a']);
    deps.reserveUploads.mockReturnValueOnce(reserve.promise);

    expect(importer.importAtDrop([{ file: file('a.pdf'), kind: 'file' }], { page: 0, x: 0, y: 0, modifiers: {}, onFailure })).toBe(true);
    await settle();

    // 두 번째 드롭을 그대로 받으면 그 `over`가 첫 드롭의 목표를 덮어써 파일이 엉뚱한 위치로 가고,
    // 두 번째 드롭은 accepts_payload에서 조용히 사라진다.
    expect(importer.awaitingDropReservation).toBe(true);
    const second = { file: file('b.pdf'), kind: 'file' as const };
    expect(importer.importAtDrop([second], { page: 1, x: 5, y: 5, modifiers: {}, onFailure })).toBe(false);
    // 거부는 엔진을 건드리지 않는다 — 이 시점의 DnD 상태는 아직 **첫 드롭의 것**이고
    // 두 번째 드롭은 `handlers/dnd.ts`의 게이트 덕에 애초에 상태를 만든 적이 없다.
    expect(dndOps(editor)).toEqual([]);
    expect(onFailure).toHaveBeenCalledExactlyOnceWith(second);
    expect(deps.reserveUploads).toHaveBeenCalledOnce();

    reserve.resolve([reservationOf('FILE1', 'n1')]);
    await settle();

    expect(dndOps(editor)).toEqual(['drop']);
    expect(nodeMessages(editor)).toHaveLength(1);
    expect(importer.awaitingDropReservation).toBe(false);

    // 첫 드롭이 끝나면 다음 드롭은 다시 받는다.
    installReceipt(editor, ['node-b']);
    expect(importer.importAtDrop([{ file: file('c.pdf'), kind: 'file' }], { page: 0, x: 0, y: 0, modifiers: {}, onFailure })).toBe(true);
    await settle();
    expect(deps.reserveUploads).toHaveBeenCalledTimes(2);
  });
});

describe('failure routing', () => {
  it('sends asset-failed for a transfer failure and marks the local attempt as a transfer failure', async () => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockRejectedValue(new Error('network down'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure })).toBe(true);
    await settle();

    expect(deps.sendAssetFailed).toHaveBeenCalledExactlyOnceWith('DOC1', [{ id: 'FILE1', nonce: 'n1' }]);
    expect(editor.localUploads.get('FILE1')?.failedStage).toBe('transfer');
    expect(deps.finalizeFile).not.toHaveBeenCalled();
    expect(onFailure).toHaveBeenCalledOnce();
    // 실패 시 노드 자동 삭제는 사라졌다.
    expect(nodeMessages(editor).filter((message) => message.type === 'node' && message.op.type === 'delete')).toEqual([]);
  });

  it('never sends asset-failed for an uncertain finalize failure and retries the same nonce', async () => {
    const { editor, deps, importer } = createImporter({ finalizeRetryDelaysMs: [1, 1] });
    editor.externalElements.push(external('existing', 'file'));
    deps.finalizeFile.mockRejectedValue(coded('asset_finalize_retryable'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    expect(deps.finalizeFile).toHaveBeenCalledTimes(3);
    expect(deps.finalizeFile.mock.calls.every(([, nonce]) => nonce === 'n1')).toBe(true);
    expect(deps.sendAssetFailed).not.toHaveBeenCalled();
    expect(editor.localUploads.get('FILE1')?.failedStage).toBe('finalize-retryable');
  });

  it('stops immediately on a deterministic finalize rejection without sending asset-failed', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.finalizeFile.mockRejectedValue(coded('asset_finalize_rejected'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    expect(deps.finalizeFile).toHaveBeenCalledOnce();
    expect(deps.sendAssetFailed).not.toHaveBeenCalled();
    expect(editor.localUploads.get('FILE1')?.failedStage).toBe('finalize-rejected');
  });

  it('falls back to a fresh reservation when finalize reports an expired lease', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_upload_expired'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    expect(deps.abandonUpload).toHaveBeenCalledExactlyOnceWith('FILE1', 'n1');
    expect(deps.reserveUploads).toHaveBeenCalledTimes(2);
    expect(deps.reserveUploads.mock.calls[1]?.[1]).toEqual([expect.objectContaining({ assetId: 'FILE1' })]);
    expect(deps.transferToReservation).toHaveBeenCalledTimes(2);
    expect(deps.finalizeFile.mock.calls.map(([, nonce]) => nonce)).toEqual(['n1', 'n2']);
    expect(editor.messages).toHaveLength(1);
  });

  it('keeps every other item of a batch running when one item fails', async () => {
    const { editor, deps, importer } = createImporter();
    const failed = file('bad.pdf');
    installReceipt(editor, ['node-a', 'node-b']);
    deps.transferToReservation.mockImplementation(async (_reservation, input) => {
      if (input === failed) throw new Error('transfer failed');
    });

    expect(
      importer.importAtSelection(
        [
          { file: failed, kind: 'file' },
          { file: file('good.pdf'), kind: 'file' },
        ],
        { onFailure: vi.fn() },
      ),
    ).toBe(true);
    await settle();

    expect(editor.localUploads.get('FILE1')?.failedStage).toBe('transfer');
    expect(editor.assets.get('FILE2')).toEqual(fileAsset('FILE2'));
    expect(editor.localUploads.size).toBe(1);
    expect(deps.sendAssetFailed).toHaveBeenCalledExactlyOnceWith('DOC1', [{ id: 'FILE1', nonce: 'n1' }]);
  });
});

describe('retry routing', () => {
  it('abandons the old lease before re-reserving after a transfer failure', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockRejectedValueOnce(new Error('network down'));
    const order: string[] = [];
    deps.abandonUpload.mockImplementation(async () => {
      order.push('abandon');
      return true;
    });

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();
    deps.reserveUploads.mockImplementation(async (_documentId, items) => {
      order.push('reserve');
      return items.map((item) => reservationOf(item.assetId ?? 'FILE1', 'n2', item.format));
    });

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    // asset-failed는 응답 없는 WS 전송이라 "이미 지워졌을 것"이라 가정하면 재예약이 SET NX에 걸린다.
    expect(order).toEqual(['abandon', 'reserve']);
    expect(deps.abandonUpload).toHaveBeenCalledExactlyOnceWith('FILE1', 'n1');
    expect(deps.finalizeFile).toHaveBeenCalledExactlyOnceWith('FILE1', 'n2');
    expect(editor.messages).toHaveLength(1);
  });

  it('re-finalizes the same nonce without abandoning after a finalize failure', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_finalize_rejected'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    expect(deps.abandonUpload).not.toHaveBeenCalled();
    expect(deps.reserveUploads).toHaveBeenCalledOnce();
    expect(deps.transferToReservation).toHaveBeenCalledOnce();
    expect(deps.finalizeFile.mock.calls.map(([, nonce]) => nonce)).toEqual(['n1', 'n1']);
  });

  it('falls back to a new reservation when a retried finalize reports an expired lease', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_finalize_retryable'));
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_finalize_retryable'));
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_finalize_retryable'));
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_finalize_retryable'));

    expect(importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() })).toBe(
      true,
    );
    await settle();
    deps.finalizeFile.mockRejectedValueOnce(coded('asset_upload_expired'));

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    expect(deps.abandonUpload).toHaveBeenCalledOnce();
    expect(deps.reserveUploads).toHaveBeenCalledTimes(2);
    expect(deps.transferToReservation).toHaveBeenCalledTimes(2);
  });

  it('refuses to retry an attempt that is not in a failed state', () => {
    const { importer } = createImporter();
    expect(importer.retry('IMG-unknown')).toBe(false);
  });
});

describe('abandon settlement loop', () => {
  const failThenRetry = async (importer: EditorAttachmentImporter, editor: FakeEditor, deps: ReturnType<typeof createDeps>['deps']) => {
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockRejectedValueOnce(new Error('network down'));
    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();
  };

  it('re-reserves as soon as the pull reports the id as missing', async () => {
    const { ctx, editor, deps, importer } = createImporter();
    await failThenRetry(importer, editor, deps);
    deps.abandonUpload.mockResolvedValue(false);
    ctx.assetRefresh = vi.fn(() => editor.resolutions.set('FILE1', { state: 'missing' }));

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    expect(ctx.assetRefresh).toHaveBeenCalledWith(['FILE1']);
    expect(deps.abandonUpload).toHaveBeenCalledOnce();
    expect(deps.reserveUploads).toHaveBeenCalledTimes(2);
  });

  it('waits and retries abandon while the pull still reports the id as pending', async () => {
    const { ctx, editor, deps, importer } = createImporter({ settleAttempts: 3 });
    await failThenRetry(importer, editor, deps);
    deps.abandonUpload.mockResolvedValueOnce(false).mockResolvedValueOnce(false).mockResolvedValue(true);
    ctx.assetRefresh = vi.fn(() => editor.resolutions.set('FILE1', { state: 'pending', meta: { kind: 'file', name: 'a.pdf', size: 6 } }));

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    expect(deps.abandonUpload).toHaveBeenCalledTimes(3);
    expect(deps.delay).toHaveBeenCalled();
    expect(deps.reserveUploads).toHaveBeenCalledTimes(2);
  });

  it('retires the local attempt when the pull reports the id as already ready', async () => {
    const { ctx, editor, deps, importer } = createImporter();
    await failThenRetry(importer, editor, deps);
    deps.abandonUpload.mockResolvedValue(false);
    ctx.assetRefresh = vi.fn(() => editor.assets.set('FILE1', imageAsset('FILE1')));
    // 이 시나리오에서 kind는 file이므로 완결 자산은 파일 맵에 들어온다.
    ctx.assetRefresh = vi.fn(() => editor.assets.set('FILE1', fileAsset('FILE1')));

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledOnce();
    expect(deps.transferToReservation).toHaveBeenCalledOnce();
    expect(editor.localUploads.size).toBe(0);
    expect(revokeObjectURL).not.toHaveBeenCalledWith('blob:a.pdf');
  });

  it('gives up waiting after a bounded number of attempts instead of blocking forever', async () => {
    const { ctx, editor, deps, importer } = createImporter({ settleAttempts: 2 });
    await failThenRetry(importer, editor, deps);
    deps.abandonUpload.mockResolvedValue(false);
    ctx.assetRefresh = vi.fn(() => editor.resolutions.set('FILE1', { state: 'pending', meta: { kind: 'file', name: 'a.pdf', size: 6 } }));

    expect(importer.retry('FILE1')).toBe(true);
    await settle();

    expect(deps.abandonUpload).toHaveBeenCalledTimes(2);
    expect(deps.reserveUploads).toHaveBeenCalledTimes(2);
  });
});

describe('reselect', () => {
  it('abandons the live lease before reserving the same ID for the newly picked file', async () => {
    const { editor, deps, importer } = createImporter();
    const order: string[] = [];
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockRejectedValueOnce(new Error('network down'));
    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();
    deps.abandonUpload.mockImplementation(async () => {
      order.push('abandon');
      return true;
    });
    deps.reserveUploads.mockImplementation(async (_documentId, items) => {
      order.push('reserve');
      return items.map((item) => reservationOf(item.assetId ?? 'FILE1', 'n2', item.format));
    });

    expect(importer.reselect({ assetId: 'FILE1', nodeId: 'existing', kind: 'file', file: file('b.pdf'), onFailure: vi.fn() })).toBe(true);
    await settle();

    expect(order).toEqual(['abandon', 'reserve']);
    expect(deps.reserveUploads.mock.calls.at(-1)?.[1]).toEqual([expect.objectContaining({ assetId: 'FILE1', name: 'b.pdf' })]);
    expect(editor.messages).toHaveLength(1);
  });

  it('discards the newly picked file when abandon reveals the asset has already completed', async () => {
    const { ctx, editor, deps, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.finalizeFile.mockRejectedValue(coded('asset_finalize_retryable'));
    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();
    deps.abandonUpload.mockResolvedValue(false);
    ctx.assetRefresh = vi.fn(() => editor.assets.set('FILE1', fileAsset('FILE1')));
    deps.reserveUploads.mockClear();

    expect(importer.reselect({ assetId: 'FILE1', nodeId: 'existing', kind: 'file', file: file('b.pdf'), onFailure: vi.fn() })).toBe(true);
    await settle();

    expect(deps.reserveUploads).not.toHaveBeenCalled();
    expect(editor.localUploads.size).toBe(0);
    expect(editor.assets.get('FILE1')).toEqual(fileAsset('FILE1'));
  });

  it('re-claims an unresolved node that has no local attempt without abandoning anything', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('node-a', 'file', 'FILE7'));
    editor.resolutions.set('FILE7', { state: 'missing' });

    expect(importer.reselect({ assetId: 'FILE7', nodeId: 'node-a', kind: 'file', file: file('b.pdf'), onFailure: vi.fn() })).toBe(true);
    await settle();

    expect(deps.abandonUpload).not.toHaveBeenCalled();
    expect(deps.reserveUploads.mock.calls[0]?.[1]).toEqual([expect.objectContaining({ assetId: 'FILE7' })]);
    expect(editor.messages).toEqual([]);
  });
});

describe('heartbeat', () => {
  it('beats every 20 seconds for active transfers and stops once none remain', async () => {
    const { editor, deps, intervals, importer } = createImporter();
    const transfer = Promise.withResolvers<undefined>();
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockReturnValue(transfer.promise);

    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();

    expect(intervals).toHaveLength(1);
    expect(intervals[0]?.ms).toBe(ASSET_HEARTBEAT_INTERVAL_MS);
    expect(ASSET_HEARTBEAT_INTERVAL_MS).toBe(20_000);

    intervals[0]?.tick();
    expect(deps.sendAssetHeartbeat).toHaveBeenCalledExactlyOnceWith('DOC1', [{ id: 'FILE1', nonce: 'n1' }]);

    transfer.resolve(undefined);
    await settle();

    expect(intervals[0]?.stopped).toBe(true);
    intervals[0]?.tick();
    expect(deps.sendAssetHeartbeat).toHaveBeenCalledOnce();
  });

  it('does not beat for an attempt that has already failed', async () => {
    const { editor, deps, intervals, importer } = createImporter();
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockRejectedValue(new Error('network down'));

    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();

    intervals[0]?.tick();
    expect(deps.sendAssetHeartbeat).not.toHaveBeenCalled();
    expect(intervals[0]?.stopped).toBe(true);
  });
});

describe('lifecycle', () => {
  it('finishes the upload even when undo removes the node, and still enqueues nothing', async () => {
    const { editor, deps, importer } = createImporter();
    const transfer = Promise.withResolvers<undefined>();
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockReturnValue(transfer.promise);

    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();
    editor.externalElements = [];
    transfer.resolve(undefined);
    await settle();

    expect(editor.assets.get('FILE1')).toEqual(fileAsset('FILE1'));
    expect(nodeMessages(editor)).toHaveLength(1);
    expect(nodeMessages(editor)[0]).toMatchObject({ op: { type: 'set_attrs' } });
  });

  it('aborts transfers, revokes previews, and stops the heartbeat without sending asset-failed on teardown', async () => {
    const { editor, deps, intervals, importer } = createImporter();
    let aborted = false;
    editor.externalElements.push(external('existing', 'image'));
    deps.transferToReservation.mockImplementation(
      (_reservation, _file, signal) =>
        new Promise((_resolve, reject) => {
          signal.addEventListener('abort', () => {
            aborted = true;
            reject(new Error('aborted'));
          });
        }),
    );

    importer.importAtSelection([{ file: file('picture.png', 'image/png'), kind: 'image' }], {
      existingNodeId: 'existing',
      onFailure: vi.fn(),
    });
    await settle();

    importer.cancelEditor(editor as never);

    expect(aborted).toBe(true);
    expect(editor.localUploads.size).toBe(0);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:picture.png');
    expect(intervals[0]?.stopped).toBe(true);
    await settle();
    expect(deps.sendAssetFailed).not.toHaveBeenCalled();
  });

  it('retires a failed card and its preview when the server reports the asset as ready', async () => {
    const { editor, deps, importer } = createImporter({ finalizeRetryDelaysMs: [] });
    editor.externalElements.push(external('existing', 'image'));
    deps.finalizeImage.mockRejectedValue(coded('asset_finalize_retryable'));

    importer.importAtSelection([{ file: file('picture.png', 'image/png'), kind: 'image' }], {
      existingNodeId: 'existing',
      onFailure: vi.fn(),
    });
    await settle();

    importer.retireCompleted(editor as never, 'IMG1');

    expect(editor.localUploads.size).toBe(0);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:picture.png');
  });

  it('drops the local attempt for a node the user deleted', async () => {
    const { editor, deps, importer } = createImporter();
    const transfer = Promise.withResolvers<undefined>();
    editor.externalElements.push(external('existing', 'file'));
    deps.transferToReservation.mockReturnValue(transfer.promise);

    importer.importAtSelection([{ file: file('a.pdf'), kind: 'file' }], { existingNodeId: 'existing', onFailure: vi.fn() });
    await settle();

    importer.cancelNode(editor as never, 'existing');

    expect(editor.localUploads.size).toBe(0);
    transfer.resolve(undefined);
    await settle();
    expect(deps.finalizeFile).not.toHaveBeenCalled();
  });

  it('starts at most five transfers at a time', async () => {
    const { editor, deps, importer } = createImporter();
    const transfers = Array.from({ length: 7 }, () => Promise.withResolvers<undefined>());
    let index = 0;
    let active = 0;
    let maxActive = 0;
    installReceipt(
      editor,
      Array.from({ length: 7 }, (_, i) => `node-${i}`),
    );
    deps.transferToReservation.mockImplementation(() => {
      const pending = transfers[index++];
      if (!pending) throw new Error('missing transfer');
      active++;
      maxActive = Math.max(maxActive, active);
      return pending.promise.finally(() => active--);
    });

    importer.importAtSelection(
      Array.from({ length: 7 }, (_, i) => ({ file: file(`${i}.pdf`), kind: 'file' as const })),
      { onFailure: vi.fn() },
    );
    await settle();

    expect(maxActive).toBe(5);
    for (const pending of transfers) pending.resolve(undefined);
    await settle();
    expect(maxActive).toBe(5);
    expect(nodeMessages(editor)).toHaveLength(7);
  });

  it('rejects an import when the destination placeholder is unavailable or the document is unknown', () => {
    const { ctx, editor, importer } = createImporter();
    editor.externalElements.push(external('filled', 'image', 'IMG9'), external('empty', 'image'));

    expect(
      importer.importAtSelection([{ file: file('a.png', 'image/png'), kind: 'image' }], { existingNodeId: 'filled', onFailure: vi.fn() }),
    ).toBe(false);
    expect(importer.canReusePlaceholder('empty', 'image')).toBe(true);
    expect(importer.canReusePlaceholder('filled', 'image')).toBe(false);

    ctx.documentId = null;
    expect(
      importer.importAtSelection([{ file: file('a.png', 'image/png'), kind: 'image' }], { existingNodeId: 'empty', onFailure: vi.fn() }),
    ).toBe(false);
    expect(editor.messages).toEqual([]);
  });

  it('reports a preview that cannot be decoded without reserving anything for it', async () => {
    const { editor, deps, importer } = createImporter();
    const broken = file('broken.png', 'image/png');
    const onFailure = vi.fn();
    installReceipt(editor, ['node-a']);
    deps.getImageDimensions.mockImplementation(async (src) => {
      if (src === 'blob:broken.png') throw new Error('decode failed');
      return { width: 1, height: 1 };
    });

    expect(
      importer.importAtSelection(
        [
          { file: broken, kind: 'image' },
          { file: file('good.png', 'image/png'), kind: 'image' },
        ],
        { onFailure },
      ),
    ).toBe(true);
    await settle();

    expect(onFailure).toHaveBeenCalledExactlyOnceWith({ file: broken, kind: 'image' });
    expect(deps.reserveUploads.mock.calls[0]?.[1]).toEqual([expect.objectContaining({ name: 'good.png' })]);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:broken.png');
    expect(nodeMessages(editor)).toHaveLength(1);
  });
});

describe('unresolved node takeover', () => {
  it('treats a node whose ID the server reports as missing like an empty placeholder', () => {
    const { editor, importer } = createImporter();
    editor.externalElements.push(
      external('missing', 'image', 'IMG-M'),
      external('pending', 'image', 'IMG-P'),
      external('ready', 'image', 'IMG-R'),
      external('unknown', 'image', 'IMG-U'),
    );
    editor.resolutions.set('IMG-M', { state: 'missing' });
    editor.resolutions.set('IMG-P', { state: 'pending', meta: { kind: 'image', name: 'peer.png', size: 1 } });
    editor.assets.set('IMG-R', imageAsset('IMG-R'));

    // 이어받기는 픽커 경로가 담당한다. 드롭 재사용 후보는 엔진이 `id.get().is_none()`만 받으므로
    // ID를 단 노드는 후보가 될 수 없다 — 넘기면 엔진이 무시하고 새 노드를 만들어 ID가 중복된다.
    expect(importer.canReusePlaceholder('missing', 'image')).toBe(false);
    expect(importer.canReusePlaceholder('pending', 'image')).toBe(false);
    expect(importer.canReusePlaceholder('ready', 'image')).toBe(false);
    expect(importer.canReusePlaceholder('unknown', 'image')).toBe(false);
  });

  it('does not hand an ID-bearing node to the engine as a drop reuse candidate', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('taken-over', 'file', 'FILE-M'));
    editor.resolutions.set('FILE-M', { state: 'missing' });
    installReceipt(editor, ['created']);

    expect(
      importer.importAtDrop([{ file: file('a.pdf'), kind: 'file' }], {
        page: 0,
        x: 0,
        y: 0,
        modifiers: {},
        reuseNodeId: 'taken-over',
        onFailure: vi.fn(),
      }),
    ).toBe(true);
    await settle();

    expect(droppedReuseNodeId(editor)).toBeUndefined();
    // 새 노드이므로 새 ID를 발급하고 그 노드에만 SetAttr을 쓴다 — FILE-M은 건드리지 않는다.
    expect(deps.reserveUploads).toHaveBeenCalledExactlyOnceWith('DOC1', [expect.objectContaining({ assetId: undefined })]);
    expect(nodeMessages(editor).map((message) => (message.type === 'node' ? message.op.id : ''))).toEqual(['created']);
  });

  it('re-claims the existing ID for the first item of a multi-pick and mints new IDs for the rest', async () => {
    const { editor, deps, importer } = createImporter();
    editor.externalElements.push(external('taken-over', 'image', 'IMG-M'));
    editor.resolutions.set('IMG-M', { state: 'missing' });
    installReceipt(editor, ['created']);

    expect(
      importer.importAtSelection(
        [
          { file: file('a.png', 'image/png'), kind: 'image' },
          { file: file('b.png', 'image/png'), kind: 'image' },
        ],
        { existingNodeId: 'taken-over', onFailure: vi.fn() },
      ),
    ).toBe(true);
    await settle();

    expect(deps.reserveUploads).toHaveBeenCalledExactlyOnceWith('DOC1', [
      expect.objectContaining({ assetId: 'IMG-M' }),
      expect.objectContaining({ assetId: undefined }),
    ]);
    expect(nodeMessages(editor).map((message) => (message.type === 'node' ? message.op.id : ''))).toEqual(['created']);
  });

  it('pulls instead of failing when the re-claimed ID turns out to be completed', async () => {
    const { ctx, editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    const assetRefresh = vi.fn();
    ctx.assetRefresh = assetRefresh;
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    editor.externalElements.push(external('taken-over', 'image', 'IMG-M'));
    editor.resolutions.set('IMG-M', { state: 'missing' });
    deps.reserveUploads.mockRejectedValue(coded('asset_already_exists'));

    expect(importer.reselect({ assetId: 'IMG-M', nodeId: 'taken-over', kind: 'image', file: file('a.png', 'image/png'), onFailure })).toBe(
      true,
    );
    await settle();

    expect(assetRefresh).toHaveBeenCalledExactlyOnceWith(['IMG-M']);
    expect(onFailure).not.toHaveBeenCalled();
    expect(editor.localUploads.size).toBe(0);
    expect(nodeMessages(editor)).toEqual([]);
  });

  it('bails when the engine ignored the reuse candidate instead of writing onto the node it did create', async () => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    editor.externalElements.push(external('empty', 'file'));
    // 엔진이 후보를 무시하고 새 노드만 만든 영수증. 후보가 목록에 아예 없으므로
    // `indexOf(candidate) > 0` 형태의 판정은 -1로 통과해 버린다.
    installReceipt(editor, ['created']);

    expect(
      importer.importAtDrop([{ file: file('a.pdf'), kind: 'file' }], {
        page: 0,
        x: 0,
        y: 0,
        modifiers: {},
        reuseNodeId: 'empty',
        onFailure,
      }),
    ).toBe(true);
    await settle();

    expect(nodeMessages(editor)).toEqual([]);
    expect(deps.transferToReservation).not.toHaveBeenCalled();
    expect(onFailure).toHaveBeenCalledOnce();
  });

  it('drops the reuse candidate when the empty placeholder disappears mid-reservation', async () => {
    const { editor, deps, importer } = createImporter();
    const onFailure = vi.fn();
    const reserve = Promise.withResolvers<UploadReservation[]>();
    editor.externalElements.push(external('empty', 'file'));
    installReceipt(editor, ['created']);
    deps.reserveUploads.mockReturnValueOnce(reserve.promise);

    expect(
      importer.importAtDrop([{ file: file('a.pdf'), kind: 'file' }], {
        page: 0,
        x: 0,
        y: 0,
        modifiers: {},
        reuseNodeId: 'empty',
        onFailure,
      }),
    ).toBe(true);
    await settle();

    editor.externalElements = [];
    reserve.resolve([reservationOf('FILE1', 'n1')]);
    await settle();

    expect(droppedReuseNodeId(editor)).toBeUndefined();
    expect(onFailure).not.toHaveBeenCalled();
  });

  it('releases the drag-and-drop gate when a drop reservation never settles', async () => {
    const { deps, importer, timeouts } = createImporter({ dropReservationTimeoutMs: 1234 });
    const onFailure = vi.fn();
    vi.spyOn(console, 'error').mockImplementation(vi.fn());
    deps.reserveUploads.mockReturnValue(Promise.withResolvers<UploadReservation[]>().promise);

    expect(importer.importAtDrop([{ file: file('a.pdf'), kind: 'file' }], { page: 0, x: 0, y: 0, modifiers: {}, onFailure })).toBe(true);
    await settle();
    expect(importer.awaitingDropReservation).toBe(true);

    expect(timeouts.at(-1)?.ms).toBe(1234);
    timeouts.at(-1)?.tick();

    // 게이트가 풀리지 않으면 이 탭의 드래그 앤 드롭이 새로고침 전까지 전부 죽는다.
    expect(importer.awaitingDropReservation).toBe(false);
    expect(importer.importAtDrop([{ file: file('b.pdf'), kind: 'file' }], { page: 0, x: 0, y: 0, modifiers: {}, onFailure })).toBe(true);
  });
});
