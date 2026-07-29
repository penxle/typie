import type { FieldSpec } from '../../../core/contracts.ts';

export type ChoiceOption = { value: string; label: string };

// 이 세대가 쓰는 위젯. 코어는 이 종류들을 모르고, FieldGroup만 해석한다.
export type EditorialRender =
  | { kind: 'yesNo'; question: string; negative: string; short: string }
  | { kind: 'triState'; question: string; negative: string; unknownLabel: string; short: string }
  | { kind: 'choice'; question: string; options: ChoiceOption[]; short: string }
  | { kind: 'reasonKind'; options: ChoiceOption[] }
  | { kind: 'scale'; question: string; anchors: string[] }
  | { kind: 'reason'; placeholder: string; maxLength: number; forKey?: string }
  | { kind: 'note'; label: string; maxLength: number };

// short는 집계 화면의 축 눈금에 쓴다 — 질문 문장은 30px 칸에 들어가지 않는다.
export const yesNo = (key: string, question: string, negative: string, short: string): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (raw === true || raw === false ? raw : null),
  render: { kind: 'yesNo', question, negative, short } satisfies EditorialRender,
});

// 이지선다에 '모름'이 없으면 답할 수 없는 판정이 허위 확신으로 적힌다 — 라운드 3에서
// 평가자들이 배경지식 한계를 코멘트로만 신고했다.
export const triState = (key: string, question: string, negative: string, unknownLabel: string, short: string): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (raw === true || raw === false || raw === 'unknown' ? raw : null),
  render: { kind: 'triState', question, negative, unknownLabel, short } satisfies EditorialRender,
});

export const choice = (key: string, question: string, options: ChoiceOption[], short: string): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (options.some((o) => o.value === raw) ? raw : null),
  render: { kind: 'choice', question, options, short } satisfies EditorialRender,
});

// 아니오를 고른 자리에만 열리는 사유 분류. 자유 서술만 받으면 라운드가 끝난 뒤 사람이
// 전수 독해로 유형을 복원해야 한다. 한 지적이 여러 유형에 동시에 해당할 수 있어 복수 선택이다.
// 값은 배열이지만 홑 문자열로 저장된 답도 읽는다 — 읽는 쪽은 전부 이 헬퍼를 거친다.
export const reasonKinds = (raw: unknown): string[] =>
  Array.isArray(raw) ? raw.filter((v): v is string => typeof v === 'string') : typeof raw === 'string' ? [raw] : [];

export const reasonKind = (key: string, options: ChoiceOption[]): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => {
    const picked = [...new Set(reasonKinds(raw))].filter((v) => options.some((o) => o.value === v));
    return picked.length > 0 ? picked : null;
  },
  render: { kind: 'reasonKind', options } satisfies EditorialRender,
});

export const scale = (key: string, question: string, anchors: string[]): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (Number.isSafeInteger(raw) && (raw as number) >= 1 && (raw as number) <= anchors.length ? raw : null),
  render: { kind: 'scale', question, anchors } satisfies EditorialRender,
});

// 아니오를 고른 자리에만 열리는 한 줄. 사유를 늘 띄워두면 답해야 하는 문항으로 읽힌다.
export const reason = (key: string, placeholder: string, maxLength = 1000): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => (typeof raw === 'string' && raw.trim() ? raw.trim().slice(0, maxLength) : null),
  render: { kind: 'reason', placeholder, maxLength } satisfies EditorialRender,
});

// 특정 문항의 아니오에만 열리는 사유. 한 그룹에 아니오 문항이 여럿이면 그룹 공유 사유로는
// 어느 문항의 사유인지 갈라지지 않는다 — 짝을 명시한다.
export const reasonFor = (key: string, forKey: string, placeholder: string, maxLength = 1000): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => (typeof raw === 'string' && raw.trim() ? raw.trim().slice(0, maxLength) : null),
  render: { kind: 'reason', placeholder, maxLength, forKey } satisfies EditorialRender,
});

export const note = (key: string, label: string, maxLength = 1000): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => (typeof raw === 'string' && raw.trim() ? raw.trim().slice(0, maxLength) : null),
  render: { kind: 'note', label, maxLength } satisfies EditorialRender,
});
