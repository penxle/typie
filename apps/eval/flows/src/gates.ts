// 산문 프롬프트가 요구하지만 아무것도 강제하지 않던 계약들을 코드로 옮긴 것.
//
// 이 파이프라인에서 지금까지 실제로 무언가를 보장하던 장치는 앵커 대조(createFindRange) 하나뿐이었다.
// 나머지 계약 — 뒤 문맥 금지, 분석 대상 구간, 묶음 분할, 묶음 보존 — 은 전부 지시문에만 있었고
// 어겨져도 조용히 지나갔다. 여기 모인 판정은 전부 결정적이며 LLM 호출을 쓰지 않는다.

export type Span = { start: number; end: number };

export type GateName =
  | 'anchor-unresolved'
  | 'anchor-out-of-window'
  | 'anchor-non-analytic'
  | 'anchor-ambiguous'
  | 'stumble-unresolved'
  | 'stumble-out-of-window'
  | 'group-duplicate-member'
  | 'group-bad-representative'
  | 'compose-missing-group'
  | 'compose-duplicate-group'
  | 'compose-unknown-group'
  | 'category-length'
  | 'review-index-out-of-range'
  | 'review-empty-item'
  | 'review-number-in-body'
  | 'review-strength-unresolved'
  | 'review-cleared-unknown-axis'
  | 'review-cleared-has-findings'
  | 'review-cleared-discarded-axis';

// payload에 버린 것의 전문을 담는다. 인용 앞자락만 남기면 "몇 건 걸렸다"는 숫자는 얻지만
// 그 기각이 옳았는지는 판정할 수 없어, 판단하려면 실행을 다시 사야 한다.
//
// action이 필요한 이유도 같다. 기각과 계측을 한 배열에 섞어 두면 원장을 되읽을 때 버린 것과
// 세기만 한 것이 구별되지 않는다.
// payload는 문자열로 굳혀 둔다. 워크플로 스텝의 반환 타입이 Serializable로 좁혀져 있어
// 임의 객체가 그대로 통과하지 못한다.
export type GateRecord = { gate: GateName; action: 'reject' | 'note'; detail: string; payload: string };

// 게이트가 과한지 모자란지는 기각 수만으로 판정할 수 없다. 분모 — 각 단계가 무엇을 몇 개
// 내놓고 몇 개가 살아남았는지 — 가 함께 있어야 비율이 나오고, 그 비율이 있어야 임계값을
// 근거로 정할 수 있다. 지금까지 어느 것도 남기지 않아 아무것도 판정할 수 없었다.
export class GateLedger {
  readonly records: GateRecord[] = [];
  readonly counters: Record<string, number> = {};

  reject(gate: GateName, detail: string, payload?: unknown): false {
    this.records.push({ gate, action: 'reject', detail, payload: payload === undefined ? '' : JSON.stringify(payload) });
    return false;
  }

  note(gate: GateName, detail: string, payload?: unknown): void {
    this.records.push({ gate, action: 'note', detail, payload: payload === undefined ? '' : JSON.stringify(payload) });
  }

  count(key: string, by = 1): void {
    this.counters[key] = (this.counters[key] ?? 0) + by;
  }

  counts(): Record<string, number> {
    const out: Record<string, number> = {};
    for (const r of this.records) out[r.gate] = (out[r.gate] ?? 0) + 1;
    return out;
  }
}

export const mergeCounters = (parts: Record<string, number>[]): Record<string, number> => {
  const out: Record<string, number> = {};
  for (const part of parts) {
    for (const [key, value] of Object.entries(part)) out[key] = (out[key] ?? 0) + value;
  }
  return out;
};

// 앵커는 분석 대상 안에서 시작해야 한다. 앞 문맥은 창 시작 이전이라 탐색 자체가 닿지 않지만,
// 뒤 문맥은 창 끝 뒤에 있어 그대로 찾힌다 — 지시문은 양쪽 다 금지하는데 한쪽만 막혀 있었다.
// 인용이 어긋나 한참 뒤 창의 비슷한 구절에 붙는 오매칭도 같은 판정으로 걸린다.
export const startsInWindow = (span: Span, window: Span): boolean => span.start >= window.start && span.start < window.end;

// 구간 안에서 시작하는지만 본다. 걸치는 것으로는 부족한 자리에 쓴다 — 본문에서 시작해 뒤에
// 붙은 후기까지 뻗은 앵커는 본문의 지적이지 후기의 지적이 아니다. 창 경계 판정과 같은 규칙이다.
export const startsInAny = (span: Span, others: Span[]): boolean => others.some((other) => startsInWindow(span, other));

export const countOccurrences = (haystack: string, needle: string): number => {
  const trimmed = needle.trim();
  if (!trimmed) return 0;
  let count = 0;
  let from = 0;
  for (;;) {
    const at = haystack.indexOf(trimmed, from);
    if (at === -1) return count;
    count += 1;
    from = at + 1;
  }
};

