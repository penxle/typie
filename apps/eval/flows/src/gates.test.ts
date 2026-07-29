import { describe, expect, it } from 'vitest';
import {
  checkCategory,
  countOccurrences,
  GateLedger,
  isAmbiguousAnchor,
  mergeCounters,
  normalizeComposed,
  normalizeGroups,
  normalizeReviewItems,
  startsInWindow,
} from './gates.ts';

const WINDOW = { start: 100, end: 200 };

describe('GateLedger', () => {
  // 인용 앞자락만 남기면 "몇 건 걸렸다"는 숫자는 얻지만 그 기각이 옳았는지 판정할 수 없다.
  it('버린 것의 전문을 남긴다', () => {
    const ledger = new GateLedger();
    const finding = { quoteStart: '가', claimType: 'omission', observation: '두 번 읽었다' };
    ledger.reject('stumble-unresolved', '가', finding);
    expect(JSON.parse(ledger.records[0].payload)).toEqual(finding);
  });

  it('기각과 계측을 구별한다', () => {
    const ledger = new GateLedger();
    ledger.reject('anchor-unresolved', 'a');
    ledger.note('anchor-ambiguous', 'b');
    expect(ledger.records.map((r) => r.action)).toEqual(['reject', 'note']);
    expect(ledger.records[1].payload).toBe('');
  });

  // 분모가 없으면 기각 수만으로는 게이트가 과한지 모자란지 판정할 수 없다.
  it('분모를 센다', () => {
    const ledger = new GateLedger();
    ledger.count('finding.emitted', 3);
    ledger.count('finding.emitted');
    expect(ledger.counters['finding.emitted']).toBe(4);
  });
});

describe('mergeCounters', () => {
  it('여러 호출의 분모를 합친다', () => {
    expect(mergeCounters([{ a: 1, b: 2 }, { a: 3 }, {}])).toEqual({ a: 4, b: 2 });
  });
});

describe('startsInWindow', () => {
  it('창 안에서 시작하면 통과한다', () => {
    expect(startsInWindow({ start: 100, end: 150 }, WINDOW)).toBe(true);
    expect(startsInWindow({ start: 199, end: 260 }, WINDOW)).toBe(true);
  });

  // 뒤 문맥은 창 끝 뒤에 있고 탐색이 그대로 닿는다 — 지시문만으로는 막히지 않던 자리다.
  it('창 끝 뒤에서 시작하면 기각한다', () => {
    expect(startsInWindow({ start: 200, end: 240 }, WINDOW)).toBe(false);
    expect(startsInWindow({ start: 900, end: 940 }, WINDOW)).toBe(false);
  });

  it('창 앞에서 시작하면 기각한다', () => {
    expect(startsInWindow({ start: 40, end: 150 }, WINDOW)).toBe(false);
  });
});

describe('countOccurrences / isAmbiguousAnchor', () => {
  it('겹치는 반복까지 센다', () => {
    expect(countOccurrences('aaaa', 'aa')).toBe(3);
    expect(countOccurrences('그는 웃었다. 그는 울었다.', '그는')).toBe(2);
  });

  it('빈 인용은 0이다', () => {
    expect(countOccurrences('본문', '  ')).toBe(0);
  });

  it('창 안에 두 번 이상이면 중의적이다', () => {
    expect(isAmbiguousAnchor('그는 웃었다. 그는 울었다.', '그는')).toBe(true);
    expect(isAmbiguousAnchor('그는 웃었다.', '그는')).toBe(false);
  });
});

describe('normalizeGroups', () => {
  it('두 묶음에 든 지적은 뒤엣것을 버린다', () => {
    const ledger = new GateLedger();
    const groups = normalizeGroups(
      [
        { members: [0, 1], representative: 0 },
        { members: [1, 2], representative: 2 },
      ],
      3,
      ledger,
    );
    expect(groups).toEqual([
      { members: [0, 1], representative: 0 },
      { members: [2], representative: 2 },
    ]);
    expect(ledger.counts()['group-duplicate-member']).toBe(1);
  });

  it('빠진 지적은 혼자인 묶음으로 되살린다', () => {
    const ledger = new GateLedger();
    expect(normalizeGroups([{ members: [0], representative: 0 }], 3, ledger)).toEqual([
      { members: [0], representative: 0 },
      { members: [1], representative: 1 },
      { members: [2], representative: 2 },
    ]);
  });

  it('묶음에 없는 대표는 첫 구성원으로 바꾼다', () => {
    const ledger = new GateLedger();
    const groups = normalizeGroups([{ members: [1, 2], representative: 7 }], 3, ledger);
    expect(groups[0].representative).toBe(1);
    expect(ledger.counts()['group-bad-representative']).toBe(1);
  });

  it('범위 밖 구성원은 버린다', () => {
    const ledger = new GateLedger();
    const groups = normalizeGroups([{ members: [0, 9], representative: 0 }], 2, ledger);
    expect(groups[0].members).toEqual([0]);
  });
});

describe('normalizeComposed', () => {
  it('없는 묶음 번호와 중복을 버린다', () => {
    const ledger = new GateLedger();
    const out = normalizeComposed([{ groupIndex: 0 }, { groupIndex: 0 }, { groupIndex: 5 }], 2, ledger);
    expect(out).toEqual([{ groupIndex: 0 }]);
    expect(ledger.counts()['compose-duplicate-group']).toBe(1);
    expect(ledger.counts()['compose-unknown-group']).toBe(1);
  });

  // 유실은 되살릴 수 없다. 조용히 지나가던 것을 세는 것이 여기서 할 수 있는 전부다.
  it('옮겨지지 않은 묶음을 기록한다', () => {
    const ledger = new GateLedger();
    normalizeComposed([{ groupIndex: 0 }], 3, ledger);
    expect(ledger.counts()['compose-missing-group']).toBe(2);
  });
});

describe('checkCategory', () => {
  it('길이를 벗어나면 기록하되 값은 살린다', () => {
    const ledger = new GateLedger();
    expect(checkCategory('  대화 화자 ', ledger)).toBe('대화 화자');
    expect(ledger.records).toHaveLength(0);
    checkCategory('아주아주아주아주아주 긴 라벨', ledger);
    expect(ledger.counts()['category-length']).toBe(1);
  });
});

describe('normalizeReviewItems', () => {
  it('범위 밖 순번을 걷어낸다', () => {
    const ledger = new GateLedger();
    const out = normalizeReviewItems([{ body: '본문', feedbackIndexes: [0, 9, 2] }], 3, ledger);
    expect(out[0].feedbackIndexes).toEqual([0, 2]);
    expect(ledger.counts()['review-index-out-of-range']).toBe(1);
  });

  it('가리킬 피드백이 없으면 항목을 버린다', () => {
    const ledger = new GateLedger();
    expect(normalizeReviewItems([{ body: '본문', feedbackIndexes: [9] }], 3, ledger)).toEqual([]);
    expect(ledger.counts()['review-empty-item']).toBe(1);
  });

  it('본문에 번호를 쓰면 기록한다', () => {
    const ledger = new GateLedger();
    normalizeReviewItems([{ body: '3번 피드백을 보세요', feedbackIndexes: [0] }], 3, ledger);
    expect(ledger.counts()['review-number-in-body']).toBe(1);
  });
});
