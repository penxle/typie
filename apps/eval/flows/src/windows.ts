import type { Scene, Window } from './analysis-types.ts';

// 창 편성은 코드가 결정론적으로 수행한다. 여기에 LLM을 쓰면 실행 간 변동(실측 55%)이
// 창 경계에까지 들어와 재현성이 더 나빠진다.

// 창 하나에 담을 목표 분량. 장면 경계를 우선하므로 실제 창은 이 값을 넘거나 못 미칠 수 있다.
export const DEFAULT_WINDOW_SIZE = 40_000;
// 목표를 넘겨도 닫을 경계를 찾지 못할 때, 여기에 닿으면 문단 경계에서 강제로 자른다.
export const DEFAULT_HARD_LIMIT = 120_000;
// 앞뒤로 붙일 읽기용 원문의 문장 수.
const TAIL_SENTENCES = 3;

type PlanOptions = {
  windowSize?: number;
  hardLimit?: number;
};

const sentenceHead = (text: string, count: number): string => {
  const pattern = /[.!?。！？…]["'”’」』〉》)\]]*\s*/g;
  let end = 0;
  for (let i = 0; i < count; i++) {
    const match = pattern.exec(text);
    if (!match) return text;
    end = match.index + match[0].length;
  }
  return text.slice(0, end);
};

const sentenceTail = (text: string, count: number): string => {
  const pattern = /[.!?。！？…]["'”’」』〉》)\]]*\s*/g;
  const ends: number[] = [];
  let match;
  while ((match = pattern.exec(text))) {
    ends.push(match.index + match[0].length);
  }
  // 마지막 문장 끝은 경계 그 자체이므로 그 앞의 것들에서 센다.
  const boundaries = ends.filter((e) => e < text.length);
  const target = boundaries.at(-count);
  return target === undefined ? text : text.slice(target);
};

// 문단 경계 → 없으면 문장 경계 → 그것도 없으면 그대로. 강제 분할에서만 쓴다.
const forcedBreak = (text: string, from: number, limit: number): number => {
  const slice = text.slice(from, from + limit);
  const paragraph = slice.lastIndexOf('\n');
  if (paragraph > 0) return from + paragraph + 1;
  const pattern = /[.!?。！？…]["'”’」』〉》)\]]*\s*/g;
  let last = -1;
  let match;
  while ((match = pattern.exec(slice))) {
    last = match.index + match[0].length;
  }
  return from + (last > 0 ? last : limit);
};

/**
 * 장면 지도로 분석 창을 편성한다.
 *
 * 경계는 boundaryQuality가 'clean'인 곳을 우선하고, 없으면 'weak'까지 허용한다.
 * 둘 다 없으면 창을 닫지 않는다 — 구조가 없는 원고에 억지로 선을 긋지 않는 것이
 * 이 설계의 핵심이다. hardLimit에 닿을 때만 문단 경계에서 강제로 자른다.
 */
export const planWindows = (text: string, scenes: Scene[], options: PlanOptions = {}): Window[] => {
  const windowSize = options.windowSize ?? DEFAULT_WINDOW_SIZE;
  const hardLimit = options.hardLimit ?? DEFAULT_HARD_LIMIT;

  if (scenes.length === 0) {
    return [{ index: 0, start: 0, end: text.length, text, head: '', tail: '', sceneCount: 0, forced: false }];
  }

  const ordered = scenes.toSorted((a, b) => a.start - b.start);
  const cuts: { at: number; sceneCount: number; forced: boolean }[] = [];

  let windowStart = 0;
  let sceneCount = 0;
  let pendingWeak: { at: number; sceneCount: number } | null = null;

  for (const scene of ordered) {
    sceneCount += 1;
    const length = scene.end - windowStart;

    if (scene.boundaryQuality === 'weak' && length >= windowSize) {
      pendingWeak = { at: scene.end, sceneCount };
    }

    if (length >= windowSize && scene.boundaryQuality === 'clean') {
      cuts.push({ at: scene.end, sceneCount, forced: false });
      windowStart = scene.end;
      sceneCount = 0;
      pendingWeak = null;
      continue;
    }

    // 목표의 두 배를 넘도록 clean이 없으면 weak에서라도 닫는다.
    if (length >= windowSize * 2 && pendingWeak) {
      cuts.push({ at: pendingWeak.at, sceneCount: pendingWeak.sceneCount, forced: false });
      windowStart = pendingWeak.at;
      sceneCount = 0;
      pendingWeak = null;
      continue;
    }

    if (length >= hardLimit) {
      const at = forcedBreak(text, windowStart, hardLimit);
      cuts.push({ at, sceneCount, forced: true });
      windowStart = at;
      sceneCount = 0;
      pendingWeak = null;
    }
  }

  const bounds = [0, ...cuts.map((c) => c.at), text.length].filter((v, i, arr) => i === 0 || v > arr[i - 1]);
  const windows: Window[] = [];
  for (let i = 0; i < bounds.length - 1; i++) {
    const start = bounds[i];
    const end = bounds[i + 1];
    windows.push({
      index: i,
      start,
      end,
      text: text.slice(start, end),
      head: start === 0 ? '' : sentenceTail(text.slice(0, start), TAIL_SENTENCES),
      tail: end === text.length ? '' : sentenceHead(text.slice(end), TAIL_SENTENCES),
      sceneCount: cuts[i]?.sceneCount ?? 0,
      forced: cuts[i]?.forced ?? false,
    });
  }

  return windows;
};