// 같은 구절이 창 안에 여러 번 나오면 코드는 첫 일치를 잡는다. 지시문은 "주변 원문을 이어 붙여
// 유일하게 식별되게" 하라고 요구하지만 지켜지는지 아무도 보지 않았다.
//
// 이번에는 세기만 한다. 짧은 인용이 반복되는 비율을 모르는 채로 기각을 걸면 멀쩡한 지적을
// 얼마나 버리는지 알 수 없다 — 비율을 먼저 재고 다음 라운드에서 정한다.
export const isAmbiguousAnchor = (windowText: string, quoteStart: string): boolean => countOccurrences(windowText, quoteStart) >= 2;

// 지적마다 정확히 한 묶음. 고아는 이미 되살리고 있었지만 두 묶음에 든 지적은 아무도 보지 않아
// 같은 지적이 피드백 두 건이 되었다.
export const normalizeGroups = <T extends { members: number[]; representative: number }>(
  groups: T[],
  findingCount: number,
  ledger: GateLedger,
): { members: number[]; representative: number }[] => {
  const taken = new Set<number>();
  const out: { members: number[]; representative: number }[] = [];

  for (const group of groups) {
    const members: number[] = [];
    for (const m of group.members) {
      if (!Number.isSafeInteger(m) || m < 0 || m >= findingCount) {
        ledger.note('group-duplicate-member', `범위 밖 지적 ${m}`);
        continue;
      }
      if (taken.has(m)) {
        ledger.note('group-duplicate-member', `지적 ${m}이 두 묶음에 들어 뒤엣것을 버림`);
        continue;
      }
      taken.add(m);
      members.push(m);
    }
    if (members.length === 0) continue;

    let representative = group.representative;
    if (!members.includes(representative)) {
      ledger.note('group-bad-representative', `대표 ${representative}가 묶음에 없어 ${members[0]}로 바꿈`);
      representative = members[0];
    }
    out.push({ members, representative });
  }

  for (let i = 0; i < findingCount; i++) {
    if (!taken.has(i)) out.push({ members: [i], representative: i });
  }

  return out;
};

// 피드백 쓰기의 계약은 "주어진 것을 모두 옮긴다"이다. 빠뜨리면 지적이 조용히 사라지고,
// 범위 밖 번호를 내면 앵커 없는 빈 피드백이 저장된다 — 둘 다 아무 신호가 없었다.
export const normalizeComposed = <T extends { groupIndex: number }>(feedbacks: T[], groupCount: number, ledger: GateLedger): T[] => {
  const seen = new Set<number>();
  const out: T[] = [];

  for (const feedback of feedbacks) {
    const index = feedback.groupIndex;
    if (!Number.isSafeInteger(index) || index < 0 || index >= groupCount) {
      ledger.reject('compose-unknown-group', `묶음 ${index}는 없는 번호`);
      continue;
    }
    if (seen.has(index)) {
      ledger.reject('compose-duplicate-group', `묶음 ${index}를 두 번 옮김`);
      continue;
    }
    seen.add(index);
    out.push(feedback);
  }

  for (let i = 0; i < groupCount; i++) {
    if (!seen.has(i)) ledger.note('compose-missing-group', `묶음 ${i}가 옮겨지지 않아 유실`);
  }

  return out;
};

export const CATEGORY_MAX = 10;

export const checkCategory = (category: string, ledger: GateLedger): string => {
  const trimmed = category.trim();
  if (trimmed.length === 0 || trimmed.length > CATEGORY_MAX) {
    ledger.note('category-length', `"${trimmed}" (${trimmed.length}자)`);
  }
  return trimmed;
};

// 총평은 피드백이 확정되기 전 번호를 참조할 수 있다. 화면은 이미 범위 밖을 버리고 있었지만
// 파이프라인은 그대로 저장해 왔다 — 걸러낸 자리에서 세어야 총평이 얼마나 헛짚는지 보인다.
const NUMBER_REFERENCE = /\[\d+]|\d+\s*번(?:\s*피드백)?/;

type ReviewItem = { body: string; feedbackIndexes: number[] };

export const normalizeReviewItems = <T extends ReviewItem>(items: T[], feedbackCount: number, ledger: GateLedger): T[] => {
  const out: T[] = [];
  for (const item of items) {
    const indexes = [...new Set(item.feedbackIndexes.filter((i) => Number.isSafeInteger(i)))];
    const kept = indexes.filter((i) => i >= 0 && i < feedbackCount);
    if (kept.length !== indexes.length) {
      ledger.note('review-index-out-of-range', indexes.filter((i) => !kept.includes(i)).join(', '));
    }
    if (!item.body.trim() || kept.length === 0) {
      ledger.reject('review-empty-item', item.body.slice(0, 40));
      continue;
    }
    if (NUMBER_REFERENCE.test(item.body)) {
      ledger.note('review-number-in-body', item.body.slice(0, 60));
    }
    out.push({ ...item, feedbackIndexes: kept });
  }
  return out;
};
