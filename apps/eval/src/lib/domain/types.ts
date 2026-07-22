export type TaskKind = 'ranking' | 'pair' | 'sanity';
export type RoundStage = 'screening' | 'confirmation';
export type PairVerdict = 'a' | 'b' | 'tie';

export type JudgmentResult =
  | { kind: 'ranking'; ranks: { setId: string; rank: number }[] }
  | { kind: 'pair'; verdict: PairVerdict }
  | { kind: 'scores'; scores: { setId: string; score: number }[] };
