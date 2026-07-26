export type TaskKind = 'ranking' | 'pair';
// corpus = 평가 대상으로 표집한 글, personal = 본인 피드백을 열람하려고 따로 들여온 글.
export type DocumentKind = 'corpus' | 'personal';
export const PERSONAL_CORPUS_VERSION = 'personal';
export type RoundStage = 'screening' | 'confirmation' | 'absolute';
export type PairVerdict = 'a' | 'b' | 'tie';

export type JudgmentResult =
  | { kind: 'ranking'; ranks: { setId: string; rank: number }[] }
  | { kind: 'pair'; verdict: PairVerdict }
  | { kind: 'scores'; scores: { setId: string; score: number }[] };
