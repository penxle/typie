// 탭 복귀 후 페이지 캔버스 렌더링 깨짐 진단용 계측. localStorage 'typie:surface-probe' 값으로 모드를
// 고르고 새로고침한다.
// '1'     = 픽셀 스냅샷 검증. CPU 캔버스는 getImageData(캔버스를 CPU 래스터로 전환시키는 부작용 있음 —
//           버그를 가리는 것이 실측으로 확인됨), GL 캔버스는 debug readback FFI로 판독한다(부작용 없음).
//           GL은 texture(원본 oracle) vs present(최종 blit oracle)를 비교하고, 멀티타일 페이지는 타일
//           경계에서 seam telemetry를 남긴다.
// 'no-gl' = OffscreenCanvas의 webgl 컨텍스트를 차단해 GL present 경로만 제거(readback 없음).
// 'cpu'   = 페이지 캔버스를 willReadFrequently로 선점해 처음부터 CPU 백킹(readback 없음).
// 'gl-loss' = 로스 주입 e2e. GL 페이지마다 한 번: texture+present 캡처 → loseContext() → 복구 대기 →
//           재캡처 → 소스별 스냅샷 일치를 콘솔에 assert.

import { seamSamplePoints } from './surface-seam';

const mode = typeof localStorage === 'undefined' ? null : localStorage.getItem('typie:surface-probe');
const enabled = mode === '1';
const blockGl = mode === 'no-gl';
const forceCpu = mode === 'cpu';
const glLoss = mode === 'gl-loss';
const active = enabled || blockGl || forceCpu || glLoss;
const pageSurfaceMode = typeof localStorage === 'undefined' ? null : localStorage.getItem('typie:page-surface');

// 프로브가 편집기에서 필요로 하는 최소 표면(구조적). Editor 인스턴스가 이를 만족한다.
type ProbeEditor = {
  debugReadSurfacePixels(page: number, x: number, y: number, w: number, h: number, source: 'texture' | 'present'): Uint8Array;
  debugSurfaceTileRanges(page: number): Int32Array;
  surfaceBackend(page: number): string;
};

type Entry = {
  canvas: HTMLCanvasElement;
  editor: ProbeEditor;
  baseline: Uint8ClampedArray | null;
  baselineAt: number;
  renders: number;
  wiped: boolean;
  blitDivergenceLogged: boolean;
  seamDivergenceLogged: boolean;
};

const editors = new Map<ProbeEditor, Map<number, Entry>>();
let intervalId: ReturnType<typeof setInterval> | undefined;

const timestamp = () => new Date().toISOString().slice(11, 23);
const report = (...args: unknown[]) => console.warn('[surface-probe]', timestamp(), ...args);
const trace = (...args: unknown[]) => console.info('[surface-probe]', timestamp(), ...args);

const BLOCK = 2;
const BLOCK_BYTES = BLOCK * BLOCK * 4;

function samplePoints(width: number, height: number): [number, number][] {
  return [
    [8, 8],
    [width - 8 - BLOCK, 8],
    [8, height - 8 - BLOCK],
    [width - 8 - BLOCK, height - 8 - BLOCK],
    [Math.floor(width / 2), Math.floor(height / 2)],
  ];
}

// GL 캔버스: 편집기 debug FFI로 각 표본점 2×2 블록을 판독해(RGBA premultiplied) 스냅샷을 만든다.
// 판독 불가(dead/lost/oversized 또는 clamp로 사라진 rect)면 null.
function captureGl(editor: ProbeEditor, page: number, source: 'texture' | 'present', points: [number, number][]): Uint8ClampedArray | null {
  const out = new Uint8ClampedArray(points.length * BLOCK_BYTES);
  for (const [i, [x, y]] of points.entries()) {
    const px = editor.debugReadSurfacePixels(page, x, y, BLOCK, BLOCK, source);
    if (px.length !== BLOCK_BYTES) return null;
    out.set(px, i * BLOCK_BYTES);
  }
  return out;
}

// wipe 감지용 baseline 스냅샷. CPU는 getImageData(2d 전용), GL은 present oracle(표시되는 픽셀)로 읽는다.
// getContext('2d')가 null이면 이미 GL 컨텍스트가 잡힌 캔버스다(다른 타입 요청은 부작용 없이 null 반환).
function capture(entry: Entry, page: number): Uint8ClampedArray | null {
  const canvas = entry.canvas;
  if (canvas.width < 32 || canvas.height < 32) return null;
  const ctx = canvas.getContext('2d');
  if (ctx) {
    const points = samplePoints(canvas.width, canvas.height);
    const out = new Uint8ClampedArray(points.length * BLOCK_BYTES);
    try {
      for (const [i, [x, y]] of points.entries()) {
        out.set(ctx.getImageData(x, y, BLOCK, BLOCK).data, i * BLOCK_BYTES);
      }
    } catch {
      return null;
    }
    return out;
  }
  return captureGl(entry.editor, page, 'present', samplePoints(canvas.width, canvas.height));
}

