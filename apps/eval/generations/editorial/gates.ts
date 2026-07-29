// 산문 프롬프트가 요구하지만 아무것도 강제하지 않던 계약들을 코드로 옮긴 것.
// 여기 모인 판정은 전부 결정적이며 LLM 호출을 쓰지 않는다.

export type GateName =
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
