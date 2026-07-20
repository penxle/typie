// 탭 복귀 후 페이지 캔버스 렌더링 깨짐 진단용 계측. localStorage 'typie:surface-probe'='1'로 켜고
// 새로고침한다.
// '1' = 픽셀 스냅샷 검증. 페이지 캔버스를 getImageData로 표본 판독(캔버스를 CPU 래스터로 전환시키는
//       부작용 있음 — 버그를 가리는 것이 실측으로 확인됨)해 baseline과 대조, 렌더 없는 wipe를 감지한다.

const mode = typeof localStorage === 'undefined' ? null : localStorage.getItem('typie:surface-probe');
const enabled = mode === '1';

// 프로브가 (editor,page)별 entry를 묶는 opaque 키. Editor 인스턴스가 이를 만족한다.
type ProbeEditor = object;

type Entry = {
  canvas: HTMLCanvasElement;
  baseline: Uint8ClampedArray | null;
  baselineAt: number;
  renders: number;
  wiped: boolean;
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

// wipe 감지용 스냅샷: 페이지 캔버스를 getImageData(2d)로 표본 판독한다. 컨텍스트를 못 잡거나
// 너무 작은 캔버스면 null.
function capture(entry: Entry): Uint8ClampedArray | null {
  const canvas = entry.canvas;
  if (canvas.width < 32 || canvas.height < 32) return null;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
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

function verifyAll() {
  for (const [, pages] of editors) {
    for (const [page, entry] of pages) {
      if (!entry.baseline || !entry.canvas.isConnected) continue;
      const current = capture(entry);
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

export function probeAttach(editor: ProbeEditor, page: number, canvas: HTMLCanvasElement): void {
  if (!enabled) return;
  let pages = editors.get(editor);
  if (!pages) {
    pages = new Map();
    editors.set(editor, pages);
  }
  pages.set(page, { canvas, baseline: null, baselineAt: 0, renders: 0, wiped: false });
  ensureInterval();
}

export function probeDetach(editor: ProbeEditor, page: number): void {
  if (!enabled) return;
  editors.get(editor)?.delete(page);
}

export function probeRendered(editor: ProbeEditor, page: number): void {
  if (!enabled) return;
  const entry = editors.get(editor)?.get(page);
  if (!entry) return;
  entry.renders += 1;
  entry.baseline = capture(entry);
  entry.baselineAt = performance.now();
  if (entry.wiped) {
    entry.wiped = false;
    trace(`re-rendered after wipe page=${page}`);
  }
}

export function probeEvent(message: string): void {
  if (!enabled) return;
  trace(message);
}

if (enabled && typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    trace(`visibilitychange state=${document.visibilityState}`);
    if (document.visibilityState === 'visible') {
      for (const delay of [100, 500, 1500, 3000, 6000]) {
        setTimeout(verifyAll, delay);
      }
    }
  });
  trace(`enabled mode=${mode}`);
}