function diffPoints(a: Uint8ClampedArray, b: Uint8ClampedArray): number[] {
  const mismatched: number[] = [];
  for (let i = 0; i * BLOCK_BYTES < a.length; i++) {
    for (let j = 0; j < BLOCK_BYTES; j++) {
      if (a[i * BLOCK_BYTES + j] !== b[i * BLOCK_BYTES + j]) {
        mismatched.push(i);
        break;
      }
    }
  }
  return mismatched;
}

// GL 전용 진단(커밋된 render 직후 1회). (1) texture vs present 불일치 = blit 산술/타일 배치/백버퍼 문제,
// (2) 멀티타일 페이지의 타일 경계 seam telemetry. 둘 다 premultiplied 동일 도메인이라 정확 일치로 판정.
function inspectGl(entry: Entry, page: number): void {
  const canvas = entry.canvas;
  if (canvas.width < 32 || canvas.height < 32) return;
  if (canvas.getContext('2d')) return; // CPU 캔버스 — 판독할 GL 표면 없음

  const editor = entry.editor;
  const corners = samplePoints(canvas.width, canvas.height);
  const texture = captureGl(editor, page, 'texture', corners);
  const present = captureGl(editor, page, 'present', corners);
  if (texture && present) {
    const mismatched = diffPoints(texture, present);
    if (mismatched.length > 0 && !entry.blitDivergenceLogged) {
      entry.blitDivergenceLogged = true;
      report(`gl texture/present divergence page=${page} points=[${mismatched.join(',')}] — blit arithmetic / backbuffer suspect`);
    } else if (mismatched.length === 0 && entry.blitDivergenceLogged) {
      entry.blitDivergenceLogged = false;
      trace(`gl texture/present agree again page=${page}`);
    }
  }

  const tileY0s = [...editor.debugSurfaceTileRanges(page)];
  const seams = seamSamplePoints(tileY0s, canvas.width, canvas.height);
  if (seams.length === 0) return;
  const seamTexture = captureGl(editor, page, 'texture', seams);
  const seamPresent = captureGl(editor, page, 'present', seams);
  if (seamTexture && seamPresent) {
    const mismatched = diffPoints(seamTexture, seamPresent);
    if (mismatched.length > 0 && !entry.seamDivergenceLogged) {
      entry.seamDivergenceLogged = true;
      report(`gl seam divergence page=${page} tiles=${tileY0s.length} points=[${mismatched.join(',')}]`);
    } else if (mismatched.length === 0 && entry.seamDivergenceLogged) {
      entry.seamDivergenceLogged = false;
      trace(`gl seams agree again page=${page}`);
    }
  }
}

function verifyAll() {
  for (const [, pages] of editors) {
    for (const [page, entry] of pages) {
      if (!entry.baseline || !entry.canvas.isConnected) continue;
      const current = capture(entry, page);
      if (!current) continue;
      const mismatched = diffPoints(entry.baseline, current);
      if (mismatched.length > 0) {
        if (!entry.wiped) {
          entry.wiped = true;
          report(
            `wipe detected page=${page} points=[${mismatched.join(',')}] renders=${entry.renders}`,
            `sinceBaseline=${((performance.now() - entry.baselineAt) / 1000).toFixed(1)}s hidden=${document.visibilityState !== 'visible'}`,
          );
        }
      } else if (entry.wiped) {
        entry.wiped = false;
        trace(`pixels match again without render page=${page}`);
      }
    }
  }
}

function ensureInterval() {
  intervalId ??= setInterval(verifyAll, 1000);
}

// 로스 주입 e2e: GL 페이지가 준비되면 한 번 실행. 컨텍스트를 강제로 잃게 하고 매니저의 복구
// (remount)를 기다린 뒤, 소스별로 복구 전/후 스냅샷 일치를 assert한다. remount는 캔버스를 교체하므로
// 매 단계 (editor,page) 키로 현재 entry를 다시 조회한다.
function scheduleLossInjection(editor: ProbeEditor, page: number): void {
  let attempts = 0;
  const tryStart = () => {
    if (attempts++ > 40) return;
    const entry = editors.get(editor)?.get(page);
    if (!entry) return; // detached
    if (entry.canvas.width < 32 || entry.canvas.getContext('2d')) {
      setTimeout(tryStart, 100);
      return;
    }
    runLossInjection(editor, page);
  };
  setTimeout(tryStart, 200);
}

