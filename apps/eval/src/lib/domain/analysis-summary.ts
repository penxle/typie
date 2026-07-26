// 절대평가 라운드의 집계. 스크리닝 지표(후보별 평균 점수·라벨 분포·ranking κ)는 여기서 쓸 수 없다 —
// 후보가 하나뿐이고, 라벨 대신 3항 판정을 받으며, 점수 척도도 질(質)이 아니라 도움 여부로 바뀌었다.
// 이전 라운드 숫자와 나란히 놓으면 오독을 부르므로 별도 블록으로 낸다.

export type AxisTally = { yes: number; no: number };

const emptyTally = (): AxisTally => ({ yes: 0, no: 0 });

export const rate = (t: AxisTally): number => (t.yes + t.no === 0 ? NaN : t.yes / (t.yes + t.no));

export type VerdictRow = {
  judgmentId: string;
  feedbackId: string;
  correct: boolean | null;
  needed: boolean | null;
  useful: boolean | null;
  note: string | null;
};

export type ReviewVerdictRow = {
  judgmentId: string;
  readCorrectly: boolean | null;
  priorityUseful: boolean | null;
  note: string | null;
};

export type FeedbackRef = { id: string; setId: string; ord: number; category: string | null; polarity: string | null };

export type AnalysisSummary = {
  axes: { correct: AxisTally; needed: AxisTally; useful: AxisTally };
  review: { readCorrectly: AxisTally; priorityUseful: AxisTally };
  // 전체 도움 척도 1~5의 분포. 평균만 내면 갈린 분포와 몰린 분포가 구별되지 않는다.
  helpfulness: number[];
  // 판정이 갈린 문서를 앞에 둔다 — 오라클이 어디서 무너지는지가 여기서 먼저 보인다.
  documents: { refId: string; characterCount: number; feedbacks: number; judged: number; no: number }[];
  // 같은 피드백을 두 명 이상이 판정한 경우의 축별 일치율. 중복 구간에서만 나온다.
  agreement: { axis: string; pairs: number; agreed: number }[];
};

const addVerdict = (tally: AxisTally, value: boolean | null): void => {
  if (value === true) tally.yes += 1;
  else if (value === false) tally.no += 1;
};

export const summarizeAnalysis = (input: {
  verdicts: VerdictRow[];
  reviewVerdicts: ReviewVerdictRow[];
  helpfulness: number[];
  feedbacks: FeedbackRef[];
  // setId → 문서 정보. 한 라운드 안에서 세트는 문서와 1:1이다.
  documentBySet: Map<string, { refId: string; characterCount: number }>;
}): AnalysisSummary => {
  const axes = { correct: emptyTally(), needed: emptyTally(), useful: emptyTally() };
  for (const v of input.verdicts) {
    addVerdict(axes.correct, v.correct);
    addVerdict(axes.needed, v.needed);
    addVerdict(axes.useful, v.useful);
  }

  const review = { readCorrectly: emptyTally(), priorityUseful: emptyTally() };
  for (const v of input.reviewVerdicts) {
    addVerdict(review.readCorrectly, v.readCorrectly);
    addVerdict(review.priorityUseful, v.priorityUseful);
  }

  const setOf = new Map(input.feedbacks.map((f) => [f.id, f.setId]));
  const perSet = new Map<string, { feedbacks: number; judged: number; no: number }>();
  for (const f of input.feedbacks) {
    const entry = perSet.get(f.setId) ?? { feedbacks: 0, judged: 0, no: 0 };
    entry.feedbacks += 1;
    perSet.set(f.setId, entry);
  }
  for (const v of input.verdicts) {
    const setId = setOf.get(v.feedbackId);
    if (!setId) continue;
    const entry = perSet.get(setId);
    if (!entry) continue;
    entry.judged += 1;
    if (v.correct === false || v.needed === false || v.useful === false) entry.no += 1;
  }

  const documents = [...perSet]
    .flatMap(([setId, entry]) => {
      const doc = input.documentBySet.get(setId);
      return doc ? [{ refId: doc.refId, characterCount: doc.characterCount, ...entry }] : [];
    })
    .toSorted((a, b) => (b.judged === 0 ? -1 : b.no / b.judged) - (a.judged === 0 ? -1 : a.no / a.judged));

  // 같은 피드백에 답이 둘 이상 모인 것만 본다. 축마다 두 답이 같았는지를 센다.
  const byFeedback = new Map<string, VerdictRow[]>();
  for (const v of input.verdicts) {
    byFeedback.set(v.feedbackId, [...(byFeedback.get(v.feedbackId) ?? []), v]);
  }
  const agreement = (['correct', 'needed', 'useful'] as const).map((axis) => {
    let pairs = 0;
    let agreed = 0;
    for (const rows of byFeedback.values()) {
      const answers = rows.map((r) => r[axis]).filter((a): a is boolean => a !== null);
      for (let i = 0; i < answers.length; i++) {
        for (let j = i + 1; j < answers.length; j++) {
          pairs += 1;
          if (answers[i] === answers[j]) agreed += 1;
        }
      }
    }
    return { axis, pairs, agreed };
  });

  return { axes, review, helpfulness: input.helpfulness, documents, agreement };
};

// '아니오'에 달린 사유. 이 라운드에서 가장 값진 정성 데이터다.
export type RejectionNote = { number: number; category: string | null; axes: string[]; note: string; at: string };

const AXIS_NAME: Record<string, string> = { correct: '정확', needed: '가치', useful: '실행' };

export const collectRejectionNotes = (input: { verdicts: (VerdictRow & { at: Date })[]; feedbacks: FeedbackRef[] }): RejectionNote[] => {
  const byId = new Map(input.feedbacks.map((f) => [f.id, f]));
  return input.verdicts
    .filter((v) => v.note && (v.correct === false || v.needed === false || v.useful === false))
    .map((v) => {
      const f = byId.get(v.feedbackId);
      return {
        number: (f?.ord ?? 0) + 1,
        category: f?.category ?? null,
        axes: [v.correct === false ? 'correct' : null, v.needed === false ? 'needed' : null, v.useful === false ? 'useful' : null]
          .filter((a): a is string => a !== null)
          .map((a) => AXIS_NAME[a] ?? a),
        note: v.note ?? '',
        at: v.at.toISOString(),
      };
    })
    .toSorted((a, b) => a.at.localeCompare(b.at));
};
