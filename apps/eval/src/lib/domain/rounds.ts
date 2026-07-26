import type { TaskKind } from './types.ts';

export type NewTask = {
  kind: TaskKind;
  documentId: string;
  setIds: string[];
  requiredJudgments: number | null;
  golden: boolean;
};

const shuffle = <T>(items: T[], rng: () => number): T[] => {
  const result = [...items];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    const temp = result[i];
    result[i] = result[j];
    result[j] = temp;
  }
  return result;
};

export const generateScreeningTasks = (
  docs: { documentId: string; setIds: string[] }[],
  opts: { overlapRatio: number; rng: () => number },
): NewTask[] =>
  docs.map((doc) => ({
    kind: 'ranking',
    documentId: doc.documentId,
    setIds: shuffle(doc.setIds, opts.rng),
    requiredJudgments: opts.rng() < opts.overlapRatio ? 2 : 1,
    golden: false,
  }));

// 절대평가 — 세트 한 벌을 그 자체로 판정한다.
// requiredJudgments만큼만 배정해 문서 수를 늘리되, 일부 문서는 상한 없이(null) 열어 동의한
// 평가자 전원이 보게 한다. 두 명만 본 문서로는 "둘이 갈렸다"까지밖에 알 수 없어서, 주관적인 축의
// 불일치가 잡음인지 실제 분열인지 보려면 여러 명이 같은 문서를 봐야 한다.
//
// 중복 문서는 비율만큼 정확히 뽑는다. 문서마다 따로 주사위를 굴리면 실제 개수가 크게 튄다 —
// 30편에 0.1을 넣었더니 3편이 아니라 7편이 걸려 평가자 부담이 배로 늘어난 적이 있다.
export const generateAbsoluteTasks = (
  docs: { documentId: string; setId: string }[],
  opts: { requiredJudgments: number; overlapRatio: number; rng: () => number },
): NewTask[] => {
  const overlapCount = Math.min(docs.length, Math.round(opts.overlapRatio * docs.length));
  const overlapIds = new Set(
    shuffle(docs, opts.rng)
      .slice(0, overlapCount)
      .map((d) => d.documentId),
  );

  return docs.map((doc) => ({
    kind: 'ranking',
    documentId: doc.documentId,
    setIds: [doc.setId],
    requiredJudgments: overlapIds.has(doc.documentId) ? null : opts.requiredJudgments,
    golden: false,
  }));
};

export const generateConfirmationTasks = (
  docs: { documentId: string; v0SetId: string; candidateSetId: string }[],
  opts: { rng: () => number },
): NewTask[] =>
  docs.map((doc) => ({
    kind: 'pair',
    documentId: doc.documentId,
    setIds: opts.rng() < 0.5 ? [doc.v0SetId, doc.candidateSetId] : [doc.candidateSetId, doc.v0SetId],
    requiredJudgments: null,
    golden: true,
  }));