function runLossInjection(editor: ProbeEditor, page: number): void {
  const entry = editors.get(editor)?.get(page);
  if (!entry) return;
  const canvas = entry.canvas;
  const w = canvas.width;
  const h = canvas.height;
  const points = samplePoints(w, h);
  const preTexture = captureGl(editor, page, 'texture', points);
  const prePresent = captureGl(editor, page, 'present', points);
  if (!preTexture || !prePresent) {
    report(`gl-loss page=${page} pre-capture failed backend=${editor.surfaceBackend(page)}`);
    return;
  }

  trace(`gl-loss injecting loseContext page=${page} backend=${editor.surfaceBackend(page)}`);
  canvas.getContext('webgl2')?.getExtension('WEBGL_lose_context')?.loseContext();

  let attempts = 0;
  const poll = () => {
    attempts++;
    const cur = editors.get(editor)?.get(page);
    const backend = editor.surfaceBackend(page);
    const ready = cur && cur.canvas.width === w && cur.canvas.height === h && !cur.canvas.getContext('2d');
    if (backend === 'gl' && ready) {
      const postTexture = captureGl(editor, page, 'texture', points);
      const postPresent = captureGl(editor, page, 'present', points);
      const textureMatch = postTexture !== null && diffPoints(preTexture, postTexture).length === 0;
      const presentMatch = postPresent !== null && diffPoints(prePresent, postPresent).length === 0;
      report(
        `gl-loss recovered page=${page} attempts=${attempts}`,
        `texture=${textureMatch ? 'match' : 'MISMATCH'} present=${presentMatch ? 'match' : 'MISMATCH'}`,
      );
      return;
    }
    if (attempts > 60) {
      report(`gl-loss page=${page} did not recover within timeout backend=${backend}`);
      return;
    }
    setTimeout(poll, 100);
  };
  setTimeout(poll, 100);
}

export function probeAttach(editor: ProbeEditor, page: number, canvas: HTMLCanvasElement): void {
  if (forceCpu) {
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (ctx) {
      trace(`forced willReadFrequently page=${page}`);
    } else {
      report(`cpu probe ineffective (gl canvas) page=${page}`);
    }
    return;
  }
  if (!enabled && !glLoss) return;
  let pages = editors.get(editor);
  if (!pages) {
    pages = new Map();
    editors.set(editor, pages);
  }
  pages.set(page, {
    canvas,
    editor,
    baseline: null,
    baselineAt: 0,
    renders: 0,
    wiped: false,
    blitDivergenceLogged: false,
    seamDivergenceLogged: false,
  });
  if (enabled) ensureInterval();
  if (glLoss) scheduleLossInjection(editor, page);
}

export function probeDetach(editor: ProbeEditor, page: number): void {
  if (!enabled && !glLoss) return;
  editors.get(editor)?.delete(page);
}

export function probeRendered(editor: ProbeEditor, page: number): void {
  if (!enabled) return;
  const entry = editors.get(editor)?.get(page);
  if (!entry) return;
  entry.renders += 1;
  entry.baseline = capture(entry, page);
  entry.baselineAt = performance.now();
  if (entry.wiped) {
    entry.wiped = false;
    trace(`re-rendered after wipe page=${page}`);
  }
  inspectGl(entry, page);
}

export function probeEvent(message: string): void {
  if (!active) return;
  trace(message);
}

if (blockGl && typeof OffscreenCanvas !== 'undefined') {
  const original = OffscreenCanvas.prototype.getContext;
  (OffscreenCanvas.prototype as { getContext: (...args: unknown[]) => unknown }).getContext = function (
    this: OffscreenCanvas,
    ...args: unknown[]
  ) {
    if (typeof args[0] === 'string' && args[0].startsWith('webgl')) {
      trace(`blocked OffscreenCanvas.getContext('${args[0]}')`);
      return null;
    }
    return (original as (...args: unknown[]) => unknown).call(this, ...args);
  };
}

if (active && typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    trace(`visibilitychange state=${document.visibilityState}`);
    if (enabled && document.visibilityState === 'visible') {
      for (const delay of [100, 500, 1500, 3000, 6000]) {
        setTimeout(verifyAll, delay);
      }
    }
  });
  trace(`enabled mode=${mode}`);
}

// 강제 CPU 프로브('cpu')는 캔버스를 willReadFrequently 2d로 선점한다 — page-surface 기본값이 GL이므로
// (killswitch 'cpu'가 아닌 한) GL 백킹과 충돌한다.
if (forceCpu && pageSurfaceMode !== 'cpu') {
  report(
    `surface-probe=cpu preempts the default gl backend (page-surface=${pageSurfaceMode ?? 'gl(default)'}); set page-surface=cpu to force cpu backing`,
  );
}
