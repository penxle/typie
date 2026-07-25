import pLimit from 'p-limit';
import { ASSET_MESSAGE_MAX_ITEMS } from '$lib/sync/protocol';
import type { AttachmentPlaceholderKind, InputModifiers } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext } from './editor.svelte';
import type { FileAsset, ImageAsset, LocalUploadFailureStage } from './types';

export type AttachmentImportItem = Readonly<{
  file: File;
  kind: AttachmentPlaceholderKind;
}>;

export type AttachmentImportFailureHandler = (item: AttachmentImportItem) => void;

export type AssetNonceItem = {
  id: string;
  nonce: string;
};

export type ReserveUploadItem = {
  kind: AttachmentPlaceholderKind;
  name: string;
  format: string;
  size: number;
  assetId?: string;
};

export type UploadReservation = {
  assetId: string;
  nonce: string;
  path: string;
  uploadUrl: string;
  uploadFields: Record<string, string>;
};

export type AttachmentImporterDeps = {
  getImageDimensions: (src: string) => Promise<{ width: number; height: number }>;
  reserveUploads: (documentId: string, items: ReserveUploadItem[]) => Promise<UploadReservation[]>;
  transferToReservation: (reservation: UploadReservation, file: File, signal: AbortSignal) => Promise<void>;
  finalizeImage: (assetId: string, nonce: string) => Promise<ImageAsset>;
  finalizeFile: (assetId: string, nonce: string) => Promise<FileAsset>;
  abandonUpload: (assetId: string, nonce: string) => Promise<boolean>;
  sendAssetFailed: (documentId: string, items: AssetNonceItem[]) => void;
  sendAssetHeartbeat: (documentId: string, items: AssetNonceItem[]) => void;
  startInterval: (ms: number, tick: () => void) => () => void;
  startTimeout: (ms: number, tick: () => void) => () => void;
  delay: (ms: number) => Promise<void>;
};

export type AttachmentImporterOptions = {
  heartbeatIntervalMs?: number;
  finalizeRetryDelaysMs?: readonly number[];
  settleDelayMs?: number;
  settleAttempts?: number;
  maxReserveItems?: number;
  dropReservationTimeoutMs?: number;
};

// lease TTL 유지 주기. 서버는 heartbeat를 fire-and-forget으로 받는다.
export const ASSET_HEARTBEAT_INTERVAL_MS = 20_000;
const FINALIZE_RETRY_DELAYS_MS = [500, 2000, 5000];
const SETTLE_DELAY_MS = 500;
const SETTLE_ATTEMPTS = 3;
const MAX_RESERVE_ITEMS = 50;
// 예약 왕복에는 클라이언트 타임아웃이 없다 — 응답이 영영 오지 않으면 이 상한이 드래그 앤 드롭을 되살린다.
const DROP_RESERVATION_TIMEOUT_MS = 30_000;

// 서버 오류 코드는 문자열 그대로가 계약이다(메시지 패턴 매칭 금지).
export const errorCodeOf = (err: unknown): string | undefined => {
  if (typeof err !== 'object' || err === null) return undefined;
  const { code } = err as { code?: unknown };
  return typeof code === 'string' ? code : undefined;
};

function chunked<T>(items: readonly T[], size: number): T[][] {
  const chunks: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    chunks.push(items.slice(index, index + size));
  }
  return chunks;
}

type PreparedItem = {
  item: AttachmentImportItem;
  previewUrl?: string;
  width?: number;
  height?: number;
};

type UploadRun = {
  assetId: string;
  documentId: string;
  editor: Editor;
  nodeId: string;
  kind: AttachmentPlaceholderKind;
  file: File;
  nonce: string;
  previewUrl?: string;
  width?: number;
  height?: number;
  controller: AbortController;
  cancelled: boolean;
  onFailure: AttachmentImportFailureHandler;
};

type FinalizeOutcome = 'done' | 'failed' | 'expired' | 'cancelled';

type AssetProbe = 'ready' | 'pending' | 'missing' | 'unknown';

type ReserveOutcome = { ok: true; reservations: UploadReservation[] } | { ok: false; code?: string };

/** 업로드를 받을 수 있는 목적지 노드. `assetId`가 있으면 그 ID를 다시 잡아야 하는 미해석 노드다. */
type ImportTarget = { assetId?: string };

/**
 * 순서가 이 클래스의 계약이다: **검증 → 예약 → 문서 기록 → 전송 → finalize**.
 * 예약이 실패하면 문서는 한 글자도 바뀌지 않고(enqueue·flush 0회), 업로드 완료는 **아무것도 enqueue하지 않는다**
 * (완료 SetAttr 없음 · 실패 시 노드 자동 삭제 없음).
 */
export class EditorAttachmentImporter {
  readonly #ctx: EditorContext;
  readonly #deps: AttachmentImporterDeps;
  // 전송 동시성 상한. **검증(미리보기 디코드)과 절대 공유하지 않는다** — 공유하면 업로드 5건이
  // 진행 중일 때 새 선택/붙여넣기/드롭이 예약조차 못 해 아무 피드백 없이 멈춘다.
  readonly #transferLimit = pLimit(5);
  readonly #previewLimit = pLimit(5);
  readonly #runs = new Map<string, UploadRun>();
  // 예약이 진행 중이라 아직 ID가 기록되지 않은 목적지 노드.
  readonly #claimedNodes = new Set<string>();
  readonly #heartbeatIntervalMs: number;
  readonly #finalizeRetryDelaysMs: readonly number[];
  readonly #settleDelayMs: number;
  readonly #settleAttempts: number;
  readonly #maxReserveItems: number;
  readonly #dropReservationTimeoutMs: number;
  #heartbeatStop: (() => void) | null = null;
  // 엔진의 DnD 상태는 하나뿐이다 — 예약을 기다리는 드롭이 있는 동안 두 번째 드롭을 겹칠 수 없다.
  #dropAwaitingReservation = false;
  #dropGateStop: (() => void) | null = null;

