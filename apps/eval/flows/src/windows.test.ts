import { describe, expect, it } from 'vitest';
import { planWindows } from './windows.ts';
import type { Scene } from './analysis-types.ts';

const scene = (start: number, end: number, boundaryQuality: Scene['boundaryQuality']): Scene => ({
  start,
  end,
  boundaryQuality,
  gist: '',
  characters: [],
  setting: '',
  pov: '',
  flashback: '',
});

// 문장 3개 단위로 꼬리를 자르므로 픽스처도 문장으로 만든다.
const sentences = (count: number, from = 0): string =>
  Array.from({ length: count }, (_, i) => `${from + i}번째 문장이 여기에 있다.`).join(' ');

describe('planWindows', () => {
  it('장면이 없으면 전문을 창 하나로 둔다', () => {
    const text = sentences(10);
    const windows = planWindows(text, []);
    expect(windows).toHaveLength(1);
    expect(windows[0].text).toBe(text);
    expect(windows[0].head).toBe('');
    expect(windows[0].tail).toBe('');
  });

  it('목표 크기보다 짧으면 나누지 않는다', () => {
    const text = sentences(20);
    const scenes = [scene(0, 100, 'clean'), scene(100, text.length, 'clean')];
    expect(planWindows(text, scenes, { windowSize: 100_000 })).toHaveLength(1);
  });

  it('clean 경계에서만 창을 닫는다', () => {
    const text = 'ㄱ'.repeat(3000);
    const scenes = [scene(0, 500, 'weak'), scene(500, 1200, 'clean'), scene(1200, 3000, 'clean')];
    const windows = planWindows(text, scenes, { windowSize: 1000, hardLimit: 100_000 });
    expect(windows.map((w) => w.start)).toEqual([0, 1200]);
  });

  it('경계가 none뿐이면 나누지 않는다 — 구조 없는 원고에 선을 긋지 않는다', () => {
    const text = 'ㄱ'.repeat(9000);
    const scenes = [scene(0, 3000, 'none'), scene(3000, 6000, 'none'), scene(6000, 9000, 'none')];
    const windows = planWindows(text, scenes, { windowSize: 1000, hardLimit: 100_000 });
    expect(windows).toHaveLength(1);
    expect(windows[0].forced).toBe(false);
  });

  it('clean이 없으면 목표의 두 배에서 weak로 닫는다', () => {
    const text = 'ㄱ'.repeat(6000);
    const scenes = [scene(0, 1500, 'weak'), scene(1500, 3000, 'none'), scene(3000, 6000, 'none')];
    const windows = planWindows(text, scenes, { windowSize: 1000, hardLimit: 100_000 });
    expect(windows.map((w) => w.start)).toEqual([0, 1500]);
  });

  it('hardLimit에 닿으면 강제로 자르고 표시한다', () => {
    const text = `${'ㄱ'.repeat(2000)}\n${'ㄴ'.repeat(3000)}`;
    const scenes = [scene(0, 2500, 'none'), scene(2500, text.length, 'none')];
    const windows = planWindows(text, scenes, { windowSize: 1000, hardLimit: 2500 });
    expect(windows.length).toBeGreaterThan(1);
    expect(windows[1].forced).toBe(true);
    // 문단 경계(개행 직후)에서 잘려야 한다
    expect(text[windows[1].start - 1]).toBe('\n');
  });

  it('창을 이어 붙이면 원문과 같다', () => {
    const text = 'ㄱ'.repeat(5000);
    const scenes = [scene(0, 1200, 'clean'), scene(1200, 2600, 'clean'), scene(2600, 5000, 'clean')];
    const windows = planWindows(text, scenes, { windowSize: 1000, hardLimit: 100_000 });
    expect(windows.map((w) => w.text).join('')).toBe(text);
  });

  it('꼬리는 원문 그대로이며 창 바깥에서 온다', () => {
    const text = `${sentences(6)} ${sentences(6, 100)}`;
    const cut = text.indexOf('100번째');
    const scenes = [scene(0, cut, 'clean'), scene(cut, text.length, 'clean')];
    const windows = planWindows(text, scenes, { windowSize: 10, hardLimit: 100_000 });

    expect(windows).toHaveLength(2);
    expect(windows[0].head).toBe('');
    expect(windows[1].tail).toBe('');
    // 뒷 꼬리는 다음 창의 앞부분과 일치하고, 앞 꼬리는 이전 창의 끝부분과 일치한다
    expect(windows[1].text.startsWith(windows[0].tail)).toBe(true);
    expect(windows[0].text.endsWith(windows[1].head)).toBe(true);
  });
});
