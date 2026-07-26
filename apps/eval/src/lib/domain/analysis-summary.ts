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

// '아니오'로 갈린 판정. 이 라운드에서 가장 값진 정성 데이터다.
//
// 사유가 달린 것만 모으지 않는다 — 사유 없는 '아니오'도 오라클이 헛짚었다는 신고이고,
// 그것만 빼면 화면에 보이는 반대가 실제보다 적어 보인다.
//
// 판정 대상(오라클이 뭐라고 했는지)을 함께 싣는다. 사유만 떼어 놓으면 "과잉 지적으로 보임"이
// 무엇을 두고 한 말인지 알 수 없어 읽을 수가 없다.
export type FeedbackDetail = FeedbackRef & { body: string };

export type Rejection = {
  feedbackId: string;
  // 화면 번호(1-based) — 평가 화면에서 매긴 순번과 같아야 되짚을 수 있다.
  number: number;
  category: string | null;
  body: string;
  refId: string;
  // 원문을 읽어야 판정을 대조할 수 있다 — 어드민 태스크 미리보기로 되짚는 열쇠.
  taskId: string | null;
  evaluator: string;
  // 축별로 '아니오'인지. 세 칸을 늘 같은 자리에 두어야 세로로 훑을 때 무늬가 드러난다.
  failed: { correct: boolean; needed: boolean; useful: boolean };
  note: string | null;
  at: string;
};

// 작품 총평에 남긴 것. 피드백 판정과 같은 이유로 여기도 대상·사람·문서를 함께 실어야 읽힌다.
// 두 축의 '아니오'와 그 사유는 지금까지 집계 숫자로만 남고 화면 어디에도 글로 나오지 않았다.
export type ReviewNote = {
  setId: string;
  refId: string;
  taskId: string | null;
  evaluator: string;
  failed: { readCorrectly: boolean; priorityUseful: boolean };
  // 총평 판정에 달린 사유.
  note: string | null;
  // 판정 전체에 남긴 말(도움 척도 옆의 자유 서술).
  comment: string | null;
  at: string;
};

export const collectReviewNotes = (input: {
  reviewVerdicts: (ReviewVerdictRow & { setId: string })[];
  judgments: { id: string; evaluator: string; comment: string | null; at: Date }[];
  documentBySet: Map<string, { refId: string }>;
  taskBySet: Map<string, string>;
}): ReviewNote[] => {
  const byJudgment = new Map(input.judgments.map((j) => [j.id, j]));
  return (
    input.reviewVerdicts
      .map((v) => {
        const j = byJudgment.get(v.judgmentId);
        const comment = j?.comment && j.comment.trim() ? j.comment : null;
        return {
          setId: v.setId,
          refId: input.documentBySet.get(v.setId)?.refId ?? '?',
          taskId: input.taskBySet.get(v.setId) ?? null,
          evaluator: j?.evaluator ?? '?',
          failed: { readCorrectly: v.readCorrectly === false, priorityUseful: v.priorityUseful === false },
          note: v.note && v.note.trim() ? v.note : null,
          comment,
          at: (j?.at ?? new Date(0)).toISOString(),
        };
      })
      // 둘 다 '예'이고 남긴 말도 없으면 읽을 것이 없다 — 목록에 두면 실제 내용이 묻힌다.
      .filter((r) => r.failed.readCorrectly || r.failed.priorityUseful || r.note || r.comment)
      .toSorted((a, b) => a.refId.localeCompare(b.refId) || a.evaluator.localeCompare(b.evaluator))
  );
};

export const collectRejections = (input: {
  verdicts: (VerdictRow & { at: Date; evaluator: string })[];
  feedbacks: FeedbackDetail[];
  documentBySet: Map<string, { refId: string }>;
  taskBySet: Map<string, string>;
}): Rejection[] => {
  const byId = new Map(input.feedbacks.map((f) => [f.id, f]));
  return input.verdicts
    .filter((v) => v.correct === false || v.needed === false || v.useful === false)
    .map((v) => {
      const f = byId.get(v.feedbackId);
      return {
        feedbackId: v.feedbackId,
        number: (f?.ord ?? 0) + 1,
        category: f?.category ?? null,
        body: f?.body ?? '',
        refId: (f ? input.documentBySet.get(f.setId)?.refId : undefined) ?? '?',
        taskId: (f ? input.taskBySet.get(f.setId) : undefined) ?? null,
        evaluator: v.evaluator,
        failed: { correct: v.correct === false, needed: v.needed === false, useful: v.useful === false },
        note: v.note && v.note.trim() ? v.note : null,
        at: v.at.toISOString(),
      };
    })
    .toSorted((a, b) => a.refId.localeCompare(b.refId) || a.number - b.number);
};