  constructor(ctx: EditorContext, deps: AttachmentImporterDeps, options: AttachmentImporterOptions = {}) {
    this.#ctx = ctx;
    this.#deps = deps;
    this.#heartbeatIntervalMs = options.heartbeatIntervalMs ?? ASSET_HEARTBEAT_INTERVAL_MS;
    this.#finalizeRetryDelaysMs = options.finalizeRetryDelaysMs ?? FINALIZE_RETRY_DELAYS_MS;
    this.#settleDelayMs = options.settleDelayMs ?? SETTLE_DELAY_MS;
    this.#settleAttempts = options.settleAttempts ?? SETTLE_ATTEMPTS;
    this.#maxReserveItems = options.maxReserveItems ?? MAX_RESERVE_ITEMS;
    this.#dropReservationTimeoutMs = options.dropReservationTimeoutMs ?? DROP_RESERVATION_TIMEOUT_MS;
  }

  /**
   * * 편집기 상태 조회
   */

  #editableEditor(): Editor | undefined {
    const editor = this.#ctx.editor;
    return editor !== undefined && this.#isEditorCurrent(editor) ? editor : undefined;
  }

  #isEditorCurrent(editor: Editor): boolean {
    return this.#ctx.editor === editor && !editor.destroyed && !editor.readOnly;
  }

  /** 새 import가 이 노드를 목적지로 삼아도 되는가(진행 중인 예약이 잡아 둔 노드 포함). */
  #available(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind): ImportTarget | undefined {
    return this.#claimedNodes.has(nodeId) ? undefined : this.#writable(editor, nodeId, kind);
  }

  /** 예약을 이미 손에 쥔 흐름이 이 노드에 ID를 기록해도 되는가(자기 claim은 무시). */
  #writable(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind): ImportTarget | undefined {
    const target = this.#placeholder(editor, nodeId, kind);
    return target !== undefined && !this.#hasRunForNode(editor, nodeId) ? target : undefined;
  }

  #claim(nodeId: string | undefined): void {
    if (nodeId !== undefined) this.#claimedNodes.add(nodeId);
  }

  #release(nodeId: string | undefined): void {
    if (nodeId !== undefined) this.#claimedNodes.delete(nodeId);
  }

  /**
   * 빈 placeholder와, 서버가 `missing`으로 확정한 ID를 단 노드를 같은 목적지로 취급한다.
   * 후자는 새 ID를 발급하지 않고 그 ID를 그대로 재예약한다(문서 무변경). 아직 해석 중(`pending`·미응답)이면
   * 곧 채워질 수 있으므로 목적지가 아니다.
   */
  #placeholder(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind): ImportTarget | undefined {
    const data = editor.externalElements.find((element) => element.node === nodeId)?.data;
    if (data?.type !== kind) return undefined;
    const assetId = data.id === undefined || data.id === '' ? undefined : data.id;
    if (assetId === undefined) return {};
    return this.#probe({ assetId, kind, editor }) === 'missing' ? { assetId } : undefined;
  }

  #carriesAssetId(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind, assetId: string): boolean {
    const data = editor.externalElements.find((element) => element.node === nodeId)?.data;
    return data?.type === kind && data.id === assetId;
  }

  #hasRunForNode(editor: Editor, nodeId: string): boolean {
    for (const run of this.#runs.values()) {
      if (run.editor === editor && run.nodeId === nodeId) return true;
    }
    return false;
  }

  /**
   * * 로컬 업로드 상태(assetId 키)
   */

  #putLocal(run: UploadRun, stage?: LocalUploadFailureStage): void {
    run.editor.localUploads.set(run.assetId, {
      kind: run.kind,
      nodeId: run.nodeId,
      file: run.file,
      nonce: run.nonce,
      previewUrl: run.previewUrl,
      width: run.width,
      height: run.height,
      name: run.file.name,
      size: run.file.size,
      phase: stage === undefined ? 'active' : 'failed',
      failedStage: stage,
    });
  }

  #markActive(run: UploadRun): void {
    this.#putLocal(run);
    this.#ensureHeartbeat();
  }

  #fail(run: UploadRun, stage: LocalUploadFailureStage): void {
    if (run.cancelled) return;
    this.#putLocal(run, stage);
    this.#stopHeartbeatIfIdle();
    try {
      run.onFailure({ file: run.file, kind: run.kind });
    } catch {
      // 호출측 보고(토스트)는 워커를 실패시키지 않는다.
    }
  }

  #retire(run: UploadRun): void {
    run.cancelled = true;
    run.controller.abort();
    this.#runs.delete(run.assetId);
    run.editor.localUploads.delete(run.assetId);
    if (run.previewUrl !== undefined) URL.revokeObjectURL(run.previewUrl);
    this.#stopHeartbeatIfIdle();
  }

  #isActive(run: UploadRun): boolean {
    return !run.cancelled && run.editor.localUploads.get(run.assetId)?.phase === 'active';
  }

  /**
   * * heartbeat
   */

  #ensureHeartbeat(): void {
    if (this.#heartbeatStop !== null) return;
    if (this.#activeItemsByDocument().size === 0) return;
    this.#heartbeatStop = this.#deps.startInterval(this.#heartbeatIntervalMs, () => this.#beat());
  }

  #stopHeartbeat(): void {
    this.#heartbeatStop?.();
    this.#heartbeatStop = null;
  }

  #stopHeartbeatIfIdle(): void {
    if (this.#activeItemsByDocument().size === 0) this.#stopHeartbeat();
  }

  #activeItemsByDocument(): Map<string, AssetNonceItem[]> {
    const grouped = new Map<string, AssetNonceItem[]>();
    for (const run of this.#runs.values()) {
      if (!this.#isActive(run)) continue;
      const items = grouped.get(run.documentId) ?? [];
      items.push({ id: run.assetId, nonce: run.nonce });
      grouped.set(run.documentId, items);
    }
    return grouped;
  }

  #beat(): void {
    const grouped = this.#activeItemsByDocument();
    if (grouped.size === 0) {
      this.#stopHeartbeat();
      return;
    }
    for (const [documentId, items] of grouped) {
      // 서버는 100건을 넘는 메시지를 malformed로 보고 연결을 끊는다.
      for (const chunk of chunked(items, ASSET_MESSAGE_MAX_ITEMS)) {
        this.#deps.sendAssetHeartbeat(documentId, chunk);
      }
    }
  }

  /**
   * * 검증·예약
   */

  async #prepare(item: AttachmentImportItem, onFailure: AttachmentImportFailureHandler): Promise<PreparedItem | undefined> {
    if (item.kind !== 'image') return { item };

    let previewUrl: string | undefined;
    try {
      previewUrl = URL.createObjectURL(item.file);
      const { width, height } = await this.#deps.getImageDimensions(previewUrl);
      return { item, previewUrl, width, height };
    } catch {
      if (previewUrl !== undefined) URL.revokeObjectURL(previewUrl);
      try {
        onFailure(item);
      } catch {
        // 호출측 보고가 배치 전체를 죽이지 않는다.
      }
      return undefined;
    }
  }

  async #prepareAll(items: readonly AttachmentImportItem[], onFailure: AttachmentImportFailureHandler): Promise<PreparedItem[]> {
    // 디코드가 필요한 이미지만 상한을 통과시킨다. 파일은 즉시 통과 — 전송 슬롯을 기다리지 않는다.
    const prepared = await Promise.all(
      items.map((item) =>
        item.kind === 'image' ? this.#previewLimit(() => this.#prepare(item, onFailure)) : this.#prepare(item, onFailure),
      ),
    );
    return prepared.filter((entry): entry is PreparedItem => entry !== undefined);
  }

  #revokePreviews(prepared: readonly PreparedItem[]): void {
    for (const entry of prepared) {
      if (entry.previewUrl !== undefined) URL.revokeObjectURL(entry.previewUrl);
    }
  }

  /**
   * 드롭 게이트를 연다. 예약이 영영 끝나지 않으면 이 탭의 드래그 앤 드롭이 전부 죽으므로,
   * `#closeDropGate`가 오지 않아도 상한 시간 뒤에는 반드시 풀린다.
   */
  #openDropGate(): void {
    this.#dropGateStop?.();
    this.#dropAwaitingReservation = true;
    this.#dropGateStop = this.#deps.startTimeout(this.#dropReservationTimeoutMs, () => {
      this.#dropGateStop = null;
      if (!this.#dropAwaitingReservation) return;
      console.error('Released the drag-and-drop gate: the drop reservation never settled.');
      this.#dropAwaitingReservation = false;
    });
  }

  #closeDropGate(): void {
    this.#dropGateStop?.();
    this.#dropGateStop = null;
    this.#dropAwaitingReservation = false;
  }

  /**
   * 드롭을 접을 때 엔진의 DnD 상태를 반드시 되돌린다.
   *
   * `drop`을 보내지 않고 끝내면 엔진은 `ExternalDnd` + 살아 있는 `drop_target`으로 남고
   * (OS 파일 드래그는 `drop` 이후 `dragend`/`dragleave`가 오지 않는다) 드롭 인디케이터가 계속 그려진다.
   * `leave`는 `ExternalDnd`를 `Idle`로 되돌리고, 이미 `Idle`이면 무해한 no-op이다.
   */
  #endDrag(editor: Editor): void {
    try {
      editor.enqueue({ type: 'dnd', op: { type: 'leave' } });
      editor.flush();
    } catch (err) {
      console.error('Failed to clear the drag state after an aborted drop:', err);
    }
  }

  async #reserve(documentId: string, items: ReserveUploadItem[]): Promise<ReserveOutcome> {
    try {
      const chunks = await Promise.all(chunked(items, this.#maxReserveItems).map((chunk) => this.#deps.reserveUploads(documentId, chunk)));
      const reservations = chunks.flat();
      return reservations.length === items.length ? { ok: true, reservations } : { ok: false };
    } catch (err) {
      console.error('Blob upload reservation failed; the document was left untouched.', err);
      return { ok: false, code: errorCodeOf(err) };
    }
  }

  #toReserveItem({ item }: PreparedItem, assetId?: string): ReserveUploadItem {
    return {
      kind: item.kind,
      name: item.file.name,
      format: item.file.type,
      size: item.file.size,
      assetId,
    };
  }

  #reportAll(prepared: readonly PreparedItem[], onFailure: AttachmentImportFailureHandler): void {
    this.#reportItems(
      prepared.map(({ item }) => item),
      onFailure,
    );
  }

  /**
   * 예약 실패 보고. 재예약하려던 ID가 이미 완결됐다면(`asset_already_exists`) 그건 실패가 아니라
   * "그 사이에 해석됐다"는 뜻이므로, 그 항목은 실패 카드 대신 pull로 넘기고 나머지만 보고한다.
   */
  #reportReserveFailure(
    code: string | undefined,
    reclaimedId: string | undefined,
    prepared: readonly PreparedItem[],
    onFailure: AttachmentImportFailureHandler,
  ): void {
    if (code !== 'asset_already_exists' || reclaimedId === undefined) {
      this.#reportAll(prepared, onFailure);
      return;
    }
    this.#ctx.assetRefresh?.([reclaimedId]);
    this.#reportAll(prepared.slice(1), onFailure);
  }

  #reportItems(items: readonly AttachmentImportItem[], onFailure: AttachmentImportFailureHandler): void {
    for (const item of items) {
      try {
        onFailure(item);
      } catch {
        // 호출측 보고가 나머지 항목의 보고를 막지 않는다.
      }
    }
  }

  /**
   * * 문서 기록(수신증 → SetAttr)
   */

  #collectReceipt(editor: Editor, requestId: string, enqueue: () => void): string[] | undefined {
    const matches: string[][] = [];
    const dispose = editor.on('attachment_placeholders_inserted', (_, event) => {
      if (event.request_id === requestId) matches.push(event.node_ids);
    });
    try {
      enqueue();
      editor.flush();
    } catch (err) {
      console.error('Failed to enqueue or flush attachment placeholder request:', err);
      return undefined;
    } finally {
      dispose();
    }
    if (matches.length === 0) {
      console.error('Attachment placeholder request emitted no matching receipt.', requestId);
      return undefined;
    }
    if (matches.length > 1) {
      console.error('Attachment placeholder request emitted duplicate matching receipts.', {
        requestId,
        receiptCount: matches.length,
      });
      return undefined;
    }
    return matches[0];
  }

  #isExactMapping(nodeIds: readonly string[], expectedCount: number): boolean {
    if (nodeIds.length !== expectedCount) {
      console.error('Attachment placeholder receipt count mismatch.', {
        expectedCount,
        actualCount: nodeIds.length,
      });
      return false;
    }
    if (new Set(nodeIds).size !== expectedCount) {
      console.error('Attachment placeholder receipt contains duplicate node IDs.', nodeIds);
      return false;
    }
    return true;
  }

  // 삽입 flush의 수신증은 삽입 트랜잭션이 커밋된 뒤에야 나오므로 SetAttr는 **다음 flush**여야 한다.
  // 항목마다 별도 flush라 Undo도 항목 단위다.
  #writeAssetId(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind, assetId: string): boolean {
    // 같은 ID를 다시 잡은 이어받기는 문서를 전혀 바꾸지 않는다(SetAttr 없음 · 히스토리 entry 0).
    if (this.#carriesAssetId(editor, nodeId, kind, assetId)) return true;

    try {
      editor.enqueue(
        kind === 'image'
          ? {
              type: 'node',
              op: { type: 'set_attr', id: nodeId, attr: { type: 'image', attr: { type: 'id', value: assetId } } },
            }
          : {
              type: 'node',
              op: { type: 'set_attrs', id: nodeId, attrs: { type: 'file', id: assetId } },
            },
      );
      editor.flush();
      return true;
    } catch (err) {
      console.error('Failed to record the reserved asset ID on the placeholder node:', err);
      return false;
    }
  }

  /**
   * * 실행
   */

  #startRun(args: {
    editor: Editor;
    documentId: string;
    nodeId: string;
    prepared: PreparedItem;
    reservation: UploadReservation;
    onFailure: AttachmentImportFailureHandler;
  }): void {
    const { editor, documentId, nodeId, prepared, reservation, onFailure } = args;
    const run: UploadRun = {
      assetId: reservation.assetId,
      documentId,
      editor,
      nodeId,
      kind: prepared.item.kind,
      file: prepared.item.file,
      nonce: reservation.nonce,
      previewUrl: prepared.previewUrl,
      width: prepared.width,
      height: prepared.height,
      controller: new AbortController(),
      cancelled: false,
      onFailure,
    };

    this.#runs.set(run.assetId, run);
    this.#markActive(run);
    this.#schedule(run, reservation);
  }

  #schedule(run: UploadRun, reservation: UploadReservation | null): void {
    void this.#transferLimit(() => this.#pipeline(run, reservation)).catch((err: unknown) => {
      console.error('Unexpected attachment import worker failure:', err);
    });
  }

  async #pipeline(run: UploadRun, reservation: UploadReservation | null): Promise<void> {
    let current = reservation;
    let reReserved = false;

    for (;;) {
      if (current) {
        try {
          await this.#deps.transferToReservation(current, run.file, run.controller.signal);
        } catch {
          if (run.cancelled) return;
          // 전송(transfer) 실패에만 asset-failed를 보낸다 — 응답 없는 fire-and-forget이고 큐에 쌓이지 않는다.
          this.#deps.sendAssetFailed(run.documentId, [{ id: run.assetId, nonce: run.nonce }]);
          this.#fail(run, 'transfer');
          return;
        }
        if (run.cancelled) return;
      }

      const outcome = await this.#finalizeLoop(run);
      if (outcome !== 'expired') return;

      // lease가 사라졌다 — 같은 nonce는 영영 실패하므로 새 nonce로 한 번 폴백한다.
      if (reReserved) {
        this.#fail(run, 'transfer');
        return;
      }
      reReserved = true;

      if (!(await this.#settleRun(run))) return;
      const renewed = await this.#reReserve(run);
      if (!renewed) return;
      current = renewed;
    }
  }

  async #finalizeLoop(run: UploadRun): Promise<FinalizeOutcome> {
    for (let attempt = 0; ; attempt++) {
      try {
        if (run.kind === 'image') {
          const asset = await this.#deps.finalizeImage(run.assetId, run.nonce);
          if (run.cancelled) return 'cancelled';
          run.editor.assets.set(asset.id, { type: 'image', ...asset });
        } else {
          const asset = await this.#deps.finalizeFile(run.assetId, run.nonce);
          if (run.cancelled) return 'cancelled';
          run.editor.assets.set(asset.id, { type: 'file', ...asset, size: Number(asset.size) });
        }
        this.#retire(run);
        return 'done';
      } catch (err) {
        if (run.cancelled) return 'cancelled';
        const code = errorCodeOf(err);

        // 결정적 거부 — 서버가 claim을 해제했으므로 같은 nonce 재시도는 무의미하다.
        if (code === 'asset_finalize_rejected' || code === 'forbidden' || code === 'validation_error') {
          this.#fail(run, 'finalize-rejected');
          return 'failed';
        }
        if (code === 'asset_upload_expired') return 'expired';

        // 불확실(인프라·timeout·네트워크) — lease는 그대로다. asset-failed를 보내지 않고 같은 nonce로 재시도한다.
        const delayMs = this.#finalizeRetryDelaysMs[attempt];
        if (delayMs === undefined) {
          this.#fail(run, 'finalize-retryable');
          return 'failed';
        }
        await this.#deps.delay(delayMs);
        if (run.cancelled) return 'cancelled';
      }
    }
  }

  async #reReserve(run: UploadRun): Promise<UploadReservation | undefined> {
    const outcome = await this.#reserve(run.documentId, [
      { kind: run.kind, name: run.file.name, format: run.file.type, size: run.file.size, assetId: run.assetId },
    ]);
    if (!outcome.ok && outcome.code === 'asset_already_exists') {
      // 그 사이에 이 ID가 완결됐다 — 실패 카드 대신 완성 자산을 보여 준다.
      this.#ctx.assetRefresh?.([run.assetId]);
      this.#retire(run);
      return undefined;
    }
    const reservation = outcome.ok ? outcome.reservations[0] : undefined;
    if (!reservation) {
      this.#fail(run, 'transfer');
      return undefined;
    }
    if (run.cancelled) return undefined;

    run.nonce = reservation.nonce;
    this.#markActive(run);
    return reservation;
  }

  /**
   * * abandon 정착 루프
   *
   * `abandonBlobUpload`의 false는 finalizing·이미 없음·nonce/소유자 불일치를 구분하지 않는다.
   * 곧바로 그 id를 pull해 분기하지 않으면 TTL이 만료된 뒤에도 false가 반복돼 무한 대기가 된다.
   */
  async #settle(target: {
    assetId: string;
    nonce: string;
    kind: AttachmentPlaceholderKind;
    editor: Editor;
    isCancelled: () => boolean;
  }): Promise<'reserve' | 'ready' | 'cancelled'> {
    for (let attempt = 0; attempt < this.#settleAttempts; attempt++) {
      let abandoned: boolean;
      try {
        abandoned = await this.#deps.abandonUpload(target.assetId, target.nonce);
      } catch {
        abandoned = false;
      }
      if (target.isCancelled()) return 'cancelled';
      if (abandoned) return 'reserve';

      this.#ctx.assetRefresh?.([target.assetId]);
      await this.#deps.delay(this.#settleDelayMs);
      if (target.isCancelled()) return 'cancelled';

      const probe = this.#probe(target);
      // `ready`면 이미 완결된 자산이다 — 로컬 시도 상태와 보유 source를 폐기하고 완성 자산을 그대로 보여준다.
      if (probe === 'ready') return 'ready';
      if (probe === 'missing') return 'reserve';
    }
    // TTL이 만료됐을 것이다. 무한 대기 대신 재예약을 시도하고, 실패하면 실패 카드로 드러낸다.
    return 'reserve';
  }

  /** 진행 중인 run의 lease를 정리한다. `false`면 이 run은 더 진행하지 않는다(취소되었거나 이미 완결). */
  async #settleRun(run: UploadRun): Promise<boolean> {
    const outcome = await this.#settle({ ...run, isCancelled: () => run.cancelled });
    if (outcome === 'ready') this.#retire(run);
    return outcome === 'reserve';
  }

  #probe(target: { assetId: string; kind: AttachmentPlaceholderKind; editor: Editor }): AssetProbe {
    const hasAsset = target.editor.assets.has(target.assetId);
    if (hasAsset) return 'ready';
    const resolution = target.editor.resolutions.get(target.assetId);
    if (resolution) return resolution.state;
    return 'unknown';
  }

  /**
   * * 공개 API
   */

  async #importAtSelection(
    items: readonly AttachmentImportItem[],
    {
      editor,
      documentId,
      existingNodeId,
      onFailure,
    }: {
      editor: Editor;
      documentId: string;
      existingNodeId?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): Promise<void> {
    try {
      await this.#runSelectionImport(items, { editor, documentId, existingNodeId, onFailure });
    } finally {
      // 예약이 어떻게 끝나든(성공·거부·예외) 목적지 claim은 반드시 풀린다.
      this.#release(existingNodeId);
    }
  }

  async #runSelectionImport(
    items: readonly AttachmentImportItem[],
    {
      editor,
      documentId,
      existingNodeId,
      onFailure,
    }: {
      editor: Editor;
      documentId: string;
      existingNodeId?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): Promise<void> {
    const prepared = await this.#prepareAll(items, onFailure);
    if (prepared.length === 0) return;

    const head = existingNodeId === undefined ? undefined : prepared[0];
    const tail = head ? prepared.slice(1) : prepared;

    // 미해석 노드를 채울 때는 새 ID를 발급하지 않고 문서에 이미 적힌 ID를 그대로 재예약한다.
    const claimed = head === undefined || existingNodeId === undefined ? undefined : this.#writable(editor, existingNodeId, head.item.kind);

    const outcome = await this.#reserve(
      documentId,
      prepared.map((entry, index) => this.#toReserveItem(entry, index === 0 ? claimed?.assetId : undefined)),
    );
    if (!outcome.ok) {
      this.#revokePreviews(prepared);
      this.#reportReserveFailure(outcome.code, claimed?.assetId, prepared, onFailure);
      return;
    }
    const reservations = outcome.reservations;

    const settled = head === undefined || existingNodeId === undefined ? undefined : this.#writable(editor, existingNodeId, head.item.kind);
    if (
      !this.#isEditorCurrent(editor) ||
      (head !== undefined && existingNodeId !== undefined && (settled === undefined || settled.assetId !== claimed?.assetId))
    ) {
      this.#revokePreviews(prepared);
      return;
    }

    const targets: { nodeId: string; prepared: PreparedItem; reservation: UploadReservation }[] = [];
    if (head && existingNodeId !== undefined) {
      const reservation = reservations[0];
      if (!reservation) return;
      targets.push({ nodeId: existingNodeId, prepared: head, reservation });
    }

    if (tail.length > 0) {
      const requestId = crypto.randomUUID();
      const nodeIds = this.#collectReceipt(editor, requestId, () => {
        editor.enqueue({
          type: 'insertion',
          op: {
            type: 'attachment_placeholders',
            request_id: requestId,
            kinds: tail.map(({ item }) => item.kind),
          },
        });
      });
      // 아직 #commit 전이라 head도 문서에 기록되지 않았다 — head 미리보기까지 함께 회수하고 전건 보고한다.
      if (!nodeIds || !this.#isExactMapping(nodeIds, tail.length)) {
        this.#revokePreviews(prepared);
        this.#reportAll(prepared, onFailure);
        return;
      }
      if (existingNodeId !== undefined && nodeIds.includes(existingNodeId)) {
        console.error('Attachment placeholder receipt contains the existing destination node ID.', {
          existingNodeId,
          nodeIds,
        });
        this.#revokePreviews(prepared);
        this.#reportAll(prepared, onFailure);
        return;
      }
      const offset = head ? 1 : 0;
      for (const [index, entry] of tail.entries()) {
        const nodeId = nodeIds[index];
        const reservation = reservations[index + offset];
        if (nodeId === undefined || reservation === undefined) return;
        targets.push({ nodeId, prepared: entry, reservation });
      }
    }

    this.#commit(editor, documentId, targets, onFailure);
  }

  async #importAtDrop(
    items: readonly AttachmentImportItem[],
    {
      editor,
      documentId,
      page,
      x,
      y,
      modifiers,
      reuseCandidate,
      onFailure,
    }: {
      editor: Editor;
      documentId: string;
      page: number;
      x: number;
      y: number;
      modifiers: InputModifiers;
      reuseCandidate?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): Promise<void> {
    try {
      await this.#runDropImport(items, { editor, documentId, page, x, y, modifiers, reuseCandidate, onFailure });
    } finally {
      this.#release(reuseCandidate);
      this.#closeDropGate();
    }
  }

  async #runDropImport(
    items: readonly AttachmentImportItem[],
    {
      editor,
      documentId,
      page,
      x,
      y,
      modifiers,
      reuseCandidate,
      onFailure,
    }: {
      editor: Editor;
      documentId: string;
      page: number;
      x: number;
      y: number;
      modifiers: InputModifiers;
      reuseCandidate?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): Promise<void> {
    const prepared = await this.#prepareAll(items, onFailure);
    if (prepared.length === 0) {
      this.#endDrag(editor);
      return;
    }

    const headKind = prepared[0]?.item.kind ?? 'file';
    // 드롭은 ID를 재예약하지 않는다 — 후보는 항상 빈 placeholder다(엔진이 그것만 재사용한다).
    // 미해석 노드 이어받기는 픽커 경로(`#runSelectionImport`)가 담당한다.
    const outcome = await this.#reserve(
      documentId,
      prepared.map((entry) => this.#toReserveItem(entry, undefined)),
    );
    if (!outcome.ok) {
      this.#endDrag(editor);
      this.#revokePreviews(prepared);
      this.#reportReserveFailure(outcome.code, undefined, prepared, onFailure);
      return;
    }
    const reservations = outcome.reservations;

    if (!this.#isEditorCurrent(editor)) {
      // 편집기가 read-only로 바뀌었을 수도 있다(파괴된 경우엔 #endDrag가 삼킨다).
      this.#endDrag(editor);
      this.#revokePreviews(prepared);
      return;
    }

    const settled = reuseCandidate === undefined ? undefined : this.#writable(editor, reuseCandidate, headKind);
    const candidate = settled === undefined ? undefined : reuseCandidate;
    const requestId = crypto.randomUUID();
    const nodeIds = this.#collectReceipt(editor, requestId, () => {
      editor.enqueue({
        type: 'dnd',
        op: {
          type: 'drop',
          page,
          x,
          y,
          modifiers,
          payload: {
            type: 'files',
            request_id: requestId,
            kinds: prepared.map(({ item }) => item.kind),
            reuse_node_id: candidate,
          },
        },
      });
    });
    // `drop`이 실제로 적용됐다면 엔진은 이미 Idle이다. enqueue/flush가 던져 적용되지 않았을 수 있으므로
    // 여기서도 상태를 되돌리고(Idle이면 no-op) 사용자에게 실패를 알린다.
    if (!nodeIds || !this.#isExactMapping(nodeIds, prepared.length)) {
      this.#endDrag(editor);
      this.#revokePreviews(prepared);
      this.#reportAll(prepared, onFailure);
      return;
    }
    if (candidate && nodeIds[0] !== candidate) {
      console.error('Attachment placeholder receipt placed the reuse candidate after the first item.', {
        reuseNodeId: candidate,
        nodeIds,
      });
      this.#endDrag(editor);
      this.#revokePreviews(prepared);
      this.#reportAll(prepared, onFailure);
      return;
    }

    const targets: { nodeId: string; prepared: PreparedItem; reservation: UploadReservation }[] = [];
    for (const [index, entry] of prepared.entries()) {
      const nodeId = nodeIds[index];
      const reservation = reservations[index];
      if (nodeId === undefined || reservation === undefined) return;
      targets.push({ nodeId, prepared: entry, reservation });
    }

    this.#commit(editor, documentId, targets, onFailure);
  }

  #commit(
    editor: Editor,
    documentId: string,
    targets: readonly { nodeId: string; prepared: PreparedItem; reservation: UploadReservation }[],
    onFailure: AttachmentImportFailureHandler,
  ): void {
    for (const target of targets) {
      if (!this.#writeAssetId(editor, target.nodeId, target.prepared.item.kind, target.reservation.assetId)) {
        this.#revokePreviews([target.prepared]);
        continue;
      }
      this.#startRun({ editor, documentId, nodeId: target.nodeId, prepared: target.prepared, reservation: target.reservation, onFailure });
    }
  }

  async #restart(run: UploadRun, stage: LocalUploadFailureStage | undefined): Promise<void> {
    if (stage === 'finalize-rejected' || stage === 'finalize-retryable') {
      const outcome = await this.#finalizeLoop(run);
      // 어떤 경로든 `asset_upload_expired`면 새 nonce 재예약으로 폴백한다.
      if (outcome !== 'expired') return;
    }

    if (!(await this.#settleRun(run))) return;
    const renewed = await this.#reReserve(run);
    if (!renewed) return;
    await this.#pipeline(run, renewed);
  }

  async #reselect({
    assetId,
    nodeId,
    kind,
    file,
    editor,
    documentId,
    onFailure,
  }: {
    assetId: string;
    nodeId: string;
    kind: AttachmentPlaceholderKind;
    file: File;
    editor: Editor;
    documentId: string;
    onFailure: AttachmentImportFailureHandler;
  }): Promise<void> {
    const prepared = await this.#prepare({ file, kind }, onFailure);
    if (!prepared) return;

    const previous = this.#runs.get(assetId);
    if (previous) {
      // 진행 중이던 전송을 중단하되 `asset-failed`는 보내지 않는다(우리가 스스로 접은 시도다).
      previous.cancelled = true;
      previous.controller.abort();
      const outcome = await this.#settle({ ...previous, isCancelled: () => false });
      this.#retire(previous);
      if (outcome === 'ready') {
        // 서버가 이미 완결했다 — 사용자가 새로 고른 파일은 버리고 완성 자산을 보여 준다.
        this.#revokePreviews([prepared]);
        return;
      }
    }

    if (!this.#isEditorCurrent(editor)) {
      this.#revokePreviews([prepared]);
      return;
    }

    const outcome = await this.#reserve(documentId, [this.#toReserveItem(prepared, assetId)]);
    if (!outcome.ok) {
      this.#revokePreviews([prepared]);
      this.#reportReserveFailure(outcome.code, assetId, [prepared], onFailure);
      return;
    }
    const reservation = outcome.reservations[0];
    if (!reservation) {
      this.#revokePreviews([prepared]);
      this.#reportAll([prepared], onFailure);
      return;
    }

    this.#startRun({ editor, documentId, nodeId, prepared, reservation, onFailure });
  }

  importAtSelection(
    items: readonly AttachmentImportItem[],
    {
      existingNodeId,
      onFailure,
    }: {
      existingNodeId?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): boolean {
    const first = items[0];
    const editor = this.#editableEditor();
    const documentId = this.#ctx.documentId;
    if (!first || !editor || !documentId) return false;
    if (existingNodeId !== undefined && this.#available(editor, existingNodeId, first.kind) === undefined) return false;

    // 예약이 끝나기 전까지 노드는 아직 비어 있다 — 그 창에서 같은 노드로 두 번째 import가 들어오면
    // 두 예약이 같은 노드에 ID를 쓰고 하나가 고아가 된다. 해제는 비동기 본문의 finally가 책임진다
    // (호출측 프로미스 체인에 의존하지 않는다).
    this.#claim(existingNodeId);
    void this.#importAtSelection([...items], { editor, documentId, existingNodeId, onFailure }).catch((err: unknown) => {
      console.error('Unexpected attachment import failure:', err);
      this.#release(existingNodeId);
    });
    return true;
  }

  importAtDrop(
    items: readonly AttachmentImportItem[],
    {
      page,
      x,
      y,
      modifiers,
      reuseNodeId,
      onFailure,
    }: {
      page: number;
      x: number;
      y: number;
      modifiers: InputModifiers;
      reuseNodeId?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): boolean {
    const first = items[0];
    const editor = this.#editableEditor();
    const documentId = this.#ctx.documentId;
    if (!first || !editor || !documentId) return false;

    // 엔진의 DnD 상태(`drop_target`)는 드래그 하나만 담는다. 앞선 드롭이 예약을 기다리는 동안 두 번째
    // 드롭을 받으면 뒤 드롭의 `over`가 앞 드롭의 목표를 덮어써서, 앞 파일이 **엉뚱한 위치**에 떨어지고
    // 뒤 파일은 조용히 사라진다. 두 번째는 즉시 접고 사용자에게 알린다.
    //
    // 여기서 `leave`를 보내면 **안 된다** — 그 순간 엔진 상태는 아직 A(앞 드롭)의 것이고, 이 드롭은
    // `handlers/dnd.ts`의 게이트 덕에 엔진을 건드린 적이 없다. 정리할 상태가 없다.
    if (this.#dropAwaitingReservation) {
      console.error('Rejected a drop that arrived while an earlier drop was still awaiting its reservation.');
      this.#reportItems(items, onFailure);
      return false;
    }

    const reuseTarget = reuseNodeId === undefined ? undefined : this.#available(editor, reuseNodeId, first.kind);
    // 엔진은 ID를 단 노드를 재사용하지 않는다 — 후보로 넘기면 무시하고 새 노드를 만들어 ID가 중복된다.
    const reuseCandidate = reuseTarget !== undefined && reuseTarget.assetId === undefined ? reuseNodeId : undefined;
    this.#claim(reuseCandidate);
    this.#openDropGate();
    void this.#importAtDrop([...items], { editor, documentId, page, x, y, modifiers, reuseCandidate, onFailure }).catch((err: unknown) => {
      console.error('Unexpected attachment import failure:', err);
      this.#release(reuseCandidate);
      this.#closeDropGate();
    });
    return true;
  }

  /**
   * 드롭 하나가 예약을 기다리는 중인가.
   *
   * `handlers/dnd.ts`가 이 값을 보고 **드래그 단계 전체**(enter/over/leave/end/내부 드래그 시작)를 막는다.
   * 엔진의 `DndState`는 슬롯 하나뿐이고 세션 태그가 없으며 `Drop`은 **처리 시점의** `drop_target`을 읽으므로
   * (`crates/editor-core/src/dnd.rs`, `handle/dnd.rs`), 예약을 기다리는 동안 새 드래그가 목표를 옮기면
   * 앞 드롭의 파일이 새 드래그가 가리키던 위치에 조용히 떨어진다.
   */
  get awaitingDropReservation(): boolean {
    return this.#dropAwaitingReservation;
  }

  /**
   * 드롭 목적지로 쓸 수 있는 노드인가. **빈 placeholder만** 해당한다 — 엔진(`handle/dnd.rs`)이
   * `id.get().is_none()`인 후보만 재사용하므로, 미해석 노드를 후보로 넘기면 엔진이 조용히 무시하고
   * 새 노드를 만들어 같은 assetId가 두 노드에 찍힌다. 미해석 노드 이어받기는 픽커 경로가 담당한다.
   */
  canReusePlaceholder(nodeId: string, kind: AttachmentPlaceholderKind): boolean {
    const editor = this.#ctx.editor;
    if (editor === undefined || !this.#isEditorCurrent(editor)) return false;
    const target = this.#available(editor, nodeId, kind);
    return target !== undefined && target.assetId === undefined;
  }

  /**
   * 실패한 시도를 같은 노드·같은 assetId로 다시 민다. **문서는 건드리지 않는다.**
   * 재시도 방법은 실패 단계마다 다르다:
   * - `transfer`: asset-failed가 lease를 이미 지웠을 수 있지만 그 전송은 무응답이라 확신할 수 없다 →
   *   멱등 정리로 abandon을 먼저 부르고 **새 nonce로 재예약**한다(바로 재예약하면 `SET NX`에 걸린다).
   * - `finalize-*`: lease가 살아 있으므로 **같은 nonce로 finalize를 재호출**한다.
   */
  retry(assetId: string): boolean {
    const run = this.#runs.get(assetId);
    const entry = run?.editor.localUploads.get(assetId);
    if (!run || !entry || entry.phase !== 'failed') return false;

    const stage = entry.failedStage;
    this.#markActive(run);
    void this.#transferLimit(() => this.#restart(run, stage)).catch((err: unknown) => {
      console.error('Unexpected attachment retry failure:', err);
    });
    return true;
  }

  /**
   * 사용자가 다른 파일을 고른다. 노드는 그대로 두고 같은 assetId를 새 nonce로 다시 잡는다.
   * abandon 없이 재예약하면 NX가 TTL까지 거부하므로 **abandon이 먼저**이며, false면 pull로 분기한다.
   */
  reselect({
    assetId,
    nodeId,
    kind,
    file,
    onFailure,
  }: {
    assetId: string;
    nodeId: string;
    kind: AttachmentPlaceholderKind;
    file: File;
    onFailure: AttachmentImportFailureHandler;
  }): boolean {
    const editor = this.#editableEditor();
    const documentId = this.#ctx.documentId;
    if (!editor || !documentId) return false;

    void this.#reselect({ assetId, nodeId, kind, file, editor, documentId, onFailure }).catch((err: unknown) => {
      console.error('Unexpected attachment reselect failure:', err);
    });
    return true;
  }

  /** `ready`는 어디서 도착했든 그 id의 로컬 시도 상태를 은퇴시킨다(낡은 실패 카드가 완성 자산을 가리지 않도록). */
  retireCompleted(editor: Editor, assetId: string): void {
    const run = this.#runs.get(assetId);
    if (run) {
      this.#retire(run);
      return;
    }
    const entry = editor.localUploads.get(assetId);
    if (!entry) return;
    if (entry.previewUrl !== undefined) URL.revokeObjectURL(entry.previewUrl);
    editor.localUploads.delete(assetId);
    this.#stopHeartbeatIfIdle();
  }

  /** 세션 종료: 전송 중단·objectURL 회수·heartbeat 정지. `asset-failed`는 보내지 않는다(TTL이 정리). */
  cancelEditor(editor: Editor): void {
    const owned = [...this.#runs.values()].filter((run) => run.editor === editor);
    for (const run of owned) {
      run.cancelled = true;
      run.controller.abort();
      this.#runs.delete(run.assetId);
    }
    for (const entry of editor.localUploads.values()) {
      if (entry.previewUrl !== undefined) URL.revokeObjectURL(entry.previewUrl);
    }
    editor.localUploads.clear();
    this.#stopHeartbeatIfIdle();
  }

  cancelNode(editor: Editor, nodeId: string): void {
    for (const run of this.#runs.values()) {
      if (run.editor === editor && run.nodeId === nodeId) this.#retire(run);
    }
    for (const [assetId, entry] of editor.localUploads) {
      if (entry.nodeId !== nodeId) continue;
      if (entry.previewUrl !== undefined) URL.revokeObjectURL(entry.previewUrl);
      editor.localUploads.delete(assetId);
    }
    this.#stopHeartbeatIfIdle();
  }
}
