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
  opts: { overlapRatio: number; sanityRatio: number; rng: () => number },
): NewTask[] => {
  const tasks: NewTask[] = docs.map((doc) => ({
    kind: 'ranking',
    documentId: doc.documentId,
    setIds: shuffle(doc.setIds, opts.rng),
    requiredJudgments: opts.rng() < opts.overlapRatio ? 2 : 1,
    golden: false,
  }));

  const sanityCount = Math.round(docs.length * opts.sanityRatio);
  const sanityDocs = shuffle(docs, opts.rng).slice(0, sanityCount);
  for (const doc of sanityDocs) {
    const setId = doc.setIds[Math.floor(opts.rng() * doc.setIds.length)];
    tasks.push({
      kind: 'sanity',
      documentId: doc.documentId,
      setIds: [setId, setId],
      requiredJudgments: 1,
      golden: false,
    });
  }

  return tasks;
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
