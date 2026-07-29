import type { FieldSpec } from '../../../core/contracts.ts';

// 동결 세대가 쓰는 위젯. 코어는 이 종류들을 모르고, FieldGroup만 해석한다.
export type AnalysisRender =
  | { kind: 'yesNo'; question: string; negative: string; short: string }
  | { kind: 'scale'; question: string; anchors: string[] }
  | { kind: 'reason'; placeholder: string; maxLength: number }
  | { kind: 'note'; label: string; maxLength: number };

// short는 집계 화면의 축 눈금에 쓴다 — 질문 문장은 30px 칸에 들어가지 않는다.
export const yesNo = (key: string, question: string, negative: string, short: string): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (raw === true || raw === false ? raw : null),
  render: { kind: 'yesNo', question, negative, short } satisfies AnalysisRender,
});

export const scale = (key: string, question: string, anchors: string[]): FieldSpec => ({
  key,
  required: true,
  sanitize: (raw) => (Number.isSafeInteger(raw) && (raw as number) >= 1 && (raw as number) <= anchors.length ? raw : null),
  render: { kind: 'scale', question, anchors } satisfies AnalysisRender,
});

// 아니오를 고른 자리에만 열리는 한 줄. 사유를 늘 띄워두면 답해야 하는 문항으로 읽힌다.
export const reason = (key: string, placeholder: string, maxLength = 1000): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => (typeof raw === 'string' && raw.trim() ? raw.trim().slice(0, maxLength) : null),
  render: { kind: 'reason', placeholder, maxLength } satisfies AnalysisRender,
});

export const note = (key: string, label: string, maxLength = 1000): FieldSpec => ({
  key,
  required: false,
  sanitize: (raw) => (typeof raw === 'string' && raw.trim() ? raw.trim().slice(0, maxLength) : null),
  render: { kind: 'note', label, maxLength } satisfies AnalysisRender,
});
